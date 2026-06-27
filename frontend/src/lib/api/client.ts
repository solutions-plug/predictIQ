/**
 * Type-safe API client generated from the OpenAPI schema.
 * Run `npm run generate-client` to regenerate `schema.d.ts` after API changes.
 */

import { getEnvConfig } from '../env';
import { apiCache, CACHE_TTL } from './cache';

const config = getEnvConfig();
const BASE_URL = config.NEXT_PUBLIC_API_URL.replace(/\/$/, "");

type HttpMethod = "GET" | "POST" | "DELETE";

const DEFAULT_RETRY_CONFIG: RetryConfig = {
  maxRetries: 3,
  initialDelayMs: 100,
  maxDelayMs: 10000,
};

function getRetryDelay(attempt: number, retryAfter?: number): number {
  if (retryAfter) return retryAfter * 1000;
  const delay = DEFAULT_RETRY_CONFIG.initialDelayMs * Math.pow(2, attempt);
  return Math.min(delay, DEFAULT_RETRY_CONFIG.maxDelayMs);
}

function sleep(ms: number): Promise<void> {
  return new Promise(resolve => setTimeout(resolve, ms));
}

interface RequestOptions {
  body?: unknown;
  params?: Record<string, string | number | undefined>;
  cacheTtl?: number;
  maxRetries?: number;
  signal?: AbortSignal;
}

interface RetryConfig {
  maxRetries: number;
  initialDelayMs: number;
  maxDelayMs: number;
}

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
};

/**
 * Returns a user-facing message for a contract error code.
 * Falls back to a generic message if the code is not recognized.
 */
export function getContractErrorMessage(code: number): string {
  return CONTRACT_ERROR_MESSAGES[code] ?? `An unexpected contract error occurred (code ${code}).`;
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
  let lastError: Error | null = null;

  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      const res = await fetch(url, {
        method,
        headers: { "Content-Type": "application/json" },
        body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
        signal: options.signal,
      });

      if (!res.ok) {
        if (res.status === 429) {
          if (attempt < maxRetries) {
            const retryAfter = res.headers.get('Retry-After');
            const delayMs = getRetryDelay(attempt, retryAfter ? parseInt(retryAfter, 10) : undefined);
            await sleep(delayMs);
            continue;
          }
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

      // Cache GET responses
      if (method === "GET" && options.cacheTtl) {
        apiCache.set(url, data, options.cacheTtl);
      }

      // Invalidate cache on mutations
      if (method === "POST" || method === "DELETE") {
        apiCache.invalidateByPattern('.*');
      }

      return data;
    } catch (networkErr) {
      if (networkErr instanceof ApiError) {
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
// Public endpoints
// ---------------------------------------------------------------------------

export const api = {
  health: (signal?: AbortSignal) => request<string>("GET", "/health", { signal }),

  getStatistics: (signal?: AbortSignal) => 
    request<Record<string, unknown>>("GET", "/api/statistics", { cacheTtl: CACHE_TTL.MEDIUM, signal }),

  getFeaturedMarkets: (signal?: AbortSignal) =>
    request<
      Array<{
        id: number;
        title: string;
        volume: number;
        ends_at: string;
        onchain_volume: string;
        resolved_outcome?: number | null;
      }>
    >("GET", "/api/markets/featured", { cacheTtl: CACHE_TTL.SHORT, signal }),

  getContent: (params?: { page?: number; page_size?: number }, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/content", { params, cacheTtl: CACHE_TTL.MEDIUM, signal }),

  // Blockchain
  getBlockchainHealth: (signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/blockchain/health", { cacheTtl: CACHE_TTL.SHORT, signal }),

  getBlockchainMarket: (marketId: number | string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/markets/${marketId}`, { cacheTtl: CACHE_TTL.MEDIUM, signal }),

  getBlockchainStats: (signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/blockchain/stats", { cacheTtl: CACHE_TTL.MEDIUM, signal }),

  getUserBets: (user: string, params?: { page?: number; page_size?: number }, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/users/${user}/bets`, { params, cacheTtl: CACHE_TTL.MEDIUM, signal }),

  getOracleResult: (marketId: number | string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/oracle/${marketId}`, { cacheTtl: CACHE_TTL.LONG, signal }),

  getTransactionStatus: (txHash: string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/tx/${txHash}`, { cacheTtl: CACHE_TTL.LONG, signal }),

  // Newsletter
  newsletterSubscribe: (body: { email: string; source?: string }, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("POST", "/api/v1/newsletter/subscribe", { body, signal }),

  newsletterConfirm: (token: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("GET", `/api/v1/newsletter/confirm`, {
      params: { token },
      signal,
    }),

  newsletterUnsubscribe: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/unsubscribe", {
      body: { email },
      signal,
    }),

  newsletterGdprExport: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; data: Record<string, unknown> }>(
      "GET",
      "/api/v1/newsletter/gdpr/export",
      { params: { email }, signal }
    ),

  newsletterGdprDelete: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/gdpr/delete", {
      body: { email },
      signal,
    }),

  // Admin / email
  resolveMarket: (marketId: number | string, signal?: AbortSignal) =>
    request<{ invalidated_keys: number }>("POST", `/api/markets/${marketId}/resolve`, { signal }),

  emailPreview: (templateName: string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/v1/email/preview/${templateName}`, { cacheTtl: CACHE_TTL.LONG, signal }),

  emailSendTest: (body: { recipient: string; template_name: string }, signal?: AbortSignal) =>
    request<{ success: boolean; message: string; message_id: string }>(
      "POST",
      "/api/v1/email/test",
      { body, signal }
    ),

  getEmailAnalytics: (params?: { template_name?: string; days?: number }, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/v1/email/analytics", { params, cacheTtl: CACHE_TTL.MEDIUM, signal }),

  getEmailQueueStats: (signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/v1/email/queue/stats", { cacheTtl: CACHE_TTL.SHORT, signal }),
};
