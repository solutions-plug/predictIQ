/**
 * #1335 — cache tags are invalidated only when a mutation actually succeeded.
 *
 * Some endpoints return HTTP 200 with `{ success: false, message: '...' }` to signal a
 * business-logic failure (regression trap, commit `4a15eda`). Such a response must leave
 * the existing cache entry untouched. The guard lives in the request helper (that is
 * where the response body is parsed), shared by both the public and admin clients.
 */
import { apiCache, CACHE_TTL } from '../cache';
import { CacheTag } from '../public-client';

const BASE_URL = process.env.NEXT_PUBLIC_API_URL!.replace(/\/$/, '');
const STATS_KEY = `${BASE_URL}/api/v1/statistics`;

function mockJsonResponse(body: unknown) {
  return {
    ok: true,
    status: 200,
    headers: { get: () => null },
    text: async () => JSON.stringify(body),
    json: async () => body,
  };
}

describe('cache invalidation is gated on mutation success (#1335)', () => {
  const originalFetch = global.fetch;

  beforeEach(() => {
    apiCache.clear();
    apiCache.set(STATS_KEY, { totalMarkets: 7 }, CACHE_TTL.MEDIUM, [CacheTag.STATISTICS]);
  });
  afterEach(() => {
    global.fetch = originalFetch;
    apiCache.clear();
  });

  it('a 200 response whose body reports failure does NOT invalidate the tag', async () => {
    global.fetch = jest.fn(async () =>
      mockJsonResponse({ success: false, message: 'Email already subscribed' }),
    ) as unknown as typeof fetch;

    const { api } = await import('../public-client');
    const result = await api.newsletterSubscribe({ email: 'a@b.co' });

    expect(result.success).toBe(false);
    // The statistics cache entry is still present and unchanged.
    expect(apiCache.get<{ totalMarkets: number }>(STATS_KEY)).toEqual({ totalMarkets: 7 });
  });

  it('a 200 response reporting success DOES invalidate the tag', async () => {
    global.fetch = jest.fn(async () =>
      mockJsonResponse({ success: true, message: 'Check your inbox' }),
    ) as unknown as typeof fetch;

    const { api } = await import('../public-client');
    await api.newsletterSubscribe({ email: 'a@b.co' });

    expect(apiCache.get(STATS_KEY)).toBeNull();
  });

  it('a non-envelope 200 body (no `success` field) invalidates as before', async () => {
    global.fetch = jest.fn(async () => mockJsonResponse({ id: 'sub_1' })) as unknown as typeof fetch;

    const { api } = await import('../public-client');
    await api.newsletterSubscribe({ email: 'a@b.co' });

    expect(apiCache.get(STATS_KEY)).toBeNull();
  });
});
