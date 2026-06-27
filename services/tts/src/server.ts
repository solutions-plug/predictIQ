/**
 * TTS Service HTTP Server
 *
 * Exposes the TTS service via HTTP with health check endpoints.
 * Supports both Express and Fastify frameworks.
 *
 * Health check endpoints:
 * - GET /health — Comprehensive health check (200 if healthy, 503 if degraded/unhealthy)
 * - GET /health/ready — Readiness probe for Kubernetes (200 if ready, 503 if not)
 * - GET /health/live — Liveness probe for Kubernetes (200 if alive, 503 if dead)
 *
 * TTS endpoints:
 * - POST /tts/enqueue — Enqueue a TTS job
 * - GET /tts/job/:id — Get job status
 * - GET /tts/jobs — List all jobs
 * - POST /tts/generate — Synchronous generation
 */

import express, { Express, Request, Response, NextFunction } from "express";
import { TTSService, TTSConfig, VOICES, AuthError } from "./TTSService";
import {
  HealthChecker,
  createHealthCheckHandler,
  createReadinessHandler,
  createLivenessHandler,
} from "./HealthCheck";
import { W3CTraceContextPropagator } from "@opentelemetry/core";
import { trace, context } from "@opentelemetry/api";

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

const config: TTSConfig = {
  provider: (process.env.TTS_PROVIDER as any) || "elevenlabs",
  elevenlabs: process.env.ELEVENLABS_API_KEY
    ? {
        apiKey: process.env.ELEVENLABS_API_KEY,
        modelId: process.env.ELEVENLABS_MODEL_ID || "eleven_multilingual_v2",
      }
    : undefined,
  google: process.env.GOOGLE_APPLICATION_CREDENTIALS
    ? {
        keyFilename: process.env.GOOGLE_APPLICATION_CREDENTIALS,
      }
    : undefined,
  outputDir: process.env.TTS_OUTPUT_DIR || "/tmp/tts-output",
  auth: process.env.TTS_API_KEY
    ? {
        type: "apikey",
        keys: process.env.TTS_API_KEY.split(","),
      }
    : undefined,
  rateLimit: {
    maxRequests: parseInt(process.env.TTS_RATE_LIMIT_MAX || "100", 10),
    windowMs: parseInt(process.env.TTS_RATE_LIMIT_WINDOW_MS || "60000", 10),
  },
  cache: {
    ttlMs: parseInt(process.env.TTS_CACHE_TTL_MS || "86400000", 10),
    maxEntries: parseInt(process.env.TTS_CACHE_MAX_ENTRIES || "1000", 10),
  },
};

// ---------------------------------------------------------------------------
// Service initialization
// ---------------------------------------------------------------------------

const service = new TTSService(config);
const healthChecker = new HealthChecker(config, service);

// ---------------------------------------------------------------------------
// Express app setup
// ---------------------------------------------------------------------------

const app: Express = express();
const port = process.env.PORT || 3000;

// Middleware
app.use(express.json());

// Request logging
app.use((req, res, next) => {
  console.log(`[${new Date().toISOString()}] ${req.method} ${req.path}`);
  next();
});

// Issue #726: Extract and propagate W3C Trace Context
const propagator = new W3CTraceContextPropagator();
app.use((req: Request, res: Response, next: NextFunction) => {
  const tracer = trace.getTracer("tts-service");
  const ctx = propagator.extract(context.active(), req.headers, {
    get: (carrier, key) => (carrier as any)[key],
    keys: (carrier) => Object.keys(carrier as any),
  });
  
  context.with(ctx, () => {
    const span = tracer.startSpan(`${req.method} ${req.path}`);
    res.on("finish", () => span.end());
    context.with(trace.setSpan(ctx, span), () => {
      next();
    });
  });
});

// Issue #723: Authentication middleware
app.use((req: Request, res: Response, next: NextFunction) => {
  // Skip auth for health checks
  if (req.path.startsWith("/health")) {
    return next();
  }

  if (config.auth) {
    const authHeader = req.headers.authorization;
    if (!authHeader) {
      return res.status(401).json({ error: "Missing Authorization header" });
    }

    const credential = authHeader.replace(/^Bearer\s+/i, "");
    try {
      const { authenticate } = require("./TTSService");
      authenticate(credential, config.auth);
      next();
    } catch (err) {
      if (err instanceof AuthError) {
        return res.status(401).json({ error: err.message });
      }
      return res.status(500).json({ error: "Authentication error" });
    }
  } else {
    next();
  }
});

// ---------------------------------------------------------------------------
// Health check endpoints
// ---------------------------------------------------------------------------

/**
 * GET /health
 * Comprehensive health check with detailed dependency status.
 * Returns 200 if healthy, 503 if degraded or unhealthy.
 */
app.get("/health", createHealthCheckHandler(healthChecker));

/**
 * GET /health/ready
 * Kubernetes readiness probe — indicates if service is ready to accept traffic.
 * Returns 200 if ready, 503 if not.
 */
app.get("/health/ready", createReadinessHandler(healthChecker));

/**
 * GET /health/live
 * Kubernetes liveness probe — indicates if service process is alive.
 * Returns 200 if alive, 503 if dead.
 */
app.get("/health/live", createLivenessHandler(healthChecker));

// ---------------------------------------------------------------------------
// TTS endpoints
// ---------------------------------------------------------------------------

/**
 * POST /tts/enqueue
 * Enqueue a TTS job and return immediately with job ID.
 *
 * Request body:
 * {
 *   "text": "Hello world",
 *   "voiceId": "el-rachel-en",
 *   "provider": "elevenlabs" (optional)
 * }
 *
 * Headers:
 * - Authorization: Bearer <api-key> (required if auth configured)
 * - Cache-Control: no-cache (optional, bypass cache)
 *
 * Response:
 * {
 *   "jobId": "tts_1234567890_abc123",
 *   "status": "pending"
 * }
 */
app.post("/tts/enqueue", (req: Request, res: Response) => {
  try {
    const { text, voiceId, provider } = req.body;
    const rateLimitKey = req.ip || "unknown";
    const bypassCache = req.headers["cache-control"]?.includes("no-cache");

    if (!text || !voiceId) {
      return res.status(400).json({ error: "Missing text or voiceId" });
    }

    const voice = VOICES[voiceId];
    if (!voice) {
      return res.status(400).json({ error: `Unknown voice: ${voiceId}` });
    }

    const jobId = service.enqueue(text, voice, provider, undefined, rateLimitKey, bypassCache);
    res.json({ jobId, status: "pending" });
  } catch (error: any) {
    const statusCode = error.statusCode || 500;
    res.status(statusCode).json({ error: error.message });
  }
});

/**
 * GET /tts/job/:id
 * Get the status and details of a TTS job.
 *
 * Response:
 * {
 *   "id": "tts_1234567890_abc123",
 *   "text": "Hello world",
 *   "status": "done",
 *   "outputPath": "/tmp/tts-output/tts_1234567890_abc123.mp3",
 *   "createdAt": "2024-01-15T10:30:00Z",
 *   "updatedAt": "2024-01-15T10:30:05Z"
 * }
 */
app.get("/tts/job/:id", (req: Request, res: Response) => {
  const job = service.getJob(req.params.id);
  if (!job) {
    return res.status(404).json({ error: "Job not found" });
  }
  res.json(job);
});

/**
 * GET /tts/jobs
 * List all jobs, optionally filtered by status.
 *
 * Query parameters:
 * - status: "pending" | "processing" | "done" | "error"
 *
 * Response:
 * [
 *   { id, text, status, ... },
 *   ...
 * ]
 */
app.get("/tts/jobs", (req: Request, res: Response) => {
  const status = req.query.status as any;
  const jobs = service.listJobs(status);
  res.json(jobs);
});

/**
 * POST /tts/generate
 * Synchronous generation — waits for completion and returns the output path.
 *
 * Request body:
 * {
 *   "text": "Hello world",
 *   "voiceId": "el-rachel-en",
 *   "provider": "elevenlabs" (optional)
 * }
 *
 * Headers:
 * - Authorization: Bearer <api-key> (required if auth configured)
 * - Cache-Control: no-cache (optional, bypass cache)
 *
 * Response:
 * {
 *   "outputPath": "/tmp/tts-output/tts_1234567890_abc123.mp3"
 * }
 */
app.post("/tts/generate", async (req: Request, res: Response) => {
  try {
    const { text, voiceId, provider } = req.body;
    const rateLimitKey = req.ip || "unknown";
    const bypassCache = req.headers["cache-control"]?.includes("no-cache");

    if (!text || !voiceId) {
      return res.status(400).json({ error: "Missing text or voiceId" });
    }

    const voice = VOICES[voiceId];
    if (!voice) {
      return res.status(400).json({ error: `Unknown voice: ${voiceId}` });
    }

    const outputPath = await service.generate(
      text,
      voice,
      provider,
      undefined,
      rateLimitKey,
      bypassCache,
    );
    res.json({ outputPath });
  } catch (error: any) {
    const statusCode = error.statusCode || 500;
    res.status(statusCode).json({ error: error.message });
  }
});

/**
 * GET /tts/voices
 * List available voices.
 *
 * Response:
 * {
 *   "el-rachel-en": { "voiceId": "...", "language": "en-US", "label": "Rachel (EN)" },
 *   ...
 * }
 */
app.get("/tts/voices", (req: Request, res: Response) => {
  res.json(VOICES);
});

// ---------------------------------------------------------------------------
// Error handling
// ---------------------------------------------------------------------------

app.use((err: any, req: Request, res: Response, next: any) => {
  console.error("Unhandled error:", err);
  res.status(500).json({ error: "Internal server error" });
});

// ---------------------------------------------------------------------------
// Server startup
// ---------------------------------------------------------------------------

app.listen(port, () => {
  console.log(`🎙️  TTS Service listening on port ${port}`);
  console.log(`📊 Health check: GET http://localhost:${port}/health`);
  console.log(`🔍 Readiness probe: GET http://localhost:${port}/health/ready`);
  console.log(`💓 Liveness probe: GET http://localhost:${port}/health/live`);
  console.log(`🎵 TTS endpoints: POST http://localhost:${port}/tts/enqueue`);
});

export default app;
