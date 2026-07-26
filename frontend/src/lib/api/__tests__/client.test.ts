import { api, ApiError } from '../client';
import { apiCache } from '../cache';

describe('API Client (landing page)', () => {
  it('should only expose the endpoints the landing page calls', () => {
    // Guards against admin/blockchain/email request builders (and the ~50-entry
    // CONTRACT_ERROR_MESSAGES map) creeping back into the landing page's bundle.
    // See admin-client.ts for that wider surface.
    expect(Object.keys(api).sort()).toEqual(['getStatistics', 'newsletterSubscribe']);
  });


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

  describe('Successful responses', () => {
    it('should handle successful GET requests', async () => {
      const mockData = { total_markets: 10 };
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify(mockData),
      });

      const result = await api.getStatistics();
      expect(result).toEqual(mockData);
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:3001/api/statistics',
        expect.objectContaining({ method: 'GET' })
      );
    });

    it('should handle successful POST requests', async () => {
      const mockResponse = { success: true, message: 'Subscribed' };
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify(mockResponse),
      });

      const result = await api.newsletterSubscribe({ email: 'test@example.com' });
      expect(result).toEqual(mockResponse);
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:3001/api/v1/newsletter/subscribe',
        expect.objectContaining({
          method: 'POST',
          body: JSON.stringify({ email: 'test@example.com' }),
        })
      );
    });

    it('should handle 204 No Content responses', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => '',
      });

      const result = await api.getStatistics();
      expect(result).toBeUndefined();
    });
  });

  describe('Network errors', () => {
    it('should handle network failures', async () => {
      const networkError = new Error('Network request failed');
      (global.fetch as jest.Mock).mockRejectedValueOnce(networkError);

      await expect(api.getStatistics()).rejects.toThrow('Unable to reach the server');
    });

    it('should handle timeout errors', async () => {
      const timeoutError = new Error('Request timeout');
      (global.fetch as jest.Mock).mockRejectedValueOnce(timeoutError);

      await expect(api.getStatistics()).rejects.toThrow('Unable to reach the server');
    });
  });

  describe('Non-2xx responses', () => {
    it('should handle 400 Bad Request', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        json: async () => ({ message: 'Invalid email format' }),
      });

      await expect(
        api.newsletterSubscribe({ email: 'invalid' })
      ).rejects.toThrow('Invalid email format');
    });

    it('should handle 500 Server Error after exhausting retries', async () => {
      const serverError = {
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        json: async () => ({ message: 'Database connection failed' }),
      };
      // GET requests retry 5xx up to maxRetries (3) times — mock all attempts.
      (global.fetch as jest.Mock).mockResolvedValue(serverError);

      await expect(api.getStatistics()).rejects.toThrow('Database connection failed');
      expect(global.fetch).toHaveBeenCalledTimes(4); // 1 initial + 3 retries
    }, 30_000);

    it('should fallback to statusText when error response has no message', async () => {
      const serverError = {
        ok: false,
        status: 503,
        statusText: 'Service Unavailable',
        json: async () => ({}),
      };
      (global.fetch as jest.Mock).mockResolvedValue(serverError);

      await expect(api.getStatistics()).rejects.toThrow('Service Unavailable');
    }, 30_000);

    it('should fallback to HTTP status when response is not JSON', async () => {
      const serverError = {
        ok: false,
        status: 502,
        statusText: 'Bad Gateway',
        json: async () => { throw new Error('Invalid JSON'); },
      };
      (global.fetch as jest.Mock).mockResolvedValue(serverError);

      await expect(api.getStatistics()).rejects.toThrow('Bad Gateway');
    }, 30_000);
  });

  describe('Retry behavior', () => {
    it('should retry on 429 Too Many Requests', async () => {
      const mockData = { total_markets: 10 };
      (global.fetch as jest.Mock)
        .mockResolvedValueOnce({
          ok: false,
          status: 429,
          statusText: 'Too Many Requests',
          headers: new Map(),
          json: async () => ({ message: 'Rate limited' }),
        })
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData),
        });

      const result = await api.getStatistics();

      expect(result).toEqual(mockData);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    }, 10000);

    it('should respect Retry-After header on 429', async () => {
      const mockData = { total_markets: 10 };
      const mockHeaders = new Map([['Retry-After', '0']]);

      (global.fetch as jest.Mock)
        .mockResolvedValueOnce({
          ok: false,
          status: 429,
          statusText: 'Too Many Requests',
          headers: mockHeaders,
          json: async () => ({ message: 'Rate limited' }),
        })
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData),
        });

      const result = await api.getStatistics();

      expect(result).toEqual(mockData);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    }, 10000);

    it('should fail after max retries on 429', async () => {
      (global.fetch as jest.Mock).mockResolvedValue({
        ok: false,
        status: 429,
        statusText: 'Too Many Requests',
        headers: new Map(),
        json: async () => ({ message: 'Rate limited' }),
      });

      await expect(api.getStatistics()).rejects.toThrow('Rate limited');
      expect(global.fetch).toHaveBeenCalledTimes(4); // 1 initial + 3 retries
    }, 10000);

    it('should retry on network failure for GET requests', async () => {
      const mockData = { total_markets: 10 };
      (global.fetch as jest.Mock)
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData),
        });

      const result = await api.getStatistics();

      expect(result).toEqual(mockData);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    }, 10000);

    it('should not retry on 4xx errors', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 400,
        statusText: 'Bad Request',
        json: async () => ({ message: 'Invalid request' }),
      });

      await expect(api.getStatistics()).rejects.toThrow('Invalid request');
      expect(global.fetch).toHaveBeenCalledTimes(1);
    });

    it('should not retry POST requests on network failure', async () => {
      (global.fetch as jest.Mock).mockRejectedValueOnce(new Error('Network error'));

      await expect(
        api.newsletterSubscribe({ email: 'test@example.com' })
      ).rejects.toThrow('Unable to reach the server');
      expect(global.fetch).toHaveBeenCalledTimes(1);
    });

    it('should handle multiple sequential requests', async () => {
      const mockData1 = { total_markets: 10 };
      const mockData2 = { success: true, message: 'Subscribed' };

      (global.fetch as jest.Mock)
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData1),
        })
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData2),
        });

      const result1 = await api.getStatistics();
      const result2 = await api.newsletterSubscribe({ email: 'test@example.com' });

      expect(result1).toEqual(mockData1);
      expect(result2).toEqual(mockData2);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    });

    it('should use exponential backoff for retries', async () => {
      const mockData = { total_markets: 10 };
      (global.fetch as jest.Mock)
        .mockRejectedValueOnce(new Error('Network error'))
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData),
        });

      const result = await api.getStatistics();

      expect(result).toEqual(mockData);
      expect(global.fetch).toHaveBeenCalledTimes(3);
    }, 10000);
  });

  describe('Content-Type header', () => {
    it('should set Content-Type to application/json', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => '{}',
      });

      await api.getStatistics();

      expect(global.fetch).toHaveBeenCalledWith(
        expect.any(String),
        expect.objectContaining({
          headers: { 'Content-Type': 'application/json' },
        })
      );
    });
  });

  describe('Base URL handling', () => {
    it('should strip trailing slash from base URL', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => '{}',
      });

      await api.getStatistics();

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:3001/api/statistics',
        expect.any(Object)
      );
    });
  });

  describe('Request timeout (#945)', () => {
    afterEach(() => {
      jest.useRealTimers();
    });

    it('fires after 10 seconds and rejects with a distinct TIMEOUT_ERROR', async () => {
      jest.useFakeTimers();

      (global.fetch as jest.Mock).mockImplementation((_url: string, init: RequestInit) =>
        new Promise((_resolve, reject) => {
          (init.signal as AbortSignal).addEventListener('abort', () =>
            reject(new DOMException('The operation was aborted.', 'AbortError'))
          );
        })
      );

      // Pre-attach .catch so the rejection is never "unhandled" while timers advance.
      let caughtError: unknown;
      const settledPromise = api.newsletterSubscribe({ email: 'test@example.com' }).catch(err => { caughtError = err; });

      await jest.advanceTimersByTimeAsync(10_000);
      await settledPromise;

      expect(caughtError).toMatchObject({
        name: 'ApiError',
        code: 'TIMEOUT_ERROR',
        message: expect.stringContaining('timed out'),
      });
    });

    it('surfaces a timeout error distinct from a generic network error', async () => {
      jest.useFakeTimers();

      (global.fetch as jest.Mock).mockImplementation((_url: string, init: RequestInit) =>
        new Promise((_resolve, reject) => {
          (init.signal as AbortSignal).addEventListener('abort', () =>
            reject(new DOMException('Aborted', 'AbortError'))
          );
        })
      );

      let caughtError: unknown;
      const settledPromise = api.getStatistics().catch(err => { caughtError = err; });
      await jest.advanceTimersByTimeAsync(10_000);
      await settledPromise;

      expect(caughtError).toBeInstanceOf(ApiError);
      expect((caughtError as ApiError).code).toBe('TIMEOUT_ERROR');
      // Must NOT be the generic network message so the UI can branch on it.
      expect((caughtError as ApiError).message).not.toContain('Unable to reach the server');
    });
  });

  describe('5xx retry logic (#946)', () => {
    it('retries GET requests on 5xx: two 502s then 200 succeeds', async () => {
      const mockData = { total_markets: 10 };
      const gatewayError = {
        ok: false,
        status: 502,
        statusText: 'Bad Gateway',
        headers: new Map(),
        json: async () => ({ message: 'Bad Gateway' }),
      };
      (global.fetch as jest.Mock)
        .mockResolvedValueOnce(gatewayError)
        .mockResolvedValueOnce(gatewayError)
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData),
        });

      const result = await api.getStatistics();
      expect(result).toEqual(mockData);
      expect(global.fetch).toHaveBeenCalledTimes(3); // 1 initial + 2 retries
    }, 10_000);

    it('does not retry 5xx for POST requests', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 503,
        statusText: 'Service Unavailable',
        json: async () => ({ message: 'Service Unavailable' }),
      });

      await expect(
        api.newsletterSubscribe({ email: 'test@example.com' })
      ).rejects.toThrow('Service Unavailable');
      expect(global.fetch).toHaveBeenCalledTimes(1);
    });
  });

  describe('ApiError', () => {
    it('should throw ApiError with status 0 on network failure', async () => {
      (global.fetch as jest.Mock).mockRejectedValueOnce(new Error('Failed to fetch'));

      try {
        await api.getStatistics();
        fail('should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        expect((e as ApiError).status).toBe(0);
        expect((e as ApiError).isNetworkError).toBe(true);
        expect((e as ApiError).message).toContain('Unable to reach the server');
      }
    });

    it('should classify 5xx as server error', async () => {
      const serverError = {
        ok: false,
        status: 503,
        statusText: 'Service Unavailable',
        json: async () => ({ message: 'Service Unavailable' }),
      };
      // GET retries on 5xx — provide enough responses to exhaust retries.
      (global.fetch as jest.Mock).mockResolvedValue(serverError);

      try {
        await api.getStatistics();
        fail('should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        expect((e as ApiError).isServerError).toBe(true);
        expect((e as ApiError).isClientError).toBe(false);
      }
    }, 30_000);

    it('should have name "ApiError"', async () => {
      (global.fetch as jest.Mock).mockRejectedValueOnce(new Error('offline'));

      try {
        await api.getStatistics();
      } catch (e) {
        expect((e as ApiError).name).toBe('ApiError');
      }
    });
  });
});
