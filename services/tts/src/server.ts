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
    // Bounds total cached audio bytes, not just entry count — MAX_INPUT_LENGTH
    // allows several-MB buffers per entry, so maxEntries alone permits
    // gigabytes of heap growth. Default: 256 MiB.
    maxBytes: parseInt(process.env.TTS_CACHE_MAX_BYTES || String(256 * 1024 * 1024), 10),
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

// Without an explicit trust proxy setting, Express derives req.ip from the
// raw socket address. Deployed behind any reverse proxy/load balancer, every
// request then appears to originate from the proxy's address, collapsing all
// distinct clients into a single rate-limit bucket (req.ip is used both as
// the Express-level rate-limit key below and as the TTSService rate-limit
// key in the route handlers). TRUST_PROXY configures how many hops (or which
// trusted subnets) sit between the client and this process so Express parses
// X-Forwarded-For and populates req.ip with the real client address.
// Defaults to 1 hop — a single load balancer, the common deployment topology
// — override via the env var to match a different topology (e.g. "false" for
// a direct/no-proxy deployment, a higher hop count, or a trusted-subnet
// keyword Express recognizes, such as "loopback").
const trustProxyEnv = process.env.TRUST_PROXY ?? "1";
const trustProxySetting: number | boolean | string =
  trustProxyEnv === "true" ? true
  : trustProxyEnv === "false" ? false
  : /^-?\d+$/.test(trustProxyEnv) ? Number(trustProxyEnv)
  : trustProxyEnv;
app.set("trust proxy", trustProxySetting);

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
});

// ---------------------------------------------------------------------------
// Error handling helpers
// ---------------------------------------------------------------------------

// Issue #1616: Route handlers must never surface raw internal error messages
// (file paths, provider SDK internals, Redis errors, stack-adjacent detail) to
// callers. This mirrors globalErrorHandler's logic: expected 4xx application
// errors keep a safe, specific message; everything else (5xx / unexpected
// throws) collapses to a generic message.
function providerErrorMessage(error: unknown): string {
  if (error instanceof TTSProviderError) {
    return error.message;
  }
  if (error instanceof AuthError) {
    return error.message;
  }
  return "Internal server error";
}

// Maps a thrown error to the HTTP status a route handler should respond with.
// Expected 4xx application errors keep their status; anything else is a 500.
function errorStatus(error: unknown): number {
  if (error instanceof AuthError) {
    return 401;
  }
  if (error instanceof TTSProviderError) {
    return 502;
  }
  return 500;
}

// Single entry point for route-handler failures: derives the status and a
// safe message, then writes the JSON error body. Use this instead of
// responding with error.message directly.
function sendError(res: Response, error: unknown): void {
  res.status(errorStatus(error)).json({ error: providerErrorMessage(error) });
}

// ---------------------------------------------------------------------------
// Routes
// ---------------------------------------------------------------------------

// Health check endpoints
app.get("/health", createHealthCheckHandler(healthChecker));
app.get("/health/ready", createReadinessHandler(healthChecker));
app.get("/health/live", createLivenessHandler(healthChecker));

// POST /tts/enqueue — Enqueue a TTS job
app.post("/tts/enqueue", ttsRateLimiter, async (req: Request, res: Response) => {
  try {
    const { text, voiceId, options } = req.body || {};
    const job = await service.enqueue(text, voiceId, options, req.ip);
    res.status(202).json(job);
  } catch (error) {
    sendError(res, error);
  }
});

// GET /tts/job/:id — Get job status
app.get("/tts/job/:id", async (req: Request, res: Response) => {
  try {
    const job = await service.getJob(req.params.id);
    if (!job) {
      res.status(404).json({ error: "Job not found" });
      return;
    }
    res.json(job);
  } catch (error) {
    sendError(res, error);
  }
});

// GET /tts/job/:id/audio — Download the generated audio for a completed job
app.get("/tts/job/:id/audio", async (req: Request, res: Response) => {
  try {
    const job = await service.getJob(req.params.id);
    if (!job) {
      res.status(404).json({ error: "Job not found" });
      return;
    }
    if (job.status !== "completed" || !job.audioPath) {
      res.status(409).json({ error: "Audio not available for this job" });
      return;
    }
    res.setHeader("Content-Type", "audio/mpeg");
    createReadStream(job.audioPath).pipe(res);
  } catch (error) {
    sendError(res, error);
  }
});

// GET /tts/jobs — List all jobs
app.get("/tts/jobs", async (req: Request, res: Response) => {
  try {
    const jobs = await service.listJobs();
    res.json(jobs);
  } catch (error) {
    sendError(res, error);
  }
});

// POST /tts/generate — Synchronous generation
app.post("/tts/generate", ttsRateLimiter, async (req: Request, res: Response) => {
  try {
    const { text, voiceId, options } = req.body || {};
    const result = await service.generate(text, voiceId, options, req.ip);
    res.json(result);
  } catch (error) {
    sendError(res, error);
  }
});

// ---------------------------------------------------------------------------
// Global error handler
// ---------------------------------------------------------------------------

// 4xx keeps a specific message; 5xx keeps a generic message to avoid leaking
// internals.
app.use((error: unknown, _req: Request, res: Response, _next: NextFunction) => {
  sendError(res, error);
});

export { app, service, healthChecker, providerErrorMessage, errorStatus, sendError };
