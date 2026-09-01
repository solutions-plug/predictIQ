/**
 * Admin API client — NOT for use in the public landing-page bundle.
 *
 * Extends the public client with admin-only endpoints and the full Soroban
 * contract error-code map. Import this only from admin/internal pages and
 * server-side code, never from components that ship to every visitor.
 *
 * Public-safe endpoints live in ./public-client.ts.
 * The legacy ./client.ts barrel re-exports everything from this module
 * so existing imports continue to work without changes.
 */

export {
  api as publicApi,
  ApiError,
  CacheTag,
} from './public-client';

import { api as publicApi, CacheTag } from './public-client';
import { fillPath } from './paths';
import { apiCache, CACHE_TTL } from './cache';
import { reportResponseHeaders } from './deprecation';
import { reportRateLimited } from './rateLimit';
import { getEnvConfig } from '../env';
import { emailTestSchema } from './requestSchemas';
import type { paths, components } from './schema';
import type { ZodType } from 'zod';

const config = getEnvConfig();
const BASE_URL = config.NEXT_PUBLIC_API_URL.replace(/\/$/, "");

type HttpMethod = "GET" | "POST" | "DELETE";

/**
 * Admin-only request paths, checked with `satisfies keyof paths` against
 * schema.d.ts (see the matching comment in public-client.ts, and #50/#51).
 */
const PATHS = {
  resolveMarket: "/api/v1/markets/{market_id}/resolve",
  blockchainReplay: "/api/blockchain/replay",
  content: "/api/v1/content",
  auditLogs: "/api/v1/audit/logs",
} satisfies Record<string, keyof paths>;

// ---------------------------------------------------------------------------
// Internal request helper (mirrors public-client; kept private to this module)
// ---------------------------------------------------------------------------

const DEFAULT_RETRY_CONFIG = {
  maxRetries: 3,
  initialDelayMs: 100,
  maxDelayMs: 10000,
};

const REQUEST_TIMEOUT_MS = 10_000;

function getRetryDelay(attempt: number, retryAfter?: number): number {
  if (retryAfter) return retryAfter * 1000;
  const base = DEFAULT_RETRY_CONFIG.initialDelayMs * Math.pow(2, attempt);
  const jitter = Math.random() * base * 0.25;
  return Math.min(base + jitter, DEFAULT_RETRY_CONFIG.maxDelayMs);
}

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
  cacheTags?: string[];
  maxRetries?: number;
  timeoutMs?: number;
  idempotent?: boolean;
  /**
   * Zod schema to validate the request body against before sending (#1341);
   * mirrors the option in public-client.ts.
   */
  bodySchema?: ZodType;
  signal?: AbortSignal;
}

import { ApiError } from './public-client';
import { csrfHeaders, isCsrfTokenError } from './csrf';

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

  if (method === "GET" && options.cacheTtl) {
    const cached = apiCache.get<T>(url);
    if (cached !== null) return cached;
  }

  const maxRetries = options.maxRetries ?? DEFAULT_RETRY_CONFIG.maxRetries;
  const timeoutMs = options.timeoutMs ?? REQUEST_TIMEOUT_MS;

  // Validate the outgoing body before the first attempt (#1341).
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
        headers: { "Content-Type": "application/json", ...csrfHeaders(method) },
        body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
        signal,
      });

      clear();
      reportResponseHeaders(res.headers);

      if (!res.ok) {
        if (res.status === 429) {
          const retryAfter = res.headers.get('Retry-After');
          const retryAfterSec = retryAfter ? parseInt(retryAfter, 10) : NaN;
          reportRateLimited(Number.isNaN(retryAfterSec) ? 1 : retryAfterSec);
          if (attempt < maxRetries) {
            const delayMs = getRetryDelay(attempt, Number.isNaN(retryAfterSec) ? undefined : retryAfterSec);
            await sleep(delayMs);
            continue;
          }
        }

        if (res.status >= 500 && attempt < maxRetries && (method === "GET" || options.idempotent)) {
          await sleep(getRetryDelay(attempt));
          continue;
        }

        let err: unknown;
        try { err = await res.json(); } catch { err = {}; }
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

      const text = await res.text();
      const data = text ? (JSON.parse(text) as T) : (undefined as unknown as T);

      if (method === "GET" && options.cacheTtl) {
        apiCache.set(url, data, options.cacheTtl, options.cacheTags);
      }

      // Guard: some endpoints return { success: false, message: '...' } with a
      // 200 status to signal business-logic failure. Only bust the cache when
      // the mutation actually succeeded — i.e. when the response has no
      // `success` field (non-envelope endpoints) or when it's explicitly `true`.
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

      if (networkErr instanceof DOMException && networkErr.name === 'AbortError') {
        if (!options.signal?.aborted) {
          throw new ApiError('The request timed out. Please try again.', 0, 'TIMEOUT_ERROR');
        }
        throw networkErr;
      }

      lastError = networkErr instanceof Error ? networkErr : new Error(String(networkErr));

      if (attempt < maxRetries && method === "GET") {
        await sleep(getRetryDelay(attempt));
        continue;
      }

      throw new ApiError(`Unable to reach the server. Please check your connection. (${lastError.message})`, 0);
    }
  }

  throw lastError || new ApiError("Request failed after retries", 0);
}

// ---------------------------------------------------------------------------
// Contract error map — admin/internal use only
// ---------------------------------------------------------------------------

/**
 * Maps Soroban contract error codes (u32) to localized user-facing messages.
 *
 * When the API returns a CONTRACT_ERROR, read `details.contract_code` and pass
 * it to `getContractErrorMessage` to get a display-ready string.
 *
 * Source of truth: contracts/predict-iq/src/errors.rs
 * Full reference:  docs/CONTRACT_ERRORS.md
 */
export const CONTRACT_ERROR_MESSAGES: Record<number, string> = {
  // Authorization & Setup
  100: "This contract has already been set up.",
  101: "You are not authorized to perform this action.",
  120: "No admin has been configured for this contract.",
  121: "The platform is currently paused. Please try again later.",
  122: "No guardian has been configured for this contract.",
  146: "The governance token contract has not been configured.",

  // Market Lifecycle
  102: "Market not found.",
  103: "This market is closed and no longer accepts activity.",
  104: "This market is still active and cannot be finalized yet.",
  115: "This market is not currently active.",
  116: "The deadline for this market has passed.",
  148: "The provided deadline is invalid.",

  // Betting
  105: "The selected outcome is not valid for this market.",
  106: "The bet amount is invalid. Please enter a valid amount.",
  107: "Insufficient balance to complete this transaction.",
  126: "Your deposit is below the minimum required amount.",
  142: "Bet not found.",
  145: "The amount provided is invalid.",

  // Resolution & Disputes
  108: "The oracle failed to provide a result. Please try again later.",
  110: "The dispute window for this market has closed.",
  117: "The outcome for this market has already been set.",
  118: "This market is not in a disputed state.",
  119: "This market is not pending resolution.",
  133: "The parent market has not been resolved yet.",
  134: "The parent market outcome does not satisfy this market's condition.",
  135: "Resolution conditions have not been met yet. Please try again later.",
  136: "The dispute window is still open. Resolution must wait.",
  137: "No majority outcome was reached. Resolution is inconclusive.",
  138: "Price data is stale. A fresh oracle feed is required.",
  139: "Oracle confidence is too low to resolve this market.",
  141: "This market was not cancelled.",
  147: "This market has not been resolved yet.",

  // Voting & Governance
  111: "Voting on this market has not started yet.",
  112: "The voting period for this market has ended.",
  113: "You have already voted on this market.",
  114: "The requested fee is too high.",
  129: "Not enough governance votes to approve this action.",
  130: "You have already voted on this upgrade.",
  140: "Your governance token balance is too low to vote.",

  // Upgrades
  127: "A timelock is active. Please wait before retrying.",
  128: "No upgrade has been initiated.",
  131: "The provided WASM hash is invalid.",
  132: "The contract upgrade failed.",
  143: "An upgrade is already pending. Only one upgrade can be in progress at a time.",
  144: "This WASM hash is in cooldown. Please wait before reusing it.",

  // System
  109: "The system circuit breaker is open. Operations are temporarily halted.",
  123: "Too many outcomes provided for this market.",
  124: "Too many winners specified for payout calculation.",
  125: "This payout mode is not supported.",
  152: "The migration data failed validation and cannot proceed.",
  154: "An arithmetic operation overflowed. Please try again.",
  159: "An arithmetic overflow occurred during calculation.",

  // Market Lifecycle
  160: "The provided time range is invalid.",

  // Betting
  155: "This reward has already been claimed.",
  156: "There are no winnings available to claim for this market.",
  157: "The provided referrer address is invalid or not registered.",

  // Resolution & Disputes
  158: "The resolution deadline for this market has passed.",

  // Ownership & Token Operations
  149: "No pending transfer was found for this identifier.",
  150: "You are not the pending owner of this asset.",
  151: "This token account is frozen and cannot be used.",
  153: "This asset has been clawed back by the issuer.",
};

/**
 * Returns a user-facing message for a contract error code.
 * Falls back to a generic message if the code is not recognized.
 */
export function getContractErrorMessage(code: number): string {
  return CONTRACT_ERROR_MESSAGES[code] ?? `An unexpected contract error occurred (code ${code}).`;
}

// ---------------------------------------------------------------------------
// Full api object: public methods + admin-only methods
// ---------------------------------------------------------------------------

export const api = {
  ...publicApi,

  // Admin / email
  resolveMarket: (marketId: number | string, signal?: AbortSignal) =>
    request<components['schemas']['InvalidationResult']>(
      "POST",
      fillPath(PATHS.resolveMarket, 'market_id', marketId),
      { cacheTags: [CacheTag.MARKETS, CacheTag.BLOCKCHAIN, CacheTag.STATISTICS], signal },
    ),

  getAuditLogs: (params: Record<string, string | number | undefined> = {}, signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>("GET", PATHS.auditLogs, {
      params, cacheTtl: CACHE_TTL.SHORT, cacheTags: [CacheTag.AUDIT], signal,
    }),

  emailPreview: (templateName: string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", fillPath("/api/v1/email/preview/{template_name}", 'template_name', templateName), {
      cacheTtl: CACHE_TTL.LONG,
      cacheTags: [CacheTag.EMAIL],
      signal,
    }),

  emailSendTest: (body: { recipient: string; template_name: string }, signal?: AbortSignal) =>
    request<{ success: boolean; message: string; message_id: string }>(
      "POST",
      "/api/v1/email/test",
      { body, bodySchema: emailTestSchema, cacheTags: [CacheTag.EMAIL], signal }
    ),

  getEmailAnalytics: (params?: { template_name?: string; days?: number }, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/v1/email/analytics", {
      params,
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.EMAIL],
      signal,
    }),

  getEmailQueueStats: (signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/v1/email/queue/stats", {
      cacheTtl: CACHE_TTL.SHORT,
      cacheTags: [CacheTag.EMAIL],
      signal,
    }),

  blockchainReplay: (body: { from_ledger: number }, signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>(
      "POST",
      PATHS.blockchainReplay,
      { body, cacheTags: [CacheTag.BLOCKCHAIN], signal }
    ),

  saveContent: (body: Record<string, unknown>, signal?: AbortSignal) =>
    request<components['schemas']['AnyObject']>(
      "POST",
      PATHS.content,
      { body, cacheTags: [CacheTag.STATISTICS, CacheTag.MARKETS], signal }
    ),
};
