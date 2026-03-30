/**
 * Type-safe API client generated from the OpenAPI schema.
 * Run `npm run generate-client` to regenerate `schema.d.ts` after API changes.
 */

const BASE_URL =
  process.env.NEXT_PUBLIC_API_URL?.replace(/\/$/, "") ?? "http://localhost:3001";

type HttpMethod = "GET" | "POST" | "DELETE";

async function request<T>(
  method: HttpMethod,
  path: string,
  options: { body?: unknown; params?: Record<string, string | number | undefined> } = {}
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

  const res = await fetch(url, {
    method,
    headers: { "Content-Type": "application/json" },
    body: options.body !== undefined ? JSON.stringify(options.body) : undefined,
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }));
    throw new Error(err?.message ?? `HTTP ${res.status}`);
  }

  // 204 / empty body
  const text = await res.text();
  return text ? (JSON.parse(text) as T) : (undefined as unknown as T);
}

// ---------------------------------------------------------------------------
// Public endpoints
// ---------------------------------------------------------------------------

export const api = {
  health: () => request<string>("GET", "/health"),

  getStatistics: () => request<Record<string, unknown>>("GET", "/api/statistics"),

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
    >("GET", "/api/markets/featured"),

  getContent: (params?: { page?: number; page_size?: number }) =>
    request<Record<string, unknown>>("GET", "/api/content", { params }),

  // Blockchain
  getBlockchainHealth: () =>
    request<Record<string, unknown>>("GET", "/api/blockchain/health"),

  getBlockchainMarket: (marketId: number | string) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/markets/${marketId}`),

  getBlockchainStats: () =>
    request<Record<string, unknown>>("GET", "/api/blockchain/stats"),

  getUserBets: (user: string, params?: { page?: number; page_size?: number }) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/users/${user}/bets`, { params }),

  getOracleResult: (marketId: number | string) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/oracle/${marketId}`),

  getTransactionStatus: (txHash: string) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/tx/${txHash}`),

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
    request<Record<string, unknown>>("GET", `/api/v1/email/preview/${templateName}`),

  emailSendTest: (body: { recipient: string; template_name: string }) =>
    request<{ success: boolean; message: string; message_id: string }>(
      "POST",
      "/api/v1/email/test",
      { body }
    ),

  getEmailAnalytics: (params?: { template_name?: string; days?: number }) =>
    request<Record<string, unknown>>("GET", "/api/v1/email/analytics", { params }),

  getEmailQueueStats: () =>
    request<Record<string, unknown>>("GET", "/api/v1/email/queue/stats"),
};
