/**
 * TTSService — AI text-to-speech for social video narrations.
 *
 * Supports ElevenLabs (primary) and Google Cloud TTS (fallback).
 * Audio jobs are processed asynchronously; output files are stored
 * locally (or an S3-compatible bucket via the configured storage adapter).
 *
 * Features:
 *  - Per-IP and per-user rate limiting (issue #531)
 *  - Audio caching by content hash (issue #532)
 *  - Provider error handling with fallback (issue #533)
 *  - Input sanitization and SSML injection prevention (issue #534)
 *  - Circuit breaker per TTS provider (opossum) to fast-fail on
 *    sustained upstream failures and protect connection pool resources
 */

import fs from "fs/promises";
import path from "path";
import { createHash } from "crypto";
import { trace, SpanStatusCode, Span } from "@opentelemetry/api";
import CircuitBreaker from "opossum";

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

export type TTSProvider = "elevenlabs" | "google";

export interface TTSVoice {
  /** Provider-specific voice ID */
  voiceId: string;
  /** BCP-47 language tag, e.g. "en-US", "es-ES" */
  language: string;
  /** Human-readable label */
  label: string;
}

export interface TTSJob {
  id: string;
  text: string;
  voice: TTSVoice;
  provider: TTSProvider;
  status: "pending" | "processing" | "done" | "error";
  outputPath?: string;
  error?: string;
  createdAt: Date;
  updatedAt: Date;
  bypassCache?: boolean;
}

// ---------------------------------------------------------------------------
// Rate limiting (issue #531)
// ---------------------------------------------------------------------------

export interface RateLimitConfig {
  /** Max requests per window per key (IP or user) */
  maxRequests: number;
  /** Window duration in milliseconds */
  windowMs: number;
}

export interface RateLimitEntry {
  count: number;
  windowStart: number;
}

/** Thrown when a rate limit is exceeded; maps to HTTP 429. */
export class RateLimitError extends Error {
  readonly statusCode = 429;
  constructor(message = "Too Many Requests") {
    super(message);
    this.name = "RateLimitError";
  }
}

export interface RateLimitMetrics {
  totalChecks: number;
  totalExceeded: number;
  /** Map of key → current count in window */
  currentCounts: Record<string, number>;
}

export class RateLimiter {
  private store = new Map<string, RateLimitEntry>();
  private metrics: RateLimitMetrics = { totalChecks: 0, totalExceeded: 0, currentCounts: {} };

  constructor(private config: RateLimitConfig) {}

  /**
   * Check and increment the counter for `key`.
   * Throws `RateLimitError` if the limit is exceeded.
   */
  check(key: string): void {
    const now = Date.now();
    this.metrics.totalChecks++;

    let entry = this.store.get(key);
    if (!entry || now - entry.windowStart >= this.config.windowMs) {
      entry = { count: 0, windowStart: now };
      this.store.set(key, entry);
    }

    entry.count++;
    this.metrics.currentCounts[key] = entry.count;

    if (entry.count > this.config.maxRequests) {
      this.metrics.totalExceeded++;
      throw new RateLimitError(
        `Rate limit exceeded for key "${key}": ${entry.count}/${this.config.maxRequests} in ${this.config.windowMs}ms`
      );
    }
  }

  getMetrics(): Readonly<RateLimitMetrics> {
    return { ...this.metrics, currentCounts: { ...this.metrics.currentCounts } };
  }

  /** Evict expired windows to keep memory bounded */
  evictExpired(): void {
    const now = Date.now();
    for (const [key, entry] of this.store) {
      if (now - entry.windowStart >= this.config.windowMs) {
        this.store.delete(key);
        delete this.metrics.currentCounts[key];
      }
    }
  }
}

// ---------------------------------------------------------------------------
// Audio cache (issue #532)
// ---------------------------------------------------------------------------

export interface CacheConfig {
  /** TTL in milliseconds */
  ttlMs: number;
  /** Max number of entries; oldest evicted when exceeded */
  maxEntries: number;
}

interface CacheEntry {
  buffer: Buffer;
  createdAt: number;
  hits: number;
}

export interface CacheMetrics {
  hits: number;
  misses: number;
  evictions: number;
  size: number;
}

export class AudioCache {
  private store = new Map<string, CacheEntry>();
  private metrics: CacheMetrics = { hits: 0, misses: 0, evictions: 0, size: 0 };

  constructor(private config: CacheConfig) {}

  /** Compute a deterministic cache key from text + voiceId + provider */
  static key(text: string, voiceId: string, provider: TTSProvider): string {
    return createHash("sha256").update(`${provider}:${voiceId}:${text}`).digest("hex");
  }

  get(key: string): Buffer | undefined {
    const entry = this.store.get(key);
    if (!entry) { this.metrics.misses++; return undefined; }
    if (Date.now() - entry.createdAt > this.config.ttlMs) {
      this.store.delete(key);
      this.metrics.size--;
      this.metrics.misses++;
      return undefined;
    }
    entry.hits++;
    this.metrics.hits++;
    return entry.buffer;
  }

  set(key: string, buffer: Buffer): void {
    if (this.store.size >= this.config.maxEntries) {
      // Evict the oldest entry
      const oldest = this.store.keys().next().value;
      if (oldest !== undefined) {
        this.store.delete(oldest);
        this.metrics.evictions++;
        this.metrics.size--;
      }
    }
    this.store.set(key, { buffer, createdAt: Date.now(), hits: 0 });
    this.metrics.size++;
  }

  getMetrics(): Readonly<CacheMetrics> {
    return { ...this.metrics };
  }
}

// ---------------------------------------------------------------------------
// Input sanitization (issue #534)
// ---------------------------------------------------------------------------

export const MAX_INPUT_LENGTH = 5000;

/** Thrown when input validation fails; maps to HTTP 400. */
export class InputValidationError extends Error {
  readonly statusCode = 400;
  constructor(message: string) {
    super(message);
    this.name = "InputValidationError";
  }
}

/**
 * Sanitize TTS input text:
 *  1. Enforce max length
 *  2. Strip SSML/XML tags to prevent injection
 *  3. Normalize whitespace
 */
export function sanitizeInput(text: string): string {
  if (typeof text !== "string" || text.trim().length === 0) {
    throw new InputValidationError("Input text must be a non-empty string");
  }
  if (text.length > MAX_INPUT_LENGTH) {
    throw new InputValidationError(
      `Input text exceeds maximum length of ${MAX_INPUT_LENGTH} characters`
    );
  }
  // Strip SSML/XML tags (prevent injection into providers that accept SSML)
  const stripped = text.replace(/<[^>]*>/g, "");
  // Normalize whitespace
  return stripped.replace(/\s+/g, " ").trim();
}

// ---------------------------------------------------------------------------
// Provider error handling (issue #533)
// ---------------------------------------------------------------------------

/** Structured TTS provider error with context */
export class TTSProviderError extends Error {
  readonly statusCode: number;
  constructor(
    public readonly provider: TTSProvider,
    message: string,
    statusCode = 502
  ) {
    super(`[${provider}] ${message}`);
    this.name = "TTSProviderError";
    this.statusCode = statusCode;
  }
}

// ---------------------------------------------------------------------------
// Circuit breaker
// ---------------------------------------------------------------------------

/**
 * Snapshot of a circuit breaker's current state, included in health checks.
 */
export interface CircuitBreakerState {
  /** Current state: "closed" (normal), "open" (fast-failing), "half-open" (probing) */
  state: "closed" | "open" | "halfOpen";
  /** Whether the breaker is currently allowing calls through */
  enabled: boolean;
  /** How many failures have been recorded in the current window */
  failures: number;
  /** How many calls have succeeded since the breaker was last reset */
  successes: number;
  /** Percentage of calls that have failed in the current window */
  percentile: number;
}

/**
 * Circuit breaker configuration for a TTS provider.
 * Shared across both ElevenLabs and Google TTS breakers.
 */
export interface CircuitBreakerConfig {
  /**
   * Number of failures required to open the circuit.
   * Default: 5
   */
  openThreshold?: number;
  /**
   * Rolling window in milliseconds over which failures are counted.
   * Default: 30_000 (30 s)
   */
  rollingWindowMs?: number;
  /**
   * Delay in milliseconds before the circuit attempts a half-open probe.
   * Default: 30_000 (30 s)
   */
  halfOpenIntervalMs?: number;
  /**
   * Timeout in milliseconds per provider call.  Calls that exceed this are
   * counted as failures.
   * Default: 10_000 (10 s)
   */
  timeoutMs?: number;
}

// Default circuit breaker settings (acceptance criteria: 5 failures in 30 s,
// half-open probe every 30 s).
const DEFAULT_CB_CONFIG: Required<CircuitBreakerConfig> = {
  openThreshold: 5,
  rollingWindowMs: 30_000,
  halfOpenIntervalMs: 30_000,
  timeoutMs: 10_000,
};

export interface TTSConfig {
  provider: TTSProvider;
  elevenlabs?: {
    apiKey: string;
    modelId?: string;
  };
  google?: {
    keyFilename?: string;
    credentials?: object;
  };
  outputDir: string;
  auth?: AuthConfig;
  /** Rate limiting — omit to disable */
  rateLimit?: RateLimitConfig;
  /** Audio caching — omit to disable */
  cache?: CacheConfig;
  /** Circuit breaker settings — omit to use defaults */
  circuitBreaker?: CircuitBreakerConfig;
}

// ---------------------------------------------------------------------------
// Auth
// ---------------------------------------------------------------------------

export interface ApiKeyAuthConfig {
  type: "apikey";
  keys: string[];
}

export interface JwtAuthConfig {
  type: "jwt";
  secret: string;
}

export type AuthConfig = ApiKeyAuthConfig | JwtAuthConfig;

export class AuthError extends Error {
  readonly statusCode = 401;
  constructor(message = "Unauthorized") {
    super(message);
    this.name = "AuthError";
  }
}

export function authenticate(credential: string | undefined, auth: AuthConfig): void {
  if (!credential) throw new AuthError("Missing credential");

  if (auth.type === "apikey") {
    if (!auth.keys.includes(credential)) throw new AuthError("Invalid API key");
    return;
  }

  const parts = credential.split(".");
  if (parts.length !== 3) throw new AuthError("Malformed JWT");

  const [headerB64, payloadB64, sigB64] = parts;
  const { createHmac } = require("crypto") as typeof import("crypto");
  const expected = createHmac("sha256", auth.secret)
    .update(`${headerB64}.${payloadB64}`)
    .digest("base64url");

  if (expected !== sigB64) throw new AuthError("Invalid JWT signature");

  const payload = JSON.parse(Buffer.from(payloadB64, "base64url").toString());
  if (payload.exp !== undefined && payload.exp < Math.floor(Date.now() / 1000)) {
    throw new AuthError("JWT expired");
  }
}

// ---------------------------------------------------------------------------
// Built-in voice catalogue
// ---------------------------------------------------------------------------

export const VOICES: Record<string, TTSVoice> = {
  "el-rachel-en": { voiceId: "21m00Tcm4TlvDq8ikWAM", language: "en-US", label: "Rachel (EN)" },
  "el-adam-en":   { voiceId: "pNInz6obpgDQGcFmaJgB", language: "en-US", label: "Adam (EN)"   },
  "el-bella-en":  { voiceId: "EXAVITQu4vr4xnSDxMaL", language: "en-US", label: "Bella (EN)"  },
  "gcp-en-us-f":  { voiceId: "en-US-Neural2-F",      language: "en-US", label: "Google EN-F" },
  "gcp-es-es-f":  { voiceId: "es-ES-Neural2-A",      language: "es-ES", label: "Google ES-F" },
  "gcp-fr-fr-f":  { voiceId: "fr-FR-Neural2-A",      language: "fr-FR", label: "Google FR-F" },
  "gcp-de-de-f":  { voiceId: "de-DE-Neural2-F",      language: "de-DE", label: "Google DE-F" },
};

// ---------------------------------------------------------------------------
// In-memory job store
// ---------------------------------------------------------------------------

const jobStore = new Map<string, TTSJob>();

function makeId(): string {
  return `tts_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

// ---------------------------------------------------------------------------
// Provider implementations
// ---------------------------------------------------------------------------

async function generateElevenLabs(
  text: string,
  voice: TTSVoice,
  config: NonNullable<TTSConfig["elevenlabs"]>
): Promise<Buffer> {
  const tracer = trace.getTracer("tts-service");
  return tracer.startActiveSpan("elevenlabs.generate", async (span: Span) => {
    try {
      span.setAttribute("tts.provider", "elevenlabs");
      span.setAttribute("tts.voice.id", voice.voiceId);
      span.setAttribute("tts.text.length", text.length);

      const modelId = config.modelId ?? "eleven_multilingual_v2";
      const url = `https://api.elevenlabs.io/v1/text-to-speech/${voice.voiceId}`;

      let res: Response;
      try {
        res = await fetch(url, {
          method: "POST",
          headers: {
            "xi-api-key": config.apiKey,
            "Content-Type": "application/json",
            Accept: "audio/mpeg",
          },
          body: JSON.stringify({
            text,
            model_id: modelId,
            voice_settings: { stability: 0.5, similarity_boost: 0.75 },
          }),
        });
      } catch (networkErr) {
        const msg = `Network error calling ElevenLabs: ${String(networkErr)}`;
        console.error(`[TTSService] ${msg}`);
        span.setStatus({ code: SpanStatusCode.ERROR, message: msg });
        throw new TTSProviderError("elevenlabs", msg);
      }

      if (!res.ok) {
        const msg = await res.text().catch(() => res.statusText);
        const detail = `ElevenLabs HTTP ${res.status}: ${msg}`;
        console.error(`[TTSService] ${detail}`);
        span.setStatus({ code: SpanStatusCode.ERROR, message: detail });
        throw new TTSProviderError("elevenlabs", detail, res.status >= 500 ? 502 : res.status);
      }

      const buffer = Buffer.from(await res.arrayBuffer());
      span.setAttribute("tts.audio.size", buffer.length);
      span.setStatus({ code: SpanStatusCode.OK });
      return buffer;
    } finally {
      span.end();
    }
  });
}

async function generateGoogle(
  text: string,
  voice: TTSVoice,
  config: NonNullable<TTSConfig["google"]>
): Promise<Buffer> {
  const tracer = trace.getTracer("tts-service");
  return tracer.startActiveSpan("google.generate", async (span: Span) => {
    try {
      span.setAttribute("tts.provider", "google");
      span.setAttribute("tts.voice.id", voice.voiceId);
      span.setAttribute("tts.text.length", text.length);

      // eslint-disable-next-line @typescript-eslint/no-var-requires
      const { TextToSpeechClient } = require("@google-cloud/text-to-speech") as {
        TextToSpeechClient: new (opts: object) => {
          synthesizeSpeech: (req: object) => Promise<[{ audioContent: Buffer | string }]>;
        };
      };

      const client = new TextToSpeechClient(config);

      let response: { audioContent: Buffer | string };
      try {
        [response] = await client.synthesizeSpeech({
          input: { text },
          voice: { languageCode: voice.language, name: voice.voiceId },
          audioConfig: { audioEncoding: "MP3" },
        });
      } catch (err) {
        const msg = `Google TTS error: ${String(err)}`;
        console.error(`[TTSService] ${msg}`);
        span.setStatus({ code: SpanStatusCode.ERROR, message: msg });
        throw new TTSProviderError("google", msg);
      }

      const audio = response.audioContent;
      const buffer = Buffer.isBuffer(audio) ? audio : Buffer.from(audio as string, "base64");
      span.setAttribute("tts.audio.size", buffer.length);
      span.setStatus({ code: SpanStatusCode.OK });
      return buffer;
    } finally {
      span.end();
    }
  });
}

// ---------------------------------------------------------------------------
// Storage helpers
// ---------------------------------------------------------------------------

async function saveAudio(buffer: Buffer, outputDir: string, jobId: string): Promise<string> {
  await fs.mkdir(outputDir, { recursive: true });
  const filePath = path.join(outputDir, `${jobId}.mp3`);
  await fs.writeFile(filePath, buffer);
  return filePath;
}

export async function mergeAudioFiles(inputPaths: string[], outputPath: string): Promise<string> {
  const chunks: Buffer[] = [];
  for (const p of inputPaths) {
    chunks.push(await fs.readFile(p));
  }
  await fs.writeFile(outputPath, Buffer.concat(chunks));
  return outputPath;
}

// ---------------------------------------------------------------------------
// TTSService
// ---------------------------------------------------------------------------

export class TTSService {
  private config: TTSConfig;
  private rateLimiter?: RateLimiter;
  private cache?: AudioCache;

  /**
   * Per-provider circuit breakers.  Each breaker wraps the raw provider
   * function so that sustained failures open the circuit and subsequent
   * calls fast-fail (throw `CircuitBreaker.OpenCircuitError`) without
   * hitting the upstream API.
   *
   * Configuration (defaults from DEFAULT_CB_CONFIG):
   *   - openThreshold  : 5 failures in the rolling window → open
   *   - rollingWindowMs: 30 000 ms
   *   - halfOpenInterval: 30 000 ms before probing again
   *   - timeoutMs      : 10 000 ms per call
   */
  private breakers: Map<TTSProvider, CircuitBreaker> = new Map();

  constructor(config: TTSConfig) {
    this.config = config;
    if (config.rateLimit) {
      this.rateLimiter = new RateLimiter(config.rateLimit);
      // Evict expired windows every minute to keep memory bounded
      setInterval(() => this.rateLimiter!.evictExpired(), 60_000).unref();
    }
    if (config.cache) {
      this.cache = new AudioCache(config.cache);
    }
    this._initCircuitBreakers();
  }

  /**
   * Build one circuit breaker per configured TTS provider.  The breaker
   * wraps a thin async action that accepts (text, voice) and delegates to
   * the raw provider implementation.
   */
  private _initCircuitBreakers(): void {
    const cbCfg: Required<CircuitBreakerConfig> = {
      ...DEFAULT_CB_CONFIG,
      ...(this.config.circuitBreaker ?? {}),
    };

    const opossumOptions: CircuitBreaker.Options = {
      // Trip the breaker when ≥ openThreshold failures occur in the window.
      // opossum opens when (failures / total) > errorThresholdPercentage / 100.
      // With errorThresholdPercentage = 50 and volumeThreshold = openThreshold,
      // the circuit opens once the threshold count of all-failure calls is reached.
      volumeThreshold: cbCfg.openThreshold,
      errorThresholdPercentage: 50,
      // Rolling stats window
      rollingCountTimeout: cbCfg.rollingWindowMs,
      // Half-open retry delay
      resetTimeout: cbCfg.halfOpenIntervalMs,
      // Per-call timeout (counted as a failure)
      timeout: cbCfg.timeoutMs,
    };

    if (this.config.elevenlabs) {
      const elBreaker = new CircuitBreaker(
        async (text: string, voice: TTSVoice) =>
          generateElevenLabs(text, voice, this.config.elevenlabs!),
        { ...opossumOptions, name: "elevenlabs" }
      );
      elBreaker.on("open",     () => console.warn("[CircuitBreaker] ElevenLabs circuit OPENED — fast-failing"));
      elBreaker.on("halfOpen", () => console.info ("[CircuitBreaker] ElevenLabs circuit HALF-OPEN — probing"));
      elBreaker.on("close",    () => console.info ("[CircuitBreaker] ElevenLabs circuit CLOSED — recovered"));
      this.breakers.set("elevenlabs", elBreaker);
    }

    if (this.config.google) {
      const gBreaker = new CircuitBreaker(
        async (text: string, voice: TTSVoice) =>
          generateGoogle(text, voice, this.config.google!),
        { ...opossumOptions, name: "google" }
      );
      gBreaker.on("open",     () => console.warn("[CircuitBreaker] Google TTS circuit OPENED — fast-failing"));
      gBreaker.on("halfOpen", () => console.info ("[CircuitBreaker] Google TTS circuit HALF-OPEN — probing"));
      gBreaker.on("close",    () => console.info ("[CircuitBreaker] Google TTS circuit CLOSED — recovered"));
      this.breakers.set("google", gBreaker);
    }
  }

  /**
   * Returns a snapshot of each provider's circuit breaker state.
   * Exposed in /health/ready so operators can see the breaker state
   * without needing to inspect service logs.
   */
  getCircuitBreakerStates(): Record<string, CircuitBreakerState> {
    const result: Record<string, CircuitBreakerState> = {};
    for (const [provider, breaker] of this.breakers) {
      const stats = breaker.stats;
      result[provider] = {
        state: breaker.opened ? "open" : breaker.halfOpen ? "halfOpen" : "closed",
        enabled: !breaker.opened,
        failures: stats.failures,
        successes: stats.successes,
        percentile: stats.percentiles?.[0.5] ?? 0,
      };
    }
    return result;
  }

  /**
   * Enqueue a TTS job and return its ID immediately.
   * @param credential API key or JWT Bearer token (required when auth is configured).
   * @param rateLimitKey IP address or user ID for rate limiting (e.g. "ip:1.2.3.4" or "user:abc").
   * @param bypassCache If true, skip cache lookup and always generate fresh audio.
   */
  enqueue(
    text: string,
    voice: TTSVoice,
    provider?: TTSProvider,
    credential?: string,
    rateLimitKey?: string,
    bypassCache?: boolean
  ): string {
    if (this.config.auth) authenticate(credential, this.config.auth);

    // Rate limiting
    if (this.rateLimiter && rateLimitKey) {
      this.rateLimiter.check(rateLimitKey);
    }

    // Input sanitization
    const sanitized = sanitizeInput(text);

    const id = makeId();
    const job: TTSJob = {
      id,
      text: sanitized,
      voice,
      provider: provider ?? this.config.provider,
      status: "pending",
      createdAt: new Date(),
      updatedAt: new Date(),
      bypassCache: bypassCache || false,
    };
    jobStore.set(id, job);

    this._process(job).catch((err) => {
      const j = jobStore.get(id);
      if (j) {
        j.status = "error";
        j.error = err instanceof Error ? err.message : String(err);
        j.updatedAt = new Date();
        console.error(`[TTSService] Job ${id} failed: ${j.error}`);
      }
    });

    return id;
  }

  getJob(id: string): TTSJob | undefined {
    return jobStore.get(id);
  }

  listJobs(status?: TTSJob["status"]): TTSJob[] {
    const all = Array.from(jobStore.values());
    return status ? all.filter((j) => j.status === status) : all;
  }

  /**
   * Synchronous generation — awaits completion and returns the output path.
   * @param rateLimitKey IP address or user ID for rate limiting.
   * @param bypassCache If true, skip cache lookup and always generate fresh audio.
   */
  async generate(
    text: string,
    voice: TTSVoice,
    provider?: TTSProvider,
    credential?: string,
    rateLimitKey?: string,
    bypassCache?: boolean
  ): Promise<string> {
    const id = this.enqueue(text, voice, provider, credential, rateLimitKey, bypassCache);
    return this._waitForJob(id);
  }

  async generateAndMerge(
    segments: Array<{ text: string; voice: TTSVoice; provider?: TTSProvider }>,
    mergedOutputPath: string,
    credential?: string,
    rateLimitKey?: string
  ): Promise<string> {
    if (this.config.auth) authenticate(credential, this.config.auth);
    const paths = await Promise.all(
      segments.map((s) => this.generate(s.text, s.voice, s.provider, credential, rateLimitKey))
    );
    return mergeAudioFiles(paths, mergedOutputPath);
  }

  getRateLimitMetrics(): RateLimitMetrics | null {
    return this.rateLimiter ? this.rateLimiter.getMetrics() : null;
  }

  getCacheMetrics(): CacheMetrics | null {
    return this.cache ? this.cache.getMetrics() : null;
  }

  // ---------------------------------------------------------------------------
  // Private
  // ---------------------------------------------------------------------------

  private async _process(job: TTSJob): Promise<void> {
    job.status = "processing";
    job.updatedAt = new Date();

    const cacheKey = this.cache && !job.bypassCache
      ? AudioCache.key(job.text, job.voice.voiceId, job.provider)
      : null;

    // Cache hit — write cached buffer to disk and skip provider call
    if (cacheKey && this.cache) {
      const cached = this.cache.get(cacheKey);
      if (cached) {
        const outputPath = await saveAudio(cached, this.config.outputDir, job.id);
        job.outputPath = outputPath;
        job.status = "done";
        job.updatedAt = new Date();
        return;
      }
    }

    const buffer = await this._generateWithFallback(job);

    if (cacheKey && this.cache) {
      this.cache.set(cacheKey, buffer);
    }

    const outputPath = await saveAudio(buffer, this.config.outputDir, job.id);
    job.outputPath = outputPath;
    job.status = "done";
    job.updatedAt = new Date();
  }

  /**
   * Try the requested provider; if it fails and a fallback is available, try that.
   * Logs errors at each step and surfaces a meaningful error if both fail.
   */
  private async _generateWithFallback(job: TTSJob): Promise<Buffer> {
    const primary = job.provider;
    const fallback: TTSProvider = primary === "elevenlabs" ? "google" : "elevenlabs";
    const hasFallback =
      fallback === "google" ? !!this.config.google : !!this.config.elevenlabs;

    try {
      return await this._callProvider(primary, job.text, job.voice);
    } catch (primaryErr) {
      const errMsg = primaryErr instanceof Error ? primaryErr.message : String(primaryErr);
      console.error(`[TTSService] Primary provider "${primary}" failed: ${errMsg}`);

      if (!hasFallback) {
        throw primaryErr instanceof TTSProviderError
          ? primaryErr
          : new TTSProviderError(primary, errMsg);
      }

      console.warn(`[TTSService] Falling back to "${fallback}"`);
      try {
        return await this._callProvider(fallback, job.text, job.voice);
      } catch (fallbackErr) {
        const fbMsg = fallbackErr instanceof Error ? fallbackErr.message : String(fallbackErr);
        console.error(`[TTSService] Fallback provider "${fallback}" also failed: ${fbMsg}`);
        throw new TTSProviderError(
          fallback,
          `Both providers failed. Primary (${primary}): ${errMsg}. Fallback (${fallback}): ${fbMsg}`
        );
      }
    }
  }

  private async _callProvider(
    provider: TTSProvider,
    text: string,
    voice: TTSVoice
  ): Promise<Buffer> {
    const breaker = this.breakers.get(provider);

    if (provider === "elevenlabs") {
      if (!this.config.elevenlabs) throw new TTSProviderError("elevenlabs", "ElevenLabs config missing");
      if (breaker) {
        try {
          return await breaker.fire(text, voice) as Buffer;
        } catch (err) {
          // Re-wrap open-circuit errors as TTSProviderError so callers get a
          // consistent error type and a meaningful 503 status code.
          if (err instanceof Error && err.message.includes("open")) {
            throw new TTSProviderError("elevenlabs", `Circuit breaker OPEN: ${err.message}`, 503);
          }
          throw err;
        }
      }
      return generateElevenLabs(text, voice, this.config.elevenlabs);
    } else {
      if (!this.config.google) throw new TTSProviderError("google", "Google TTS config missing");
      if (breaker) {
        try {
          return await breaker.fire(text, voice) as Buffer;
        } catch (err) {
          if (err instanceof Error && err.message.includes("open")) {
            throw new TTSProviderError("google", `Circuit breaker OPEN: ${err.message}`, 503);
          }
          throw err;
        }
      }
      return generateGoogle(text, voice, this.config.google);
    }
  }

  private _waitForJob(id: string, intervalMs = 200, timeoutMs = 60_000): Promise<string> {
    return new Promise((resolve, reject) => {
      const start = Date.now();
      const tick = setInterval(() => {
        const job = jobStore.get(id);
        if (!job) { clearInterval(tick); return reject(new Error(`Job ${id} not found`)); }
        if (job.status === "done") { clearInterval(tick); return resolve(job.outputPath!); }
        if (job.status === "error") { clearInterval(tick); return reject(new Error(job.error)); }
        if (Date.now() - start > timeoutMs) {
          clearInterval(tick);
          reject(new Error(`Job ${id} timed out after ${timeoutMs}ms`));
        }
      }, intervalMs);
    });
  }
}
