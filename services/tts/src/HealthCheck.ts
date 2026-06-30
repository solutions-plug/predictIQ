/**
 * Health Check Module for TTS Service
 *
 * Provides a health check endpoint that verifies:
 * - Service is running and responsive
 * - Dependencies (TTS providers) are configured and accessible
 * - Output directory is writable
 * - Job store is functional
 *
 * Health status levels:
 * - 200 OK: Service is healthy and ready to accept requests
 * - 503 Service Unavailable: Service is degraded or not ready
 */

import { trace, SpanStatusCode } from "@opentelemetry/api";
import { TTSConfig, TTSService } from "./TTSService";
import fs from "fs/promises";
import path from "path";

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/**
 * Maximum number of in-flight + pending jobs allowed before the readiness
 * probe returns a degraded/503 response. Callers should stop sending new
 * work when this threshold is exceeded to prevent unbounded memory growth.
 */
export const MAX_QUEUE_DEPTH = 500;

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export interface HealthCheckResult {
  status: "healthy" | "degraded" | "unhealthy";
  timestamp: string;
  uptime: number; // milliseconds
  checks: {
    service: HealthCheckStatus;
    outputDirectory: HealthCheckStatus;
    elevenlabs?: HealthCheckStatus;
    google?: HealthCheckStatus;
    jobStore: HealthCheckStatus;
    jobQueueDepth?: HealthCheckStatus;
  };
  message: string;
}

export interface HealthCheckStatus {
  status: "ok" | "warning" | "error";
  message: string;
  latency?: number; // milliseconds
}

// ---------------------------------------------------------------------------
// Health Check Implementation
// ---------------------------------------------------------------------------

export class HealthChecker {
  private config: TTSConfig;
  private service: TTSService;
  private startTime: number;

  constructor(config: TTSConfig, service: TTSService) {
    this.config = config;
    this.service = service;
    this.startTime = Date.now();
  }

  /**
   * Run comprehensive health checks and return detailed status.
   * This is the main entry point for health check endpoints.
   */
  async check(): Promise<HealthCheckResult> {
    const tracer = trace.getTracer("tts-health-check");
    return tracer.startActiveSpan("health_check", async (span) => {
      try {
        const checks: HealthCheckResult["checks"] = {
          service: this.checkService(),
          outputDirectory: await this.checkOutputDirectory(),
          jobStore: this.checkJobStore(),
          jobQueueDepth: this.checkJobQueueDepth(),
        };

        // Check configured providers
        if (this.config.elevenlabs) {
          checks.elevenlabs = await this.checkElevenLabs();
        }

        if (this.config.google) {
          checks.google = await this.checkGoogle();
        }

        // Determine overall status
        const allStatuses = Object.values(checks).map((c) => c.status);
        const hasError = allStatuses.includes("error");
        const hasWarning = allStatuses.includes("warning");

        const status = hasError
          ? "unhealthy"
          : hasWarning
            ? "degraded"
            : "healthy";
        const message = this.buildMessage(status, checks);

        const result: HealthCheckResult = {
          status,
          timestamp: new Date().toISOString(),
          uptime: Date.now() - this.startTime,
          checks,
          message,
        };

        span.setAttribute("health.status", status);
        span.setStatus({ code: SpanStatusCode.OK });

        return result;
      } catch (error) {
        span.setStatus({ code: SpanStatusCode.ERROR, message: String(error) });
        throw error;
      } finally {
        span.end();
      }
    });
  }

  /**
   * Readiness check — used by Kubernetes readiness probes and the Docker
   * HEALTHCHECK on /health/ready.
   *
   * Performs real dependency probes so that the service is only marked
   * "ready" when it can actually process TTS requests:
   *   1. TTS provider reachability (lightweight API / credential validation)
   *   2. Output directory writability
   *   3. Job queue depth (guards against unbounded memory growth)
   *   4. Circuit breaker states (open breakers → service not ready)
   *
   * Returns 503 with a structured JSON body if any check fails.
   */
  async readiness(): Promise<{
    status: "ok" | "error";
    message: string;
    checks: Record<string, HealthCheckStatus>;
    circuitBreakers?: Record<string, { state: string; failures: number; successes: number }>;
  }> {
    const checks: Record<string, HealthCheckStatus> = {};

    // 1. Output directory writability
    checks.outputDirectory = await this.checkOutputDirectory();

    // 2. TTS provider reachability
    if (this.config.elevenlabs) {
      checks.elevenlabs = await this.checkElevenLabs();
    }
    if (this.config.google) {
      checks.google = await this.checkGoogle();
    }

    // 3. Job queue depth
    checks.jobQueueDepth = this.checkJobQueueDepth();

    // 4. Circuit breaker states — open breakers make the service effectively
    //    unable to process new jobs, so mark as error to fail the readiness probe.
    const cbStates = this.service.getCircuitBreakerStates();
    for (const [provider, state] of Object.entries(cbStates)) {
      if (state.state === "open") {
        checks[`circuitBreaker_${provider}`] = {
          status: "error",
          message: `${provider} circuit breaker is OPEN (${state.failures} failures) — fast-failing`,
        };
      } else if (state.state === "halfOpen") {
        checks[`circuitBreaker_${provider}`] = {
          status: "warning",
          message: `${provider} circuit breaker is HALF-OPEN — probing for recovery`,
        };
      }
    }

    const hasError = Object.values(checks).some((c) => c.status === "error");

    if (hasError) {
      const failingChecks = Object.entries(checks)
        .filter(([, c]) => c.status === "error")
        .map(([k, c]) => `${k}: ${c.message}`)
        .join("; ");
      return {
        status: "error",
        message: `Service not ready — failing checks: ${failingChecks}`,
        checks,
        circuitBreakers: cbStates,
      };
    }

    return {
      status: "ok",
      message: "Service is ready to accept requests",
      checks,
      circuitBreakers: cbStates,
    };
  }

  /**
   * Lightweight liveness check — returns quickly to verify process is alive.
   * Suitable for Kubernetes liveness probes.
   */
  liveness(): HealthCheckStatus {
    return {
      status: "ok",
      message: "Service process is alive",
    };
  }

  // ---------------------------------------------------------------------------
  // Private checks
  // ---------------------------------------------------------------------------

  private checkService(): HealthCheckStatus {
    try {
      // Verify service instance is functional
      if (!this.service) {
        return { status: "error", message: "Service instance not initialized" };
      }

      return {
        status: "ok",
        message: "TTS service is running",
      };
    } catch (error) {
      return {
        status: "error",
        message: `Service check failed: ${String(error)}`,
      };
    }
  }

  private async checkOutputDirectory(): Promise<HealthCheckStatus> {
    const start = Date.now();
    try {
      const dir = this.config.outputDir;

      // Ensure directory exists
      await fs.mkdir(dir, { recursive: true });

      // Verify writability by creating and immediately removing a probe file
      const testFile = path.join(dir, `.health-check-${Date.now()}`);
      await fs.writeFile(testFile, "health-check");
      await fs.unlink(testFile);

      return {
        status: "ok",
        message: `Output directory is writable: ${dir}`,
        latency: Date.now() - start,
      };
    } catch (error) {
      return {
        status: "error",
        message: `Output directory check failed: ${String(error)}`,
        latency: Date.now() - start,
      };
    }
  }

  private checkJobStore(): HealthCheckStatus {
    try {
      // Verify job store is functional
      const jobs = this.service.listJobs();

      return {
        status: "ok",
        message: `Job store is functional (${jobs.length} jobs in memory)`,
      };
    } catch (error) {
      return {
        status: "error",
        message: `Job store check failed: ${String(error)}`,
      };
    }
  }

  /**
   * Check that the active job queue has not exceeded MAX_QUEUE_DEPTH.
   * Counts jobs in "pending" and "processing" states as in-flight work.
   */
  private checkJobQueueDepth(): HealthCheckStatus {
    try {
      const pending = this.service.listJobs("pending").length;
      const processing = this.service.listJobs("processing").length;
      const depth = pending + processing;

      if (depth >= MAX_QUEUE_DEPTH) {
        return {
          status: "error",
          message: `Job queue depth exceeded limit: ${depth}/${MAX_QUEUE_DEPTH} (pending=${pending}, processing=${processing})`,
        };
      }

      // Warn at 80 % capacity
      if (depth >= MAX_QUEUE_DEPTH * 0.8) {
        return {
          status: "warning",
          message: `Job queue depth near limit: ${depth}/${MAX_QUEUE_DEPTH} (pending=${pending}, processing=${processing})`,
        };
      }

      return {
        status: "ok",
        message: `Job queue depth is healthy: ${depth}/${MAX_QUEUE_DEPTH}`,
      };
    } catch (error) {
      return {
        status: "error",
        message: `Job queue depth check failed: ${String(error)}`,
      };
    }
  }

  /**
   * ElevenLabs probe — makes a lightweight API call (GET /v1/user) to verify
   * the API key is valid and the endpoint is reachable.  Falls back to a
   * credential format check if the network call fails so that misconfigured
   * keys are still surfaced as errors.
   */
  private async checkElevenLabs(): Promise<HealthCheckStatus> {
    const start = Date.now();
    try {
      if (!this.config.elevenlabs) {
        return { status: "warning", message: "ElevenLabs not configured" };
      }

      const { apiKey } = this.config.elevenlabs;

      if (!apiKey) {
        return { status: "error", message: "ElevenLabs API key not set" };
      }

      if (apiKey.length < 10) {
        return {
          status: "error",
          message: "ElevenLabs API key appears invalid (too short)",
        };
      }

      // Lightweight probe: GET /v1/user — returns 200 for valid keys, 401
      // for invalid keys, and will throw on network errors.
      let res: Response;
      try {
        const controller = new AbortController();
        const timeout = setTimeout(() => controller.abort(), 5000);
        try {
          res = await fetch("https://api.elevenlabs.io/v1/user", {
            method: "GET",
            headers: { "xi-api-key": apiKey },
            signal: controller.signal,
          });
        } finally {
          clearTimeout(timeout);
        }
      } catch (networkErr) {
        // Network unreachable — surface as error so readiness probe fails
        return {
          status: "error",
          message: `ElevenLabs unreachable: ${String(networkErr)}`,
          latency: Date.now() - start,
        };
      }

      if (res.status === 401) {
        return {
          status: "error",
          message: "ElevenLabs API key rejected (HTTP 401)",
          latency: Date.now() - start,
        };
      }

      if (!res.ok) {
        return {
          status: "warning",
          message: `ElevenLabs probe returned HTTP ${res.status}`,
          latency: Date.now() - start,
        };
      }

      return {
        status: "ok",
        message: "ElevenLabs is reachable and API key is valid",
        latency: Date.now() - start,
      };
    } catch (error) {
      return {
        status: "error",
        message: `ElevenLabs check failed: ${String(error)}`,
        latency: Date.now() - start,
      };
    }
  }

  /**
   * Google TTS probe — validates credentials format and, when a key file is
   * provided, verifies it exists and contains valid JSON with the required
   * fields.  A real synthesizeSpeech call is intentionally avoided here to
   * keep probe latency low; the circuit breaker (TTSService) handles runtime
   * failure detection.
   */
  private async checkGoogle(): Promise<HealthCheckStatus> {
    const start = Date.now();
    try {
      if (!this.config.google) {
        return { status: "warning", message: "Google TTS not configured" };
      }

      const { keyFilename, credentials } = this.config.google;

      if (!keyFilename && !credentials) {
        return {
          status: "error",
          message: "Google TTS credentials not configured",
        };
      }

      // If using keyFilename, verify file exists and is readable valid JSON
      if (keyFilename) {
        try {
          await fs.access(keyFilename);
          const content = await fs.readFile(keyFilename, "utf-8");
          const parsed = JSON.parse(content) as Record<string, unknown>;
          if (!parsed["project_id"] || !parsed["private_key"]) {
            return {
              status: "error",
              message: "Google TTS key file is missing required fields (project_id, private_key)",
              latency: Date.now() - start,
            };
          }
        } catch (err) {
          return {
            status: "error",
            message: `Google TTS key file invalid: ${String(err)}`,
            latency: Date.now() - start,
          };
        }
      }

      // Validate inline credentials structure when provided
      if (credentials && typeof credentials === "object") {
        const creds = credentials as Record<string, unknown>;
        if (!creds["project_id"] || !creds["private_key"]) {
          return {
            status: "error",
            message: "Google TTS credentials missing required fields (project_id, private_key)",
            latency: Date.now() - start,
          };
        }
      }

      return {
        status: "ok",
        message: "Google TTS credentials are valid and accessible",
        latency: Date.now() - start,
      };
    } catch (error) {
      return {
        status: "error",
        message: `Google TTS check failed: ${String(error)}`,
        latency: Date.now() - start,
      };
    }
  }

  private buildMessage(status: string, checks: Record<string, HealthCheckStatus>): string {
    const parts: string[] = [];

    if (status === "healthy") {
      parts.push("✅ All systems operational");
    } else if (status === "degraded") {
      parts.push("⚠️ Service is degraded");
      Object.entries(checks).forEach(([key, check]) => {
        if (check.status === "warning") {
          parts.push(`  - ${key}: ${check.message}`);
        }
      });
    } else {
      parts.push("❌ Service is unhealthy");
      Object.entries(checks).forEach(([key, check]) => {
        if (check.status === "error") {
          parts.push(`  - ${key}: ${check.message}`);
        }
      });
    }

    return parts.join("\n");
  }
}

// ---------------------------------------------------------------------------
// HTTP Endpoint Helpers
// ---------------------------------------------------------------------------

/**
 * Express/Fastify middleware for health check endpoints.
 *
 * Usage:
 *   app.get('/health',       createHealthCheckHandler(healthChecker));
 *   app.get('/health/ready', createReadinessHandler(healthChecker));
 *   app.get('/health/live',  createLivenessHandler(healthChecker));
 */

export function createHealthCheckHandler(healthChecker: HealthChecker) {
  return async (req: any, res: any) => {
    try {
      const result = await healthChecker.check();
      const statusCode = result.status === "healthy" ? 200 : 503;
      res.status(statusCode).json(result);
    } catch (error) {
      res.status(503).json({
        status: "unhealthy",
        timestamp: new Date().toISOString(),
        message: `Health check failed: ${String(error)}`,
      });
    }
  };
}

export function createReadinessHandler(healthChecker: HealthChecker) {
  return async (req: any, res: any) => {
    try {
      const result = await healthChecker.readiness();
      const statusCode = result.status === "ok" ? 200 : 503;
      res.status(statusCode).json({
        status: result.status === "ok" ? "ready" : "not_ready",
        message: result.message,
        checks: result.checks,
        circuitBreakers: result.circuitBreakers,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      res.status(503).json({
        status: "not_ready",
        message: `Readiness check failed: ${String(error)}`,
        timestamp: new Date().toISOString(),
      });
    }
  };
}

export function createLivenessHandler(healthChecker: HealthChecker) {
  return async (req: any, res: any) => {
    try {
      const result = healthChecker.liveness();
      res.status(200).json({
        status: "alive",
        message: result.message,
        timestamp: new Date().toISOString(),
      });
    } catch (error) {
      res.status(503).json({
        status: "dead",
        message: `Liveness check failed: ${String(error)}`,
        timestamp: new Date().toISOString(),
      });
    }
  };
}
