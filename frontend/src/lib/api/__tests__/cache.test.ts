import { apiCache, CACHE_TTL } from '../cache';

describe('apiCache', () => {
  beforeEach(() => {
    apiCache.clear();
  });

  describe('get and set', () => {
    it('should store and retrieve cached data', () => {
      const key = 'test-key';
      const data = { id: 1, name: 'test' };

      apiCache.set(key, data, CACHE_TTL.SHORT);
      const result = apiCache.get(key);

      expect(result).toEqual(data);
    });

    it('should return null for non-existent keys', () => {
      const result = apiCache.get('non-existent');
      expect(result).toBeNull();
    });

    it('should return null for expired entries', () => {
      const key = 'test-key';
      const data = { id: 1 };

      apiCache.set(key, data, 1); // 1ms TTL
      
      // Wait for expiration
      return new Promise(resolve => {
        setTimeout(() => {
          const result = apiCache.get(key);
          expect(result).toBeNull();
          resolve(undefined);
        }, 10);
      });
    });
  });

  describe('invalidateByPattern', () => {
    it('should invalidate cache by string pattern', () => {
      apiCache.set('api/users/1', { id: 1 }, CACHE_TTL.SHORT);
      apiCache.set('api/users/2', { id: 2 }, CACHE_TTL.SHORT);
      apiCache.set('api/posts/1', { id: 1 }, CACHE_TTL.SHORT);

      apiCache.invalidateByPattern('api/users');

      expect(apiCache.get('api/users/1')).toBeNull();
      expect(apiCache.get('api/users/2')).toBeNull();
      expect(apiCache.get('api/posts/1')).not.toBeNull();
    });

    it('should invalidate cache by regex pattern', () => {
      apiCache.set('api/users/1', { id: 1 }, CACHE_TTL.SHORT);
      apiCache.set('api/posts/1', { id: 1 }, CACHE_TTL.SHORT);

      apiCache.invalidateByPattern(/users/);

      expect(apiCache.get('api/users/1')).toBeNull();
      expect(apiCache.get('api/posts/1')).not.toBeNull();
    });
  });

  describe('clear', () => {
    it('should clear all cache entries', () => {
      apiCache.set('key1', { data: 1 }, CACHE_TTL.SHORT);
      apiCache.set('key2', { data: 2 }, CACHE_TTL.SHORT);

      apiCache.clear();

      expect(apiCache.get('key1')).toBeNull();
      expect(apiCache.get('key2')).toBeNull();
    });
  });

  describe('markStale', () => {
    it('should mark an existing entry as stale', () => {
      apiCache.set('key', { value: 1 }, CACHE_TTL.LONG);
      apiCache.markStale('key');
      const result = apiCache.getWithMeta('key');
      expect(result).not.toBeNull();
      expect(result?.stale).toBe(true);
    });

    it('should keep the data accessible after marking stale', () => {
      apiCache.set('key', { value: 42 }, CACHE_TTL.LONG);
      apiCache.markStale('key');
      expect(apiCache.get('key')).toEqual({ value: 42 });
    });

    it('should be a no-op for non-existent keys', () => {
      expect(() => apiCache.markStale('does-not-exist')).not.toThrow();
    });
  });

  describe('markStaleByPattern', () => {
    it('should mark all matching entries stale', () => {
      apiCache.set('api/users/1', { id: 1 }, CACHE_TTL.LONG);
      apiCache.set('api/users/2', { id: 2 }, CACHE_TTL.LONG);
      apiCache.set('api/posts/1', { id: 1 }, CACHE_TTL.LONG);
      apiCache.markStaleByPattern('api/users');
      expect(apiCache.getWithMeta('api/users/1')?.stale).toBe(true);
      expect(apiCache.getWithMeta('api/users/2')?.stale).toBe(true);
      expect(apiCache.getWithMeta('api/posts/1')?.stale).toBe(false);
    });
  });

  describe('getWithMeta', () => {
    it('should return data with stale=false for a fresh entry', () => {
      apiCache.set('key', { value: 1 }, CACHE_TTL.LONG);
      const result = apiCache.getWithMeta('key');
      expect(result).not.toBeNull();
      expect(result?.stale).toBe(false);
      expect(result?.data).toEqual({ value: 1 });
    });

    it('should return null for non-existent key', () => {
      expect(apiCache.getWithMeta('missing')).toBeNull();
    });

    it('should return stale=true for an expired entry still held in memory', () => {
      apiCache.set('key', { value: 1 }, 1); // 1 ms TTL
      apiCache.markStale('key'); // force stale so it is retained past TTL
      return new Promise<void>(resolve => {
        setTimeout(() => {
          const result = apiCache.getWithMeta('key');
          expect(result).not.toBeNull();
          expect(result?.stale).toBe(true);
          resolve();
        }, 10);
      });
    });

    it('should include the original set timestamp', () => {
      const before = Date.now();
      apiCache.set('key', {}, CACHE_TTL.LONG);
      const result = apiCache.getWithMeta('key');
      const after = Date.now();
      expect(result?.timestamp).toBeGreaterThanOrEqual(before);
      expect(result?.timestamp).toBeLessThanOrEqual(after);
    });
  });

  describe('invalidate', () => {
    it('should remove a single entry', () => {
      apiCache.set('key', { value: 1 }, CACHE_TTL.SHORT);
      apiCache.invalidate('key');
      expect(apiCache.get('key')).toBeNull();
    });

    it('should be a no-op for non-existent keys', () => {
      expect(() => apiCache.invalidate('missing')).not.toThrow();
    });
  });

  describe('TTL constants', () => {
    it('should have correct TTL values', () => {
      expect(CACHE_TTL.SHORT).toBe(60 * 1000);
      expect(CACHE_TTL.MEDIUM).toBe(5 * 60 * 1000);
      expect(CACHE_TTL.LONG).toBe(30 * 60 * 1000);
    });
  });
});
