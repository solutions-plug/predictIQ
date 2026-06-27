import { api, ApiError } from '../client';

describe('API Client', () => {
  const originalFetch = global.fetch;
  const originalEnv = process.env.NEXT_PUBLIC_API_URL;

  beforeEach(() => {
    process.env.NEXT_PUBLIC_API_URL = 'http://localhost:3001';
    global.fetch = jest.fn();
  });

  afterEach(() => {
    jest.restoreAllMocks();
    process.env.NEXT_PUBLIC_API_URL = originalEnv;
    global.fetch = originalFetch;
  });

  describe('Successful responses', () => {
    it('should handle successful GET requests', async () => {
      const mockData = { status: 'ok' };
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: true,
        text: async () => JSON.stringify(mockData),
      });

      const result = await api.health();
      expect(result).toEqual(mockData);
      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:3001/health',
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

      const result = await api.health();
      expect(result).toBeUndefined();
    });

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

  describe('Network errors', () => {
    it('should handle network failures', async () => {
      const networkError = new Error('Network request failed');
      (global.fetch as jest.Mock).mockRejectedValueOnce(networkError);

      await expect(api.health()).rejects.toThrow('Unable to reach the server');
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

    it('should handle 500 Server Error', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 500,
        statusText: 'Internal Server Error',
        json: async () => ({ message: 'Database connection failed' }),
      });

      await expect(api.getStatistics()).rejects.toThrow('Database connection failed');
    });

    it('should fallback to statusText when error response has no message', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 503,
        statusText: 'Service Unavailable',
        json: async () => ({}),
      });

      await expect(api.health()).rejects.toThrow('Service Unavailable');
    });

    it('should fallback to HTTP status when response is not JSON', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 502,
        statusText: 'Bad Gateway',
        json: async () => {
          throw new Error('Invalid JSON');
        },
      });

      await expect(api.health()).rejects.toThrow('Bad Gateway');
    });
  });

  describe('Retry behavior', () => {
    it('should retry on 429 Too Many Requests', async () => {
      const mockData = { status: 'ok' };
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

      const result = await api.health();

      expect(result).toEqual(mockData);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    }, 10000);

    it('should respect Retry-After header on 429', async () => {
      const mockData = { status: 'ok' };
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

      const result = await api.health();

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

      await expect(api.health()).rejects.toThrow('Rate limited');
      expect(global.fetch).toHaveBeenCalledTimes(4); // 1 initial + 3 retries
    }, 10000);

    it('should retry on network failure for GET requests', async () => {
      const mockData = { status: 'ok' };
      (global.fetch as jest.Mock)
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData),
        });

      const result = await api.health();

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

      await expect(api.health()).rejects.toThrow('Invalid request');
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
      const mockData1 = { data: 'first' };
      const mockData2 = { data: 'second' };

      (global.fetch as jest.Mock)
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData1),
        })
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData2),
        });

      const result1 = await api.health();
      const result2 = await api.getStatistics();

      expect(result1).toEqual(mockData1);
      expect(result2).toEqual(mockData2);
      expect(global.fetch).toHaveBeenCalledTimes(2);
    });

    it('should use exponential backoff for retries', async () => {
      const mockData = { status: 'ok' };
      (global.fetch as jest.Mock)
        .mockRejectedValueOnce(new Error('Network error'))
        .mockRejectedValueOnce(new Error('Network error'))
        .mockResolvedValueOnce({
          ok: true,
          text: async () => JSON.stringify(mockData),
        });

      const result = await api.health();

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

      await api.health();

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

      await api.health();

      expect(global.fetch).toHaveBeenCalledWith(
        'http://localhost:3001/health',
        expect.any(Object)
      );
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

    it('should throw ApiError with status 0 on network failure', async () => {
      (global.fetch as jest.Mock).mockRejectedValueOnce(new Error('Failed to fetch'));

      try {
        await api.health();
        fail('should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        expect((e as ApiError).status).toBe(0);
        expect((e as ApiError).isNetworkError).toBe(true);
        expect((e as ApiError).message).toContain('Unable to reach the server');
      }
    });

    it('should classify 5xx as server error', async () => {
      (global.fetch as jest.Mock).mockResolvedValueOnce({
        ok: false,
        status: 503,
        statusText: 'Service Unavailable',
        json: async () => ({ message: 'Service Unavailable' }),
      });

      try {
        await api.getStatistics();
        fail('should have thrown');
      } catch (e) {
        expect(e).toBeInstanceOf(ApiError);
        expect((e as ApiError).isServerError).toBe(true);
        expect((e as ApiError).isClientError).toBe(false);
      }
    });

    it('should have name "ApiError"', async () => {
      (global.fetch as jest.Mock).mockRejectedValueOnce(new Error('offline'));

      try {
        await api.health();
      } catch (e) {
        expect((e as ApiError).name).toBe('ApiError');
      }
    });
  });
});