/**
 * Public API client — safe to import from the landing-page bundle.
 *
 * This module contains ONLY the endpoints needed by public-facing pages
 * (statistics, featured markets, newsletter, blockchain read-only).
 * It deliberately excludes admin endpoints (resolveMarket, email*, GDPR delete)
 * and the full Soroban contract error-code map so that none of that code
 * ships to every visitor's browser.
 *
 * Admin code lives in ./admin-client.ts.
 * The legacy ./client.ts barrel re-exports everything from admin-client.ts
 * to keep existing non-public imports working without changes.
 */

import { getEnvConfig } from '../env';
import { apiCache, CACHE_TTL } from './cache';
import type { paths, components } from './schema';

const config = getEnvConfig();
const BASE_URL = config.NEXT_PUBLIC_API_URL.replace(/\/$/, "");

type HttpMethod = "GET" | "POST" | "DELETE";

/**
 * Request paths, keyed by client method name. Each value is checked with
 * `satisfies keyof paths` against schema.d.ts (auto-generated from
 * services/api/openapi.yaml) so that renaming or removing a path on the
 * backend fails this file's type check instead of shipping a silent 404
 * (see #50, #51).
 */
const PATHS = {
  statistics: "/api/v1/statistics",
  featuredMarkets: "/api/v1/markets/featured",
  content: "/api/v1/content",
  blockchainHealth: "/api/v1/blockchain/health",
  blockchainMarket: "/api/v1/blockchain/markets/{market_id}",
  blockchainStats: "/api/v1/blockchain/stats",
  userBets: "/api/v1/blockchain/users/{user}/bets",
  oracleResult: "/api/v1/blockchain/oracle/{market_id}",
  transactionStatus: "/api/v1/blockchain/tx/{tx_hash}",
} satisfies Record<string, keyof paths>;

/** Fills a `{placeholder}` segment of a schema path template with an encoded value. */
export function fillPath(template: string, placeholder: string, value: string | number): string {
  return template.replace(`{${placeholder}}`, encodeURIComponent(value));
}

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

interface RequestOptions {
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

interface RetryConfig {
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

async function request<T>(
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
      //
      // Guard: some endpoints return { success: false, message: '...' } with a
      // 200 status to signal business-logic failure (e.g. newsletterSubscribe).
      // Only bust the cache when the mutation actually succeeded — i.e. when the
      // response has no `success` field (non-envelope endpoints) or when
      // `success` is explicitly `true`.
      if (method === "POST" || method === "DELETE") {
        const bodyObj = (typeof data === 'object' && data !== null) ? data as Record<string, unknown> : null;
        const succeeded = bodyObj === null || !('success' in bodyObj) || bodyObj['success'] === true;
        if (succeeded) {
          if (options.cacheTags?.length) {
            apiCache.invalidateByTags(options.cacheTags);
          } else {
            apiCache.invalidateByPattern('.*');
          }
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

// ---------------------------------------------------------------------------
// Public endpoints only — no admin methods below this line
// ---------------------------------------------------------------------------

export const api = {
  health: (signal?: AbortSignal) => request<string>("GET", "/health", { signal }),

  getStatistics: (signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>("GET", PATHS.statistics, {
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.STATISTICS],
      signal,
    }),

  getFeaturedMarkets: (signal?: AbortSignal) =>
    request<components['schemas']['FeaturedMarketView'][]>("GET", PATHS.featuredMarkets, {
      cacheTtl: CACHE_TTL.SHORT,
      cacheTags: [CacheTag.MARKETS],
      signal,
    }),

  getContent: (params?: { page?: number; page_size?: number }, signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>("GET", PATHS.content, { params, cacheTtl: CACHE_TTL.MEDIUM, signal }),

  // Blockchain (read-only)
  getBlockchainHealth: (signal?: AbortSignal) =>
    request<components['schemas']['BlockchainHealth']>("GET", PATHS.blockchainHealth, {
      cacheTtl: CACHE_TTL.SHORT,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  getBlockchainMarket: (marketId: number | string, signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>(
      "GET",
      fillPath(PATHS.blockchainMarket, 'market_id', marketId),
      { cacheTtl: CACHE_TTL.MEDIUM, cacheTags: [CacheTag.BLOCKCHAIN, CacheTag.MARKETS], signal },
    ),

  getBlockchainStats: (signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>("GET", PATHS.blockchainStats, {
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  getUserBets: (user: string, params?: { page?: number; page_size?: number }, signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>("GET", fillPath(PATHS.userBets, 'user', user), {
      params,
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  getOracleResult: (marketId: number | string, signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>(
      "GET",
      fillPath(PATHS.oracleResult, 'market_id', marketId),
      { cacheTtl: CACHE_TTL.LONG, cacheTags: [CacheTag.BLOCKCHAIN], signal },
    ),

  getTransactionStatus: (txHash: string, signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>("GET", fillPath(PATHS.transactionStatus, 'tx_hash', txHash), {
      cacheTtl: CACHE_TTL.LONG,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  // Newsletter (public subscription / self-service)
  newsletterSubscribe: (body: { email: string; source?: string }, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("POST", "/api/v1/newsletter/subscribe", {
      body,
      cacheTags: [CacheTag.NEWSLETTER, CacheTag.STATISTICS],
      signal,
    }),

  newsletterConfirm: (token: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("GET", `/api/v1/newsletter/confirm`, {
      params: { token },
      cacheTags: [CacheTag.NEWSLETTER],
      signal,
    }),

  newsletterUnsubscribe: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/unsubscribe", {
      body: { email },
      cacheTags: [CacheTag.NEWSLETTER, CacheTag.STATISTICS],
      signal,
    }),

  newsletterGdprExport: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; data: Record<string, unknown> }>(
      "POST",
      "/api/v1/newsletter/gdpr/export",
      { body: { email }, cacheTags: [CacheTag.NEWSLETTER], signal }
    ),

  newsletterGdprDelete: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/gdpr/delete", {
      body: { email },
      cacheTags: [CacheTag.NEWSLETTER],
      signal,
    }),
};
