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
 * - GET /tts/job/:id/audio — Download the generated audio for a completed job
 * - GET /tts/jobs — List all jobs
 * - POST /tts/generate — Synchronous generation
 */

import { createReadStream } from "fs";
import express, { Express, Request, Response, NextFunction } from "express";
import rateLimit from "express-rate-limit";
import { TTSService, TTSConfig, VOICES, AuthError, TTSProviderError } from "./TTSService";
import {
  HealthChecker,
  createHealthCheckHandler,
  createReadinessHandler,
  createLivenessHandler,
} from "./HealthCheck";
import { W3CTraceContextPropagator } from "@opentelemetry/core";
import { trace, context } from "@opentelemetry/api";
import { rateLimitKeyGenerator } from "./rateLimitKey";
import { initTracing } from "./tracing";
import { createRedisSharedStore } from "./SharedStore";

// ---------------------------------------------------------------------------
// Tracing
// ---------------------------------------------------------------------------

// Issue #1134: initTracing() was defined but never called anywhere, so every
// tracer.startActiveSpan(...) call ran against the default no-op global
// tracer provider. Must run once, before any request handling, so the
// HttpInstrumentation and OTLP exporter are wired up in time.
initTracing();

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
  retry: {
    maxRetries: parseInt(process.env.TTS_MAX_RETRIES || "3", 10),
    maxDelayMs: parseInt(process.env.TTS_MAX_DELAY_MS || "60000", 10),
  },
  // Issue #1133: job store, rate limiting, and cache are process-local Maps
  // by default, which break correctness under horizontal scaling (a GET
  // /tts/job/:id can land on a different pod than the one that processed
  // the job, and rate limits multiply by replica count). Set REDIS_URL when
  // running more than one instance behind a load balancer so state is
  // shared across replicas.
  sharedStore: process.env.REDIS_URL ? createRedisSharedStore(process.env.REDIS_URL) : undefined,
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

// Security headers — applied to every response (JSON error bodies included).
// Mirrors services/api/src/security.rs's security_headers_middleware.
app.use((req: Request, res: Response, next: NextFunction) => {
  res.setHeader("X-Content-Type-Options", "nosniff");
  res.setHeader("X-Frame-Options", "DENY");
  res.setHeader("Referrer-Policy", "no-referrer");
  res.setHeader(
    "Content-Security-Policy",
    "default-src 'none'; frame-ancestors 'none'",
  );
  next();
});

// Middleware
app.use(
  helmet({
    contentSecurityPolicy: {
      useDefaults: false,
      directives: {
        defaultSrc: ["'none'"],
        imgSrc: ["'self'"],
        styleSrc: ["'self'"],
        scriptSrc: ["'self'"],
        connectSrc: ["'self'"],
        objectSrc: ["'none'"],
        frameAncestors: ["'none'"],
      },
    },
    frameguard: { action: "deny" },
    referrerPolicy: { policy: "no-referrer" },
  })
);
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

// Issue #995 / #1132: Express-level rate limiting, keyed on the caller's IP.
// The Authorization header cannot be used as a key here: it hasn't been
// validated yet (auth middleware runs after this, and is optional), so a
// rotating fake credential would otherwise reset the bucket on every request.
const ttsRateLimitPerMinute = parseInt(process.env.TTS_RATE_LIMIT_PER_MINUTE || "60", 10);
const ttsRateLimiter = rateLimit({
  windowMs: 60 * 1000,
  max: ttsRateLimitPerMinute,
  keyGenerator: rateLimitKeyGenerator,
  handler: (_req: Request, res: Response): void => {
    res.setHeader("Retry-After", "60");
    res.status(429).json({ error: "Too Many Requests" });
  },
  standardHeaders: false,
  legacyHeaders: false,
  skip: (req: Request) => req.path.startsWith("/health"),
});
app.use(ttsRateLimiter);

/** Extract the bearer credential from the Authorization header, if present. */
function extractCredential(req: Request): string | undefined {
  const authHeader = req.headers.authorization;
  if (!authHeader) return undefined;
  return authHeader.replace(/^Bearer\s+/i, "");
}

// Issue #723: Authentication middleware
app.use((req: Request, res: Response, next: NextFunction) => {
  // Only /health/live (liveness probe) is exempt from auth — orchestrators
  // need it reachable without credentials. The detailed /health and
  // /health/ready payloads disclose internal config and provider state
  // (API key validity, circuit breaker stats, queue depth) and must be
  // authenticated like any other endpoint.
  if (req.path === "/health/live") {
    return next();
  }

  if (config.auth) {
    const credential = extractCredential(req);
    if (!credential) {
      return res.status(401).json({ error: "Missing Authorization header" });
    }

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
// Helper functions
// ---------------------------------------------------------------------------

/** Return a generic message for provider errors to avoid leaking upstream details. */
export function providerErrorMessage(error: any): string {
  if (error instanceof TTSProviderError) {
    return "TTS provider request failed";
  }
  return error.message;
}

const VALID_PROVIDERS = new Set(["elevenlabs", "google"]);

/** Validate that `provider` is a recognized value; return false and respond 400 if not. */
function validateProvider(provider: any, res: Response): boolean {
  if (provider !== undefined && !VALID_PROVIDERS.has(provider)) {
    res.status(400).json({ error: `Unknown provider: ${provider}` });
    return false;
  }
  return true;
}

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
app.post("/tts/enqueue", async (req: Request, res: Response) => {
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

     if (!validateProvider(provider, res)) return;

     // enqueueAsync so rate limiting is enforced consistently across
    // replicas when REDIS_URL / config.sharedStore is configured (#1133).
    const credential = extractCredential(req);
    const jobId = await service.enqueueAsync(text, voice, provider, credential, rateLimitKey, bypassCache);
    res.json({ jobId, status: "pending" });
  } catch (error: any) {
     const statusCode = error.statusCode || 500;
     res.status(statusCode).json({ error: providerErrorMessage(error) });
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
app.get("/tts/job/:id", async (req: Request, res: Response) => {
  try {
    // getJobAsync falls back to the shared store so polling succeeds
    // regardless of which replica originally processed the job (#1133).
    const id = Array.isArray(req.params.id) ? req.params.id[0] : req.params.id;
    const credential = extractCredential(req);
    const job = await service.getJobAsync(id, credential);
    if (!job) {
      // Same response whether the job doesn't exist or belongs to another
      // tenant, so a credential can't distinguish the two by probing IDs.
      return res.status(404).json({ error: "Job not found" });
    }
    res.json(job);
  } catch (error: any) {
    const statusCode = error.statusCode || 500;
    res.status(statusCode).json({ error: error.message });
  }
});

/**
 * GET /tts/job/:id/audio
 * Download the generated audio for a completed job (issue #1130).
 *
 * The job endpoint above returns `outputPath`, a filesystem path local to
 * this container — there was previously no way for an external caller to
 * actually retrieve that file over HTTP. Enforces the same ownership check
 * as GET /tts/job/:id: a credential can only download audio for jobs it
 * created.
 *
 * Headers:
 * - Authorization: Bearer <api-key> (required if auth configured)
 *
 * Response: audio/mpeg stream (200), or JSON error (404 not found /
 * not owned by caller, 409 job not yet complete).
 */
app.get("/tts/job/:id/audio", async (req: Request, res: Response) => {
  try {
    const id = Array.isArray(req.params.id) ? req.params.id[0] : req.params.id;
    const credential = extractCredential(req);
    const job = await service.getJobAsync(id, credential);
    if (!job) {
      // Same response whether the job doesn't exist or belongs to another
      // tenant, matching GET /tts/job/:id's ownership check.
      return res.status(404).json({ error: "Job not found" });
    }
    if (job.status !== "done" || !job.outputPath) {
      return res.status(409).json({ error: `Job is not complete (status: ${job.status})` });
    }

    res.setHeader("Content-Type", "audio/mpeg");
    res.setHeader("Content-Disposition", `attachment; filename="${id}.mp3"`);

    const stream = createReadStream(job.outputPath);
    stream.on("error", (err) => {
      console.error(`[server] Failed to stream audio for job ${id}: ${err.message}`);
      if (!res.headersSent) {
        res.status(500).json({ error: "Failed to read audio file" });
      } else {
        res.destroy();
      }
    });
    stream.pipe(res);
  } catch (error: any) {
    const statusCode = error.statusCode || 500;
    res.status(statusCode).json({ error: error.message });
  }
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
app.get("/tts/jobs", async (req: Request, res: Response) => {
  try {
    const status = req.query.status as any;
    const credential = extractCredential(req);
    const jobs = await service.listJobsAsync(status, credential);
    res.json(jobs);
  } catch (error: any) {
    const statusCode = error.statusCode || 500;
    res.status(statusCode).json({ error: error.message });
  }
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

     if (!validateProvider(provider, res)) return;

     const credential = extractCredential(req);
    const outputPath = await service.generate(
      text,
      voice,
      provider,
      credential,
      rateLimitKey,
      bypassCache,
    );
    res.json({ outputPath });
  } catch (error: any) {
     const statusCode = error.statusCode || 500;
     res.status(statusCode).json({ error: providerErrorMessage(error) });
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

// Guarded so importing this module (e.g. from tests) doesn't bind a real port.
if (require.main === module) {
  app.listen(port, () => {
    console.log(`🎙️  TTS Service listening on port ${port}`);
    console.log(`📊 Health check: GET http://localhost:${port}/health`);
    console.log(`🔍 Readiness probe: GET http://localhost:${port}/health/ready`);
    console.log(`💓 Liveness probe: GET http://localhost:${port}/health/live`);
    console.log(`🎵 TTS endpoints: POST http://localhost:${port}/tts/enqueue`);
  });
}

export default app;
