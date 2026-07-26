/**
 * Shared low-level HTTP request layer: retry/backoff, timeout, caching, and
 * error normalization. Used by both `client.ts` (the small set of endpoints
 * the landing page calls) and `admin-client.ts` (the wider internal/admin
 * surface), so neither has to duplicate this logic.
 */

import { getEnvConfig } from '../env';
import { apiCache } from './cache';

const config = getEnvConfig();
const BASE_URL = config.NEXT_PUBLIC_API_URL.replace(/\/$/, "");

export type HttpMethod = "GET" | "POST" | "DELETE";

const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxRetries: 3,
  initialDelayMs: 100,
  maxDelayMs: 10000,
};

/** Per-attempt request timeout in milliseconds. */
const REQUEST_TIMEOUT_MS = 10_000;

/**
 * Cache tag constants used to associate GET responses with resource namespaces
 * and to target only the affected entries when a mutation completes.
 *
 * Invalidation strategy (tag-based):
 *   - Each GET endpoint declares the tags of the resources it reads.
 *   - Each mutation declares the tags of the resources it writes.
 *   - On mutation success, only entries carrying those tags are dropped.
 */
export const CacheTag = {
  STATISTICS: 'statistics',
  MARKETS: 'markets',
  BLOCKCHAIN: 'blockchain',
  NEWSLETTER: 'newsletter',
  EMAIL: 'email',
} as const;

function getRetryDelay(attempt: number, retryAfter?: number): number {
  if (retryAfter) return retryAfter * 1000;
  const base = DEFAULT_RETRY_CONFIG.initialDelayMs * Math.pow(2, attempt);
  // Add up to 25 % random jitter to spread out thundering-herd retries.
  const jitter = Math.random() * base * 0.25;
  return Math.min(base + jitter, DEFAULT_RETRY_CONFIG.maxDelayMs);
}

/**
 * Create a per-attempt abort signal that fires after `timeoutMs` milliseconds.
 * If `userSignal` is provided it is linked: aborting either one aborts the other.
 */
function createRequestSignal(
  timeoutMs: number,
  userSignal?: AbortSignal
): { signal: AbortSignal; clear: () => void } {
  const controller = new AbortController();
  const timerId = setTimeout(() => controller.abort(), timeoutMs);
  const clear = () => clearTimeout(timerId);

  if (userSignal) {
    if (userSignal.aborted) {
      clear();
      controller.abort(userSignal.reason);
    } else {
      userSignal.addEventListener('abort', () => {
        clear();
        controller.abort(userSignal.reason);
      }, { once: true });
    }
  }

  return { signal: controller.signal, clear };
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

export interface RequestOptions {
  body?: unknown;
  params?: Record<string, string | number | undefined>;
  cacheTtl?: number;
  /** Resource tags applied to a cached GET entry, or invalidated on a mutation. */
  cacheTags?: string[];
  maxRetries?: number;
  /** Per-attempt timeout in ms. Defaults to REQUEST_TIMEOUT_MS (10 s). */
  timeoutMs?: number;
  /**
   * Mark a non-GET request as safe to retry on 5xx.
   * Only set this for endpoints that are truly idempotent (e.g. PUT upserts).
   */
  idempotent?: boolean;
  signal?: AbortSignal;
}

export interface RetryConfig {
  maxRetries: number;
  initialDelayMs: number;
  maxDelayMs: number;
}

/**
 * Structured API error with HTTP status code and user-friendly message.
 * Thrown for both network failures and non-2xx responses.
 *
 * Usage:
 *   try { await api.getStatistics() }
 *   catch (e) {
 *     if (e instanceof ApiError) {
 *       console.log(e.status, e.message); // e.g. 404, "Market not found"
 *     }
 *   }
 */
export class ApiError extends Error {
  /** HTTP status code, or 0 for network/connection failures. */
  readonly status: number;
  /** Machine-readable error code from the API (e.g. "NOT_FOUND", "RATE_LIMITED"). */
  readonly code: string;
  /** Optional additional context returned by the API. */
  readonly details?: Record<string, unknown>;

  constructor(message: string, status: number, code = "UNKNOWN_ERROR", details?: Record<string, unknown>) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
    this.details = details;
  }

  /** True for client errors (4xx). */
  get isClientError(): boolean {
    return this.status >= 400 && this.status < 500;
  }

  /** True for server errors (5xx). */
  get isServerError(): boolean {
    return this.status >= 500;
  }

  /** True for network/connection failures (status 0). */
  get isNetworkError(): boolean {
    return this.status === 0;
  }
}

export async function request<T>(
  method: HttpMethod,
  path: string,
  options: RequestOptions = {}
): Promise<T> {
  let url = `${BASE_URL}${path}`;

  if (options.params) {
    const qs = new URLSearchParams();
    for (const [k, v] of Object.entries(options.params)) {
      if (v !== undefined) qs.set(k, String(v));
    }
    const str = qs.toString();
    if (str) url += `?${str}`;
  }

  // Check cache for GET requests
  if (method === "GET" && options.cacheTtl) {
    const cached = apiCache.get<T>(url);
    if (cached !== null) {
      return cached;
    }
  }

  const maxRetries = options.maxRetries ?? DEFAULT_RETRY_CONFIG.maxRetries;
  const timeoutMs = options.timeoutMs ?? REQUEST_TIMEOUT_MS;
  let lastError: Error | null = null;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    const { signal, clear } = createRequestSignal(timeoutMs, options.signal);

    try {
      const res = await fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
        signal,
      });

      clear();

      if (!res.ok) {
        if (res.status === 429) {
          if (attempt < maxRetries) {
            const retryAfter = res.headers.get('Retry-After');
            const delayMs = getRetryDelay(attempt, retryAfter ? parseInt(retryAfter, 10) : undefined);
            await sleep(delayMs);
            continue;
          }
        }

        // Retry transient 5xx errors for safe/idempotent methods only.
        if (res.status >= 500 && attempt < maxRetries && (method === "GET" || options.idempotent)) {
          const delayMs = getRetryDelay(attempt);
          await sleep(delayMs);
          continue;
        }

        let err: unknown;
        try {
          err = await res.json();
        } catch {
          err = {};
        }
        const errObj = (typeof err === 'object' && err !== null) ? err as Record<string, unknown> : {};
        const message = (errObj['message'] as string | undefined) ?? res.statusText ?? `HTTP ${res.status}`;
        const code = (errObj['code'] as string | undefined) ?? "UNKNOWN_ERROR";
        const details = errObj['details'] as Record<string, unknown> | undefined;
        throw new ApiError(message, res.status, code, details);
      }

      // 204 / empty body
      const text = await res.text();
      const data = text ? (JSON.parse(text) as T) : (undefined as unknown as T);

      // Cache GET responses with their resource tags for targeted invalidation.
      if (method === "GET" && options.cacheTtl) {
        apiCache.set(url, data, options.cacheTtl, options.cacheTags);
      }

      // On mutations, invalidate only the affected resource tags instead of
      // the entire cache. Fall back to a full clear for untagged mutations.
      if (method === "POST" || method === "DELETE") {
        if (options.cacheTags?.length) {
          apiCache.invalidateByTags(options.cacheTags);
        } else {
          apiCache.invalidateByPattern('.*');
        }
      }

      return data;
    } catch (networkErr) {
      clear();

      if (networkErr instanceof ApiError) throw networkErr;

      // If the abort came from our timeout (not a caller-supplied signal), surface
      // a distinct TIMEOUT_ERROR so the UI can show a specific message.
      if (networkErr instanceof DOMException && networkErr.name === 'AbortError') {
        if (!options.signal?.aborted) {
          throw new ApiError('The request timed out. Please try again.', 0, 'TIMEOUT_ERROR');
        }
        // Caller-initiated abort: propagate as-is so error boundaries can ignore it.
        throw networkErr;
      }

      lastError = networkErr instanceof Error ? networkErr : new Error(String(networkErr));

      if (attempt < maxRetries && method === "GET") {
        const delayMs = getRetryDelay(attempt);
        await sleep(delayMs);
        continue;
      }

      const msg = lastError.message;
      throw new ApiError(`Unable to reach the server. Please check your connection. (${msg})`, 0);
    }
  }

  throw lastError || new ApiError("Request failed after retries", 0);
}
