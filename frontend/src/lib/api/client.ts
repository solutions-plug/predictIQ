/**
 * Type-safe API client generated from the OpenAPI schema.
 * Run `npm run generate-client` to regenerate `schema.d.ts` after API changes.
 */

import { getEnvConfig } from './env';
import { apiCache, CACHE_TTL } from './cache';

const config = getEnvConfig();
const BASE_URL = config.apiUrl.replace(/\/$/, "");

type HttpMethod = "GET" | "POST" | "DELETE";

interface RequestOptions {
  body?: unknown;
  params?: Record<string, string | number | undefined>;
  cacheTtl?: number;
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

  let res: Response;
  try {
    res = await fetch(url, {
      method,
      headers: { "Content-Type": "application/json" },
      body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
    });
  } catch (networkErr) {
    // Network failure (offline, DNS error, CORS, timeout, etc.)
    const msg = networkErr instanceof Error ? networkErr.message : "Network request failed";
    throw new ApiError(`Unable to reach the server. Please check your connection. (${msg})`, 0);
  }

  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }));
    const message = err?.message ?? `HTTP ${res.status}`;
    const code = err?.code ?? "UNKNOWN_ERROR";
    const details = err?.details ?? undefined;
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
}

// ---------------------------------------------------------------------------
// Public endpoints
// ---------------------------------------------------------------------------

export const api = {
  health: () => request<string>("GET", "/health"),

  getStatistics: () => 
    request<Record<string, unknown>>("GET", "/api/statistics", { cacheTtl: CACHE_TTL.MEDIUM }),

  getFeaturedMarkets: () =>
    request<
      Array<{
        id: number;
        title: string;
        volume: number;
        ends_at: string;
        onchain_volume: string;
        resolved_outcome?: number | null;
      }>
    >("GET", "/api/markets/featured", { cacheTtl: CACHE_TTL.SHORT }),

  getContent: (params?: { page?: number; page_size?: number }) =>
    request<Record<string, unknown>>("GET", "/api/content", { params, cacheTtl: CACHE_TTL.MEDIUM }),

  // Blockchain
  getBlockchainHealth: () =>
    request<Record<string, unknown>>("GET", "/api/blockchain/health", { cacheTtl: CACHE_TTL.SHORT }),

  getBlockchainMarket: (marketId: number | string) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/markets/${marketId}`, { cacheTtl: CACHE_TTL.MEDIUM }),

  getBlockchainStats: () =>
    request<Record<string, unknown>>("GET", "/api/blockchain/stats", { cacheTtl: CACHE_TTL.MEDIUM }),

  getUserBets: (user: string, params?: { page?: number; page_size?: number }) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/users/${user}/bets`, { params, cacheTtl: CACHE_TTL.MEDIUM }),

  getOracleResult: (marketId: number | string) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/oracle/${marketId}`, { cacheTtl: CACHE_TTL.LONG }),

  getTransactionStatus: (txHash: string) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/tx/${txHash}`, { cacheTtl: CACHE_TTL.LONG }),

  // Newsletter
  newsletterSubscribe: (body: { email: string; source?: string }) =>
    request<{ success: boolean; message: string }>("POST", "/api/v1/newsletter/subscribe", { body }),

  newsletterConfirm: (token: string) =>
    request<{ success: boolean; message: string }>("GET", `/api/v1/newsletter/confirm`, {
      params: { token },
    }),

  newsletterUnsubscribe: (email: string) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/unsubscribe", {
      body: { email },
    }),

  newsletterGdprExport: (email: string) =>
    request<{ success: boolean; data: Record<string, unknown> }>(
      "GET",
      "/api/v1/newsletter/gdpr/export",
      { params: { email } }
    ),

  newsletterGdprDelete: (email: string) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/gdpr/delete", {
      body: { email },
    }),

  // Admin / email
  resolveMarket: (marketId: number | string) =>
    request<{ invalidated_keys: number }>("POST", `/api/markets/${marketId}/resolve`),

  emailPreview: (templateName: string) =>
    request<Record<string, unknown>>("GET", `/api/v1/email/preview/${templateName}`, { cacheTtl: CACHE_TTL.LONG }),

  emailSendTest: (body: { recipient: string; template_name: string }) =>
    request<{ success: boolean; message: string; message_id: string }>(
      "POST",
      "/api/v1/email/test",
      { body }
    ),

  getEmailAnalytics: (params?: { template_name?: string; days?: number }) =>
    request<Record<string, unknown>>("GET", "/api/v1/email/analytics", { params, cacheTtl: CACHE_TTL.MEDIUM }),

  getEmailQueueStats: () =>
    request<Record<string, unknown>>("GET", "/api/v1/email/queue/stats", { cacheTtl: CACHE_TTL.SHORT }),
};
