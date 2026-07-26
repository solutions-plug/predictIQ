/**
 * API client for the landing page. Only wraps the endpoints the landing page
 * actually calls (getStatistics, newsletterSubscribe) so its bundle doesn't
 * ship the wider admin/blockchain/email surface — see `admin-client.ts` for
 * that. Both share the low-level request/retry/cache logic in `request.ts`.
 *
 * Run `npm run generate-client` to regenerate `schema.d.ts` after API changes.
 */

import { request, ApiError, CacheTag } from './request';
import { CACHE_TTL } from './cache';

export { ApiError };

export const api = {
  getStatistics: (signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/statistics", {
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.STATISTICS],
      signal,
    }),

  newsletterSubscribe: (body: { email: string; source?: string }, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("POST", "/api/v1/newsletter/subscribe", {
      body,
      cacheTags: [CacheTag.NEWSLETTER, CacheTag.STATISTICS],
      signal,
    }),
};
