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
import { createHash, createHmac, timingSafeEqual } from "crypto";
import { trace, SpanStatusCode, Span } from "@opentelemetry/api";
import CircuitBreaker from "opossum";
import type { SharedStoreConfig } from "./SharedStore";

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
  /**
   * Identity of the credential that created this job (the API key itself,
   * or the JWT `sub` claim). Used to enforce per-tenant access on
   * getJob/listJobs so one credential cannot read another's jobs.
   * `ANONYMOUS_OWNER` when no auth is configured (single-tenant deployment).
   */
  owner: string;
}

/** Owner tag used for jobs created when no auth is configured. */
export const ANONYMOUS_OWNER = "anonymous";

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
// Retry with exponential backoff + full jitter (issue #994)
// ---------------------------------------------------------------------------

export interface RetryConfig {
  /** Maximum number of retry attempts (not counting the first attempt). Default 3. */
  maxRetries: number;
  /** Maximum delay between retries in milliseconds. Default 60 000. */
  maxDelayMs: number;
}

/** Returns true for transient errors that warrant a retry (429, 5xx). */
function isRetryable(err: unknown): boolean {
  if (err instanceof TTSProviderError) {
    const code = err.statusCode;
    // 400, 401, 403 are non-retriable client / auth errors
    if (code === 400 || code === 401 || code === 403) return false;
    return code === 429 || code >= 500;
  }
  // Network-level errors (no statusCode) are always retriable
  return true;
}

/** Full-jitter exponential backoff: delay ∈ [0, min(maxDelayMs, 1000 * 2^attempt)]. */
export async function backoffDelay(attempt: number, maxDelayMs: number): Promise<void> {
  const cap = Math.min(maxDelayMs, 1000 * Math.pow(2, attempt));
  const ms = Math.random() * cap;
  await new Promise<void>((resolve) => setTimeout(resolve, ms));
}

/** Call `fn` and retry up to `config.maxRetries` times on transient errors. */
export async function withRetry<T>(
  fn: () => Promise<T>,
  config: RetryConfig,
  label = "operation"
): Promise<T> {
  let lastErr: unknown;
  for (let attempt = 0; attempt <= config.maxRetries; attempt++) {
    try {
      return await fn();
    } catch (err) {
      lastErr = err;
      if (!isRetryable(err) || attempt === config.maxRetries) throw err;
      console.warn(
        `[TTSService] ${label} failed (attempt ${attempt + 1}/${config.maxRetries + 1}), retrying after backoff…`
      );
      await backoffDelay(attempt, config.maxDelayMs);
    }
  }
  throw lastErr;
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
  /** Retry config for transient provider errors — omit to use defaults */
  retry?: RetryConfig;
  /**
   * Shared, cross-replica backing (e.g. Redis) for job status, rate
   * limiting, and cache (issue #1133). Omit to keep process-local in-memory
   * state, which only gives correct results for a single instance.
   */
  sharedStore?: SharedStoreConfig;
  /**
   * TTL in milliseconds for completed/errored jobs before they're evicted
   * from the in-memory job store. Omit to use the default (1 hour).
   */
  jobTtlMs?: number;
  /**
   * Retention period in milliseconds for generated audio files in
   * `outputDir` before they're deleted by the periodic cleanup sweep.
   * Omit to use the default (24 hours).
   */
  audioRetentionMs?: number;
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

/**
 * Constant-time string comparison to prevent timing side-channel attacks
 * (CWE-208). Both inputs are padded to equal length before comparison so
 * that `timingSafeEqual` always receives same-length buffers.
 */
function constantTimeEqual(a: string, b: string): boolean {
  const bufA = Buffer.from(a);
  const bufB = Buffer.from(b);
  const len = Math.max(bufA.length, bufB.length);
  const paddedA = Buffer.alloc(len, 0);
  const paddedB = Buffer.alloc(len, 0);
  bufA.copy(paddedA);
  bufB.copy(paddedB);
  return timingSafeEqual(paddedA, paddedB);
}

/**
 * Verify `credential` against `auth` and return a stable tenant identity for it:
 * - API key auth: the key itself is the tenant boundary.
 * - JWT auth: the `sub` claim if present, otherwise the raw credential.
 * Throws `AuthError` if the credential is missing or invalid.
 */
export function authenticate(credential: string | undefined, auth: AuthConfig): string {
  if (!credential) throw new AuthError("Missing credential");

  if (auth.type === "apikey") {
    let keyValid = 0;
    const credBuf = Buffer.from(credential);
    for (const key of auth.keys) {
      const keyBuf = Buffer.from(key);
      const len = Math.max(credBuf.length, keyBuf.length);
      const paddedCred = Buffer.alloc(len, 0);
      const paddedKey = Buffer.alloc(len, 0);
      credBuf.copy(paddedCred);
      keyBuf.copy(paddedKey);
      keyValid |= timingSafeEqual(paddedCred, paddedKey) ? 1 : 0;
    }
    if (keyValid === 0) throw new AuthError("Invalid API key");
    return credential;
  }

  const parts = credential.split(".");
  if (parts.length !== 3) throw new AuthError("Malformed JWT");

  const [headerB64, payloadB64, sigB64] = parts;
  const expected = createHmac("sha256", auth.secret)
    .update(`${headerB64}.${payloadB64}`)
    .digest("base64url");

  if (!constantTimeEqual(expected, sigB64)) throw new AuthError("Invalid JWT signature");

  const payload = JSON.parse(Buffer.from(payloadB64, "base64url").toString());
  if (payload.exp === undefined || payload.exp < Math.floor(Date.now() / 1000)) {
    throw new AuthError("JWT expired");
  }

  return typeof payload.sub === "string" && payload.sub ? payload.sub : credential;
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

/** Default TTL for completed/errored jobs before eviction: 1 hour. */
export const DEFAULT_JOB_TTL_MS = 60 * 60 * 1000;

function makeId(): string {
  return `tts_${Date.now()}_${Math.random().toString(36).slice(2, 8)}`;
}

/**
 * Evict terminal-state (done/error) jobs older than `ttlMs`, keeping
 * jobStore bounded under sustained traffic. Pending/processing jobs are
 * never evicted here.
 */
export function evictExpiredJobs(ttlMs: number): void {
  const now = Date.now();
  for (const [id, job] of jobStore) {
    if (
      (job.status === "done" || job.status === "error") &&
      now - job.updatedAt.getTime() >= ttlMs
    ) {
      jobStore.delete(id);
    }
  }
}

// ---------------------------------------------------------------------------
// Provider implementations
// ---------------------------------------------------------------------------

async function generateElevenLabs(
  text: string,
  voice: TTSVoice,
  config: NonNullable<TTSConfig["elevenlabs"]>,
  timeoutMs: number = DEFAULT_CB_CONFIG.timeoutMs
): Promise<Buffer> {
  const tracer = trace.getTracer("tts-service");
  return tracer.startActiveSpan("elevenlabs.generate", async (span: Span) => {
    try {
      span.setAttribute("tts.provider", "elevenlabs");
      span.setAttribute("tts.voice.id", voice.voiceId);
      span.setAttribute("tts.text.length", text.length);

      const modelId = config.modelId ?? "eleven_multilingual_v2";
      const url = `https://api.elevenlabs.io/v1/text-to-speech/${voice.voiceId}`;

      // Tie an AbortController to the breaker's timeout so a timed-out call
      // actually cancels the in-flight HTTP request instead of letting it
      // run to completion in the background.
      const controller = new AbortController();
      const abortTimer = setTimeout(() => controller.abort(), timeoutMs);

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
          signal: controller.signal,
        });
      } catch (networkErr) {
        const isAbort = networkErr instanceof Error && networkErr.name === "AbortError";
        const msg = isAbort
          ? `ElevenLabs request aborted after exceeding timeout of ${timeoutMs}ms`
          : `Network error calling ElevenLabs: ${String(networkErr)}`;
        console.error(`[TTSService] ${msg}`);
        span.setStatus({ code: SpanStatusCode.ERROR, message: msg });
        throw new TTSProviderError("elevenlabs", msg);
      } finally {
        clearTimeout(abortTimer);
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
  config: NonNullable<TTSConfig["google"]>,
  timeoutMs: number = DEFAULT_CB_CONFIG.timeoutMs
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
          synthesizeSpeech: (
            req: object,
            options?: object
          ) => Promise<[{ audioContent: Buffer | string }]>;
        };
      };

      const client = new TextToSpeechClient(config);

      let response: { audioContent: Buffer | string };
      try {
        // Pass a gax deadline so the underlying gRPC call is actually
        // cancelled at the timeout boundary instead of running in the
        // background after the circuit breaker gives up on it.
        [response] = await client.synthesizeSpeech(
          {
            input: { text },
            voice: { languageCode: voice.language, name: voice.voiceId },
            audioConfig: { audioEncoding: "MP3" },
          },
          { timeout: timeoutMs }
        );
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

/** Default retention period for generated audio files: 24 hours. */
export const DEFAULT_AUDIO_RETENTION_MS = 24 * 60 * 60 * 1000;

/**
 * Delete files in `outputDir` whose mtime is older than `retentionMs`.
 * Guards against unbounded disk growth since saveAudio() never deletes
 * what it writes. Missing directory or per-file races are ignored.
 */
export async function cleanupOldAudioFiles(outputDir: string, retentionMs: number): Promise<void> {
  let entries: string[];
  try {
    entries = await fs.readdir(outputDir);
  } catch {
    return;
  }

  const now = Date.now();
  await Promise.all(
    entries.map(async (name) => {
      const filePath = path.join(outputDir, name);
      try {
        const stat = await fs.stat(filePath);
        if (stat.isFile() && now - stat.mtimeMs >= retentionMs) {
          await fs.unlink(filePath);
        }
      } catch {
        // File may have been removed concurrently; ignore.
      }
    })
  );
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
  private cbConfig: Required<CircuitBreakerConfig> = DEFAULT_CB_CONFIG;

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
    const jobTtlMs = config.jobTtlMs ?? DEFAULT_JOB_TTL_MS;
    // Evict completed/errored jobs past their TTL every minute to keep
    // jobStore memory bounded (MAX_QUEUE_DEPTH only guards pending/processing).
    setInterval(() => evictExpiredJobs(jobTtlMs), 60_000).unref();

    const audioRetentionMs = config.audioRetentionMs ?? DEFAULT_AUDIO_RETENTION_MS;
    // Sweep outputDir every 15 minutes so generated .mp3 files don't
    // accumulate forever and fill the (often small, ephemeral) disk.
    setInterval(() => {
      cleanupOldAudioFiles(this.config.outputDir, audioRetentionMs).catch((err) => {
        console.error(`[TTSService] Audio cleanup sweep failed: ${String(err)}`);
      });
    }, 15 * 60_000).unref();

    this._initCircuitBreakers();
  }

  /**
   * Build one circuit breaker per configured TTS provider.  The breaker
   * wraps the *entire retried request* (raw provider call + backoff
   * retries), not a single raw call.  This is deliberate: firing the
   * breaker once per retry attempt would let one failed user request
   * inflate the breaker's failure count by up to `maxRetries + 1`, tripping
   * it far earlier than the configured threshold intends, and — once
   * open — a retry loop wrapped *around* the breaker would keep retrying
   * the "circuit open" error with full exponential backoff instead of
   * failing fast (see issue #1131). By wrapping retries *inside* the
   * breaker action, opossum's `fire()` is called exactly once per external
   * request, records exactly one success/failure per request, and — when
   * already open — rejects immediately without invoking the action (and
   * therefore without running any retry/backoff logic) at all.
   */
  private _initCircuitBreakers(): void {
    const cbCfg: Required<CircuitBreakerConfig> = {
      ...DEFAULT_CB_CONFIG,
      ...(this.config.circuitBreaker ?? {}),
    };
    this.cbConfig = cbCfg;

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
      // Per-call timeout (counted as a failure). Bounds the whole retried
      // request, since the action below includes the retry/backoff loop.
      timeout: cbCfg.timeoutMs,
      // Issue #1135: 4xx TTSProviderErrors (bad voice, invalid credential, ...)
      // are client/config mistakes, not upstream provider health signals.
      // Excluding them from breaker accounting stops a handful of bad
      // requests from tripping the breaker for every other user of the
      // provider. Genuine 5xx/network failures (no statusCode, or >= 500)
      // still count as failures.
      errorFilter: (err: unknown) =>
        err instanceof TTSProviderError && err.statusCode >= 400 && err.statusCode < 500,
    };

    if (this.config.elevenlabs) {
      const elBreaker = new CircuitBreaker(
        async (text: string, voice: TTSVoice) =>
          withRetry(
            () => generateElevenLabs(text, voice, this.config.elevenlabs!, cbCfg.timeoutMs),
            this._retryConfig(),
            "provider:elevenlabs"
          ),
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
          withRetry(
            () => generateGoogle(text, voice, this.config.google!, cbCfg.timeoutMs),
            this._retryConfig(),
            "provider:google"
          ),
        { ...opossumOptions, name: "google" }
      );
      gBreaker.on("open",     () => console.warn("[CircuitBreaker] Google TTS circuit OPENED — fast-failing"));
      gBreaker.on("halfOpen", () => console.info ("[CircuitBreaker] Google TTS circuit HALF-OPEN — probing"));
      gBreaker.on("close",    () => console.info ("[CircuitBreaker] Google TTS circuit CLOSED — recovered"));
      this.breakers.set("google", gBreaker);
    }
  }

  private _retryConfig(): RetryConfig {
    return {
      maxRetries: this.config.retry?.maxRetries ?? 3,
      maxDelayMs: this.config.retry?.maxDelayMs ?? 60_000,
    };
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
   *
   * Rate limiting and job visibility are process-local here — use
   * `enqueueAsync` when `config.sharedStore` is configured so multiple
   * replicas behind a load balancer see consistent state (issue #1133).
   *
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
    const owner = this._resolveOwner(credential);

    // Rate limiting
    if (this.rateLimiter && rateLimitKey) {
      this.rateLimiter.check(rateLimitKey);
    }

    const job = this._createJob(text, voice, provider, bypassCache, owner);
    jobStore.set(job.id, job);
    this._process(job).catch((err) => this._handleProcessError(job.id, err));
    return job.id;
  }

  /**
   * Async equivalent of `enqueue`. When `config.sharedStore` is configured,
   * rate limiting is enforced against the shared counter (consistent across
   * replicas) and the job is written to the shared job store immediately so
   * `getJobAsync` on another replica can see it right away, before
   * processing even completes (issue #1133).
   */
  async enqueueAsync(
    text: string,
    voice: TTSVoice,
    provider?: TTSProvider,
    credential?: string,
    rateLimitKey?: string,
    bypassCache?: boolean
  ): Promise<string> {
    const owner = this._resolveOwner(credential);

    if (rateLimitKey) {
      const sharedRateLimit = this.config.sharedStore?.rateLimitStore;
      if (sharedRateLimit && this.config.rateLimit) {
        const count = await sharedRateLimit.incr(rateLimitKey, this.config.rateLimit.windowMs);
        if (count > this.config.rateLimit.maxRequests) {
          throw new RateLimitError(
            `Rate limit exceeded for key "${rateLimitKey}": ${count}/${this.config.rateLimit.maxRequests} in ${this.config.rateLimit.windowMs}ms`
          );
        }
      } else if (this.rateLimiter) {
        this.rateLimiter.check(rateLimitKey);
      }
    }

    const job = this._createJob(text, voice, provider, bypassCache, owner);
    jobStore.set(job.id, job);
    await this._persistJob(job);

    this._process(job).catch((err) => this._handleProcessError(job.id, err));
    return job.id;
  }

  /**
   * Resolve `credential` to a stable tenant identity, enforcing auth if
   * configured. Used both to tag newly created jobs with their owner and to
   * check ownership on lookup, so the two paths can never disagree.
   */
  private _resolveOwner(credential?: string): string {
    if (!this.config.auth) return ANONYMOUS_OWNER;
    return authenticate(credential, this.config.auth);
  }

  /**
   * Look up a job by ID, scoped to the requesting credential's tenant.
   * Process-local lookup only — use `getJobAsync` under `config.sharedStore`
   * so polling succeeds regardless of which replica handled the job (issue #1133).
   * Returns `undefined` both when the job doesn't exist and when it exists
   * but belongs to a different tenant — the two cases are indistinguishable
   * to the caller so a credential cannot probe for other tenants' job IDs.
   */
  getJob(id: string, credential?: string): TTSJob | undefined {
    const owner = this._resolveOwner(credential);
    const job = jobStore.get(id);
    if (!job || job.owner !== owner) return undefined;
    return job;
  }

  /**
   * Looks up a job locally first (fast path for the replica that processed
   * it), falling back to the shared store so polling succeeds regardless of
   * which replica originally handled the job (issue #1133). Scoped to the
   * requesting credential's tenant like `getJob`, on both the local and
   * shared-store paths.
   */
  async getJobAsync(id: string, credential?: string): Promise<TTSJob | undefined> {
    const owner = this._resolveOwner(credential);
    const local = jobStore.get(id);
    if (local) return local.owner === owner ? local : undefined;
    const remote = await this.config.sharedStore?.jobStore?.getJob(id);
    if (!remote || remote.owner !== owner) return undefined;
    return remote;
  }

  /** List jobs belonging to the requesting credential's tenant, optionally filtered by status. */
  listJobs(status?: TTSJob["status"], credential?: string): TTSJob[] {
    const owner = this._resolveOwner(credential);
    const all = Array.from(jobStore.values()).filter((j) => j.owner === owner);
    return status ? all.filter((j) => j.status === status) : all;
  }

  /**
   * Async equivalent of `listJobs`, preferring the shared store when
   * configured so listings are consistent across replicas (issue #1133).
   * Scoped to the requesting credential's tenant like `listJobs`.
   */
  async listJobsAsync(status?: TTSJob["status"], credential?: string): Promise<TTSJob[]> {
    const owner = this._resolveOwner(credential);
    if (this.config.sharedStore?.jobStore) {
      const all = await this.config.sharedStore.jobStore.listJobs(status);
      return all.filter((j) => j.owner === owner);
    }
    return this.listJobs(status, credential);
  }

  /**
   * Returns jobs across *all* tenants, optionally filtered by status.
   * For internal operational use only (health checks, queue-depth metrics)
   * — never expose this over a tenant-facing API, since it bypasses the
   * ownership scoping that `listJobs`/`getJob` enforce.
   */
  listAllJobsUnscoped(status?: TTSJob["status"]): TTSJob[] {
    const all = Array.from(jobStore.values());
    return status ? all.filter((j) => j.status === status) : all;
  }

  /**
   * Synchronous generation — awaits completion and returns the output path.
   * Uses `enqueueAsync` internally so rate limiting is consistent across
   * replicas when `config.sharedStore` is configured.
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
    const id = await this.enqueueAsync(text, voice, provider, credential, rateLimitKey, bypassCache);
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

  /** Builds a pending TTSJob. Shared by `enqueue` and `enqueueAsync`. */
  private _createJob(
    text: string,
    voice: TTSVoice,
    provider: TTSProvider | undefined,
    bypassCache: boolean | undefined,
    owner: string
  ): TTSJob {
    const sanitized = sanitizeInput(text);
    return {
      id: makeId(),
      text: sanitized,
      voice,
      provider: provider ?? this.config.provider,
      status: "pending",
      createdAt: new Date(),
      updatedAt: new Date(),
      bypassCache: bypassCache || false,
      owner,
    };
  }

  /** Writes a job to the shared store, if one is configured (issue #1133). No-op otherwise. */
  private async _persistJob(job: TTSJob): Promise<void> {
    if (this.config.sharedStore?.jobStore) {
      await this.config.sharedStore.jobStore.setJob(job);
    }
  }

  private _handleProcessError(id: string, err: unknown): void {
    const job = jobStore.get(id);
    if (!job) return;
    job.status = "error";
    job.error = err instanceof Error ? err.message : String(err);
    job.updatedAt = new Date();
    console.error(`[TTSService] Job ${id} failed: ${job.error}`);
    this._persistJob(job).catch((persistErr) =>
      console.error(`[TTSService] Failed to persist job ${id} error state: ${persistErr}`)
    );
  }

  /** Prefers the shared cache store when configured so cache hit rate isn't split across replicas (issue #1133). */
  private async _cacheGet(key: string): Promise<Buffer | undefined> {
    const shared = this.config.sharedStore?.cacheStore;
    if (shared) return shared.get(key);
    return this.cache?.get(key);
  }

  private async _cacheSet(key: string, buffer: Buffer): Promise<void> {
    const shared = this.config.sharedStore?.cacheStore;
    if (shared) {
      await shared.set(key, buffer, this.config.cache?.ttlMs ?? 86_400_000);
      return;
    }
    this.cache?.set(key, buffer);
  }

  private async _process(job: TTSJob): Promise<void> {
    job.status = "processing";
    job.updatedAt = new Date();
    await this._persistJob(job);

    const cachingEnabled = (this.cache || this.config.sharedStore?.cacheStore) && !job.bypassCache;
    const cacheKey = cachingEnabled
      ? AudioCache.key(job.text, job.voice.voiceId, job.provider)
      : null;

    // Cache hit — write cached buffer to disk and skip provider call
    if (cacheKey) {
      const cached = await this._cacheGet(cacheKey);
      if (cached) {
        const outputPath = await saveAudio(cached, this.config.outputDir, job.id);
        job.outputPath = outputPath;
        job.status = "done";
        job.updatedAt = new Date();
        await this._persistJob(job);
        return;
      }
    }

    const buffer = await this._generateWithFallback(job);

    if (cacheKey) {
      await this._cacheSet(cacheKey, buffer);
    }

    const outputPath = await saveAudio(buffer, this.config.outputDir, job.id);
    job.outputPath = outputPath;
    job.status = "done";
    job.updatedAt = new Date();
    await this._persistJob(job);
  }

  /**
   * Try the requested provider; if it fails and a fallback is available, try that.
   * Transient errors (429, 5xx) are retried with exponential backoff + full jitter
   * *inside* `_callProvider`'s circuit breaker action (see `_initCircuitBreakers`),
   * so each provider is attempted at most once per call here — retries are not
   * layered on top of the breaker, which would defeat its fast-fail behavior.
   * Non-retriable errors (400, 401, 403) propagate immediately.
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
        // Retries happen inside the breaker's action (see
        // _initCircuitBreakers) — fire() is called exactly once here per
        // external request, and rejects immediately without retry/backoff
        // when the circuit is already open (fail-fast, not retried by any
        // outer caller — see _generateWithFallback).
        return await this._fireBreaker("elevenlabs", breaker, text, voice);
      }
      return withRetry(
        () => generateElevenLabs(text, voice, this.config.elevenlabs!, this.cbConfig.timeoutMs),
        this._retryConfig(),
        "provider:elevenlabs"
      );
    } else {
      if (!this.config.google) throw new TTSProviderError("google", "Google TTS config missing");
      if (breaker) {
        return await this._fireBreaker("google", breaker, text, voice);
      }
      return withRetry(
        () => generateGoogle(text, voice, this.config.google!, this.cbConfig.timeoutMs),
        this._retryConfig(),
        "provider:google"
      );
    }
  }

  private async _fireBreaker(
    provider: TTSProvider,
    breaker: CircuitBreaker,
    text: string,
    voice: TTSVoice
  ): Promise<Buffer> {
    try {
      return await breaker.fire(text, voice) as Buffer;
    } catch (err) {
      if (err instanceof Error && err.message.includes("open")) {
        throw new TTSProviderError(provider, `Circuit breaker OPEN: ${err.message}`, 503);
      }
      throw err;
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
