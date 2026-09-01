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
import { reportResponseHeaders } from './deprecation';
import { csrfHeaders, isCsrfTokenError } from './csrf';
import { newIdempotencyKey, isValidIdempotencyKey } from './idempotency';
import {
  newsletterSubscribeSchema,
  emailRequestSchema,
  gdprExportSchema,
  placeBetSchema,
} from './requestSchemas';
import { reportRateLimited } from './rateLimit';
import type { paths, components } from './schema';
import type { ZodType } from 'zod';

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

/**
 * `placeBet` is not yet part of services/api/openapi.yaml (see #78), so its
 * path is kept out of the `satisfies keyof paths` check above rather than
 * loosening that check for every other entry.
 */
const PLACE_BET_PATH = "/api/v1/blockchain/markets/{market_id}/bets";

// Path-parameter encoding lives in ./paths. Re-exported here so existing importers of
// `fillPath` from './public-client' keep working.
export { fillPath, fillPathParams } from './paths';
import { fillPath } from './paths';

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
  AUDIT: 'audit',
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
  /**
   * Idempotency key for this logical submission (#1340). `true` generates a
   * fresh UUID v4; a string reuses a caller-supplied key. Whatever value is
   * resolved is sent as the `Idempotency-Key` header on every automatic retry
   * of this `request()` call, so a network-layer retry can't create a
   * duplicate. A new `request()` call is a new logical submission and gets a
   * new key.
   */
  idempotencyKey?: string | true;
  /**
   * Zod schema (derived from the generated OpenAPI types) to validate the
   * request body against before it is sent (#1341). A failure throws an
   * `ApiError` with code `CLIENT_VALIDATION_ERROR` locally instead of letting
   * the server return a raw 400. Schemas are loose, so absent-but-optional
   * fields are never rejected.
   */
  bodySchema?: ZodType;
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

  // De-dupe concurrent GETs to the same URL: two components reading the same
  // resource at once share a single network request (#1334). Mutations are never
  // de-duped - each write is a distinct action.
  if (method === "GET") {
    return apiCache.dedupe(url, () => sendWithRetries<T>(method, url, options));
  }
  return sendWithRetries<T>(method, url, options);
}

async function sendWithRetries<T>(
  method: HttpMethod,
  url: string,
  options: RequestOptions
): Promise<T> {
  const maxRetries = options.maxRetries ?? DEFAULT_RETRY_CONFIG.maxRetries;
  const timeoutMs = options.timeoutMs ?? REQUEST_TIMEOUT_MS;

  // Resolve the idempotency key once, outside the retry loop, so every retry of
  // this logical submission carries the same key (#1340).
  const idempotencyKey =
    options.idempotencyKey === true
      ? newIdempotencyKey()
      : isValidIdempotencyKey(options.idempotencyKey)
        ? options.idempotencyKey
        : undefined;

  // Validate the outgoing body against the generated-schema-derived contract
  // before the first attempt, so a malformed body fails locally with a clear
  // message instead of surfacing as a raw server 400 (#1341).
  if (options.bodySchema && options.body !== undefined) {
    const parsed = options.bodySchema.safeParse(options.body);
    if (!parsed.success) {
      const detail = parsed.error.issues
        .map((issue) => `${issue.path.join('.') || '(root)'}: ${issue.message}`)
        .join('; ');
      throw new ApiError(
        `Request body failed client-side validation: ${detail}`,
        0,
        'CLIENT_VALIDATION_ERROR',
        { issues: parsed.error.issues },
      );
    }
  }

  let lastError: Error | null = null;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    const { signal, clear } = createRequestSignal(timeoutMs, options.signal);

    try {
      const res = await fetch(url, {
        method,
        headers: {
          "Content-Type": "application/json",
          ...csrfHeaders(method),
          ...(idempotencyKey ? { "Idempotency-Key": idempotencyKey } : {}),
        },
        body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
        signal,
      });

      clear();
      reportResponseHeaders(res.headers);

      if (!res.ok) {
        if (res.status === 429) {
          const retryAfter = res.headers.get('Retry-After');
          const retryAfterSec = retryAfter ? parseInt(retryAfter, 10) : NaN;
          // Surface the cooldown to the UI (one shared countdown toast, #1339).
          reportRateLimited(Number.isNaN(retryAfterSec) ? 1 : retryAfterSec);
          if (attempt < maxRetries) {
            const delayMs = getRetryDelay(attempt, Number.isNaN(retryAfterSec) ? undefined : retryAfterSec);
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
        const code = (errObj['code'] as string | undefined) ?? "UNKNOWN_ERROR";
        // A stale/expired CSRF token gets a distinct, actionable message
        // instead of surfacing as a confusing generic 403 (#1417).
        const message = isCsrfTokenError(res.status, code)
          ? "Your session has expired. Please refresh the page and try again."
          : (errObj['message'] as string | undefined) ?? res.statusText ?? `HTTP ${res.status}`;
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

/**
 * Soroban contract error code for "market not yet resolved" (see
 * CONTRACT_ERROR_MESSAGES[147] in admin-client.ts / docs/CONTRACT_ERRORS.md).
 *
 * This is deliberately duplicated as a single constant here — rather than
 * importing the full CONTRACT_ERROR_MESSAGES map from admin-client.ts — so
 * that public pages can recognize this one, routine, pre-resolution state
 * without pulling the entire admin-only error catalog into the public
 * bundle (see the module doc comment above).
 */
export const MARKET_NOT_RESOLVED_CODE = 147;

/**
 * True when `error` is the "market not yet resolved" contract error. This is
 * an expected, routine state for any market before resolution — not a
 * failure — so callers (payout/claim UI) should render an informational
 * "resolution pending" state instead of a destructive error toast (see #1369).
 */
export function isMarketNotResolvedError(error: unknown): boolean {
  if (!(error instanceof ApiError)) return false;
  if (error.code !== 'CONTRACT_ERROR') return false;
  return error.details?.['contract_code'] === MARKET_NOT_RESOLVED_CODE;
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

  /**
   * Submits a bet for the connected wallet. Returns the pending on-chain tx.
   *
   * An idempotency key is generated per call so an automatic network retry
   * can't place the bet twice (#1340). A caller that also needs to survive a
   * double-click (see #13) can pass its own `idempotencyKey` and hold it
   * stable until the submission succeeds; editing the amount/outcome and
   * resubmitting is a new logical submission and should use a new key.
   */
  placeBet: (
    marketId: number | string,
    body: { wallet: string; outcome: number; amount: string },
    options: { idempotencyKey?: string; signal?: AbortSignal } = {}
  ) =>
    request<{ tx_hash: string; status: string }>(
      "POST",
      fillPath(PLACE_BET_PATH, 'market_id', marketId),
      {
        body,
        bodySchema: placeBetSchema,
        cacheTags: [CacheTag.BLOCKCHAIN, CacheTag.MARKETS],
        idempotencyKey: options.idempotencyKey ?? true,
        signal: options.signal,
      },
    ),

  // Newsletter (public subscription / self-service)
  newsletterSubscribe: (body: { email: string; source?: string }, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("POST", "/api/v1/newsletter/subscribe", {
      body,
      bodySchema: newsletterSubscribeSchema,
      cacheTags: [CacheTag.NEWSLETTER, CacheTag.STATISTICS],
      // The subscribe endpoint declares Idempotency-Key in openapi.yaml; a
      // retry of a submit that already succeeded server-side then returns the
      // same result instead of a spurious "already subscribed" (#1340).
      idempotencyKey: true,
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
      bodySchema: emailRequestSchema,
      cacheTags: [CacheTag.NEWSLETTER, CacheTag.STATISTICS],
      signal,
    }),

  newsletterGdprRequestToken: (body: { email: string }, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>(
      "POST",
      "/api/v1/newsletter/gdpr/request-token",
      { body, bodySchema: emailRequestSchema, cacheTags: [CacheTag.NEWSLETTER], signal }
    ),

  newsletterGdprExport: (
    body: { email: string; token: string } | string,
    signal?: AbortSignal
  ) =>
    request<{ success: boolean; data: Record<string, unknown>; message?: string }>(
      "POST",
      "/api/v1/newsletter/gdpr/export",
      {
        body: typeof body === 'string' ? { email: body } : body,
        bodySchema: gdprExportSchema,
        cacheTags: [CacheTag.NEWSLETTER],
        signal,
      }
    ),

  newsletterGdprDelete: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/gdpr/delete", {
      body: { email },
      bodySchema: emailRequestSchema,
      cacheTags: [CacheTag.NEWSLETTER],
      signal,
    }),
};
