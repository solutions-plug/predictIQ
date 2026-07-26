import { api, ApiError, getContractErrorMessage } from '../admin-client';
import { api as landingApi } from '../client';
import { apiCache } from '../cache';

describe('API Client (admin/internal)', () => {
  const originalFetch = global.fetch;
  const originalEnv = process.env.NEXT_PUBLIC_API_URL;

  beforeEach(() => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    global.fetch = jest.fn();
    // Clear the module-level cache singleton so each test starts clean.
    apiCache.clear();
  });

  afterEach(() => {
    jest.restoreAllMocks();
    process.env.NEXT_PUBLIC_API_URL = originalEnv;
    global.fetch = originalFetch;
  });

  describe('Endpoint wrappers', () => {
    const mockOk = (data: unknown = { ok: true }) =>
      (global.fetch as jest.Mock).mockResolvedValue({
        ok: true,
        text: async () => JSON.stringify(data),
      });

    const base = 'http://localhost:3001';

    it.each([
      ['health', () => api.health(), 'GET', '/health'],
      ['getFeaturedMarkets', () => api.getFeaturedMarkets(), 'GET', '/api/markets/featured'],
      ['getBlockchainHealth', () => api.getBlockchainHealth(), 'GET', '/api/blockchain/health'],
      ['getBlockchainMarket', () => api.getBlockchainMarket(1), 'GET', '/api/blockchain/markets/1'],
      ['getBlockchainStats', () => api.getBlockchainStats(), 'GET', '/api/blockchain/stats'],
      ['getUserBets', () => api.getUserBets('GABC'), 'GET', '/api/blockchain/users/GABC/bets'],
      ['getOracleResult', () => api.getOracleResult(7), 'GET', '/api/blockchain/oracle/7'],
      ['getTransactionStatus', () => api.getTransactionStatus('0xdead'), 'GET', '/api/blockchain/tx/0xdead'],
      ['newsletterConfirm', () => api.newsletterConfirm('tok'), 'GET', '/api/v1/newsletter/confirm'],
      ['newsletterUnsubscribe', () => api.newsletterUnsubscribe('a@b.com'), 'DELETE', '/api/v1/newsletter/unsubscribe'],
      ['newsletterGdprExport', () => api.newsletterGdprExport('a@b.com'), 'GET', '/api/v1/newsletter/gdpr/export'],
      ['newsletterGdprDelete', () => api.newsletterGdprDelete('a@b.com'), 'DELETE', '/api/v1/newsletter/gdpr/delete'],
      ['resolveMarket', () => api.resolveMarket(3), 'POST', '/api/markets/3/resolve'],
      ['emailPreview', () => api.emailPreview('welcome'), 'GET', '/api/v1/email/preview/welcome'],
      ['emailSendTest', () => api.emailSendTest({ recipient: 'a@b.com', template_name: 'welcome' }), 'POST', '/api/v1/email/test'],
      ['getEmailAnalytics', () => api.getEmailAnalytics({ days: 7 }), 'GET', '/api/v1/email/analytics'],
      ['getEmailQueueStats', () => api.getEmailQueueStats(), 'GET', '/api/v1/email/queue/stats'],
    ])('%s calls the correct method and path', async (_name, call, method, path) => {
      mockOk();
      await call();
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining(`${base}${path}`),
        expect.objectContaining({ method }),
      );
    });
  });

  describe('Query parameters', () => {
    it('should handle query parameters', async () => {
      const mockData = { markets: [] };
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify(mockData),
      });

      await api.getContent({ page: 1, page_size: 10 });
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('page=1&page_size=10'),
        expect.any(Object)
      );
    });

    it('should filter out undefined query parameters', async () => {
      const mockData = { markets: [] };
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify(mockData),
      });

      await api.getContent({ page: 1 });
      expect(global.fetch).toHaveBeenCalledWith(
        expect.stringContaining('page=1'),
        expect.any(Object)
      );
      expect(global.fetch).toHaveBeenCalledWith(
        expect.not.stringContaining('page_size'),
        expect.any(Object)
      );
    });
  });

  describe('Non-2xx responses', () => {
    it('should handle 401 Unauthorized', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 401,
        statusText: 'Unauthorized',
        json: async () => ({ message: 'Authentication required' }),
      });

      await expect(api.getBlockchainHealth()).rejects.toThrow('Authentication required');
    });

    it('should handle 404 Not Found', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        json: async () => ({ message: 'Market not found' }),
      });

      await expect(api.getBlockchainMarket(999)).rejects.toThrow('Market not found');
    });
  });

  describe('5xx retry logic (#946)', () => {
    it('does not retry 5xx for DELETE requests', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 502,
        statusText: 'Bad Gateway',
        json: async () => ({ message: 'Bad Gateway' }),
      });

      await expect(
        api.newsletterUnsubscribe('test@example.com')
      ).rejects.toThrow('Bad Gateway');
      expect(global.fetch).toHaveBeenCalledTimes(1);
    });
  });

  describe('DELETE requests', () => {
    it('should handle DELETE requests with body', async () => {
      const mockResponse = { success: true };
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify(mockResponse),
      });

      const result = await api.newsletterUnsubscribe('test@example.com');
      expect(result).toEqual(mockResponse);
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:3001/api/v1/newsletter/unsubscribe',
        expect.objectContaining({
          method: 'DELETE',
          body: JSON.stringify({ email: 'test@example.com' }),
        })
      );
    });
  });

  describe('ApiError', () => {
    it('should throw ApiError with status code on non-2xx response', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 404,
        statusText: 'Not Found',
        json: async () => ({ message: 'Market not found' }),
      });

      try {
        await api.getBlockchainMarket(999);
        fail('should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        expect((e as ApiError).status).toBe(404);
        expect((e as ApiError).message).toBe('Market not found');
        expect((e as ApiError).isClientError).toBe(true);
        expect((e as ApiError).isServerError).toBe(false);
        expect((e as ApiError).isNetworkError).toBe(false);
      }
    });
  });

  describe('getContractErrorMessage', () => {
    it('returns the mapped message for a known contract error code', () => {
      expect(getContractErrorMessage(102)).toBe('Market not found.');
    });

    it('falls back to a generic message for an unrecognized code', () => {
      expect(getContractErrorMessage(999999)).toBe('An unexpected contract error occurred (code 999999).');
    });
  });

  describe('Cache invalidation strategy (#947)', () => {
    it('invalidates only tagged resources on mutation — not the entire cache', async () => {
      // Prime a statistics cache entry tagged 'statistics' via the landing client.
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify({ total_markets: 10 }),
      });
      await landingApi.getStatistics();

      // Prime a markets cache entry tagged 'markets'.
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify([{ id: 1 }]),
      });
      await api.getFeaturedMarkets();

      // Mutate: resolveMarket invalidates 'markets', 'blockchain', 'statistics'.
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify({ invalidated_keys: 2 }),
      });
      await api.resolveMarket(1);

      // Both statistics and markets caches must be cleared, even though
      // getStatistics lives in a different module (client.ts) than the
      // mutation (admin-client.ts) — they share the same apiCache tags.
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify({ total_markets: 11 }),
      });
      const freshStats = await landingApi.getStatistics();
      expect(freshStats).toEqual({ total_markets: 11 });
    });

    it('GET returns fresh data after a mutation invalidates its cache tag', async () => {
      // Cache getFeaturedMarkets.
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify([{ id: 1, title: 'Old Market' }]),
      });
      const first = await api.getFeaturedMarkets();
      expect(first).toEqual([{ id: 1, title: 'Old Market' }]);

      // resolveMarket invalidates the 'markets' tag.
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify({ invalidated_keys: 1 }),
      });
      await api.resolveMarket(1);

      // Next getFeaturedMarkets must hit the network, not the cache.
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify([{ id: 1, title: 'Resolved Market' }]),
      });
      const second = await api.getFeaturedMarkets();
      expect(second).toEqual([{ id: 1, title: 'Resolved Market' }]);
    });
  });
});
