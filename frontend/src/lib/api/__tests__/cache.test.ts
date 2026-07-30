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

  describe('TTL constants', () => {
    it('should have correct TTL values', () => {
      expect(CACHE_TTL.SHORT).toBe(60 * 1000);
      expect(CACHE_TTL.MEDIUM).toBe(5 * 60 * 1000);
      expect(CACHE_TTL.LONG).toBe(30 * 60 * 1000);
    });
  });

  describe('tag-based invalidation (#947)', () => {
    it('invalidates entries matching any given tag while leaving others intact', () => {
      apiCache.set('url/markets/featured', [{ id: 1 }], CACHE_TTL.SHORT, ['markets']);
      apiCache.set('url/blockchain/markets/1', { id: 1 }, CACHE_TTL.MEDIUM, ['markets', 'blockchain']);
      apiCache.set('url/statistics', { total: 100 }, CACHE_TTL.MEDIUM, ['statistics']);

      apiCache.invalidateByTags(['markets']);

      expect(apiCache.get('url/markets/featured')).toBeNull();
      expect(apiCache.get('url/blockchain/markets/1')).toBeNull();
      // statistics has a different tag and must survive
      expect(apiCache.get('url/statistics')).toEqual({ total: 100 });
    });

    it('invalidates entries carrying multiple tags when any matches', () => {
      apiCache.set('url/blockchain/stats', { txs: 50 }, CACHE_TTL.MEDIUM, ['blockchain', 'statistics']);

      apiCache.invalidateByTags(['blockchain']);

      expect(apiCache.get('url/blockchain/stats')).toBeNull();
    });

    it('does not affect entries without tags', () => {
      apiCache.set('url/content', { items: [] }, CACHE_TTL.SHORT); // no tags

      apiCache.invalidateByTags(['markets']);

      expect(apiCache.get('url/content')).toEqual({ items: [] });
    });

    it('GET returns fresh data after mutation invalidates its cache tag', () => {
      // Simulate a cached GET response tagged 'markets'.
      apiCache.set('url/markets/featured', [{ id: 1, title: 'Old' }], CACHE_TTL.SHORT, ['markets']);
      expect(apiCache.get('url/markets/featured')).toEqual([{ id: 1, title: 'Old' }]);

      // Simulate a mutation that invalidates the 'markets' tag.
      apiCache.invalidateByTags(['markets']);

      // Cache miss — the next GET will fetch fresh data from the server.
      expect(apiCache.get('url/markets/featured')).toBeNull();
    });

    it('is a no-op when no entries carry the given tag', () => {
      apiCache.set('url/email/analytics', { sent: 10 }, CACHE_TTL.MEDIUM, ['email']);

      // Invalidating an unrelated tag must not remove anything.
      apiCache.invalidateByTags(['newsletter']);

      expect(apiCache.get('url/email/analytics')).toEqual({ sent: 10 });
    });
  });
});
