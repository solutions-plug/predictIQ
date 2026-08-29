import { api, ApiError } from '../public-client';
import { apiCache } from '../cache';
import { newIdempotencyKey, isValidIdempotencyKey } from '../idempotency';

/**
 * #1340 — the client generates one idempotency key per logical submission and
 * reuses it across automatic retries, so a failed-then-succeeded submit does
 * not create a duplicate against a backend that deduplicates on the key.
 */

const UUID_RE = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

describe('newIdempotencyKey', () => {
  it('returns a UUID-shaped string', () => {
    expect(newIdempotencyKey()).toMatch(UUID_RE);
  });

  it('returns a distinct value on every call', () => {
    const keys = new Set(Array.from({ length: 100 }, () => newIdempotencyKey()));
    expect(keys.size).toBe(100);
  });

  it('rejects empty or over-long keys', () => {
    expect(isValidIdempotencyKey('')).toBe(false);
    expect(isValidIdempotencyKey('x'.repeat(129))).toBe(false);
    expect(isValidIdempotencyKey(newIdempotencyKey())).toBe(true);
  });
});

describe('idempotency key on mutating requests', () => {
  const originalFetch = global.fetch;
  const originalEnv = process.env.NEXT_PUBLIC_API_URL;

  beforeEach(() => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    apiCache.clear();
  });

  afterEach(() => {
    jest.restoreAllMocks();
    process.env.NEXT_PUBLIC_API_URL = originalEnv;
    global.fetch = originalFetch;
  });

  function keyOf(call: unknown[]): string | undefined {
    const init = call[1] as RequestInit | undefined;
    const headers = (init?.headers ?? {}) as Record<string, string>;
    return headers['Idempotency-Key'];
  }

  it('attaches an Idempotency-Key header to placeBet', async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify({ tx_hash: '0xabc', status: 'pending' }),
    });

    await api.placeBet(7, { wallet: 'GABC', outcome: 1, amount: '10' });

    const call = (global.fetch as jest.Mock).mock.calls[0];
    expect(keyOf(call)).toMatch(UUID_RE);
  });

  it('reuses the same key across an automatic retry of one submission', async () => {
    global.fetch = jest
      .fn()
      // first attempt: rate limited -> the client retries
      .mockResolvedValueOnce({
        ok: false,
        status: 429,
        headers: { get: (h: string) => (h === 'Retry-After' ? '0' : null) },
        json: async () => ({ message: 'slow down' }),
      })
      // retry: succeeds
      .mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify({ tx_hash: '0xabc', status: 'pending' }),
      });

    await api.placeBet(7, { wallet: 'GABC', outcome: 1, amount: '10' });

    const calls = (global.fetch as jest.Mock).mock.calls;
    expect(calls).toHaveLength(2);
    expect(keyOf(calls[0])).toBeDefined();
    expect(keyOf(calls[1])).toBe(keyOf(calls[0]));
  }, 10000);

  it('generates a fresh key for each new user-initiated submission', async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify({ tx_hash: '0xabc', status: 'pending' }),
    });

    await api.placeBet(7, { wallet: 'GABC', outcome: 1, amount: '10' });
    await api.placeBet(7, { wallet: 'GABC', outcome: 2, amount: '25' });

    const calls = (global.fetch as jest.Mock).mock.calls;
    expect(keyOf(calls[0])).not.toBe(keyOf(calls[1]));
  });

  it('honours a caller-supplied key (double-click guard use case)', async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: true,
      text: async () => JSON.stringify({ tx_hash: '0xabc', status: 'pending' }),
    });

    const key = newIdempotencyKey();
    await api.placeBet(7, { wallet: 'GABC', outcome: 1, amount: '10' }, { idempotencyKey: key });
    await api.placeBet(7, { wallet: 'GABC', outcome: 1, amount: '10' }, { idempotencyKey: key });

    const calls = (global.fetch as jest.Mock).mock.calls;
    expect(keyOf(calls[0])).toBe(key);
    expect(keyOf(calls[1])).toBe(key);
  });

  it('does not create a duplicate when a failed-then-succeeded submit is retried (deduping backend)', async () => {
    // A backend that has already committed the bet on the (lost) first response
    // and now deduplicates on the key: a second request with the same key
    // returns the original result instead of placing a second bet.
    const committed: Array<{ key: string }> = [];

    global.fetch = jest.fn().mockImplementation((_url: string, init: RequestInit) => {
      const headers = (init.headers ?? {}) as Record<string, string>;
      const key = headers['Idempotency-Key'];
      const already = committed.find((c) => c.key === key);
      if (already) {
        return Promise.resolve({
          ok: true,
          text: async () => JSON.stringify({ tx_hash: '0xabc', status: 'pending', deduped: true }),
        });
      }
      committed.push({ key });
      // The write lands, but the response is lost -> the client sees a 429 and retries.
      return Promise.resolve({
        ok: false,
        status: 429,
        headers: { get: (h: string) => (h === 'Retry-After' ? '0' : null) },
        json: async () => ({ message: 'try again' }),
      });
    });

    const result = await api.placeBet(7, { wallet: 'GABC', outcome: 1, amount: '10' });

    expect(result).toMatchObject({ tx_hash: '0xabc' });
    expect(committed).toHaveLength(1);
  }, 10000);

  it('surfaces a normal ApiError if every retry is exhausted', async () => {
    global.fetch = jest.fn().mockResolvedValue({
      ok: false,
      status: 429,
      headers: { get: (h: string) => (h === 'Retry-After' ? '0' : null) },
      json: async () => ({ message: 'rate limited' }),
    });

    await expect(
      api.placeBet(7, { wallet: 'GABC', outcome: 1, amount: '10' }),
    ).rejects.toBeInstanceOf(ApiError);
  }, 10000);
});
