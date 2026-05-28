/**
 * Simple in-memory cache for API responses with TTL support.
 * Cache is invalidated on mutations (POST, DELETE).
 * Entries are marked stale on API errors so callers can show a visual indicator.
 */

interface CacheEntry<T> {
  data: T;
  timestamp: number;
  ttl: number;
  stale: boolean;
}

export interface CacheResult<T> {
  data: T;
  stale: boolean;
  timestamp: number;
}

class ApiCache {
  private cache = new Map<string, CacheEntry<unknown>>();

  /**
   * Get cached data if available. Returns stale data rather than null so
   * callers always have something to display while a fresh request is in flight.
   */
  get<T>(key: string): T | null {
    const result = this.getWithMeta<T>(key);
    return result ? result.data : null;
  }

  /**
   * Get cached data together with staleness metadata. Use this when the UI
   * needs to show a "Last updated X minutes ago" indicator.
   */
  getWithMeta<T>(key: string): CacheResult<T> | null {
    const entry = this.cache.get(key) as CacheEntry<T> | undefined;
    if (!entry) return null;

    const isExpired = Date.now() - entry.timestamp > entry.ttl;
    // Stale entries are retained past TTL so users see data rather than nothing.
    if (isExpired && !entry.stale) {
      this.cache.delete(key);
      return null;
    }

    return {
      data: entry.data,
      stale: entry.stale || isExpired,
      timestamp: entry.timestamp,
    };
  }

  /**
   * Set cache entry with TTL in milliseconds
   */
  set<T>(key: string, data: T, ttlMs: number): void {
    this.cache.set(key, {
      data,
      timestamp: Date.now(),
      ttl: ttlMs,
      stale: false,
    });
  }

  /**
   * Mark a single cache entry as stale. Called when the API returns an error
   * for a fresh request so the UI can display the old value with a warning.
   */
  markStale(key: string): void {
    const entry = this.cache.get(key);
    if (entry) {
      entry.stale = true;
    }
  }

  /**
   * Mark all entries matching a pattern as stale (e.g. after a partial API
   * outage affects a whole resource namespace).
   */
  markStaleByPattern(pattern: string | RegExp): void {
    const regex = typeof pattern === 'string' ? new RegExp(pattern) : pattern;
    for (const [key, entry] of this.cache.entries()) {
      if (regex.test(key)) {
        entry.stale = true;
      }
    }
  }

  /**
   * Remove a single entry. Use this on an explicit user-triggered refresh so
   * the next request fetches fresh data unconditionally.
   */
  invalidate(key: string): void {
    this.cache.delete(key);
  }

  /**
   * Invalidate cache by pattern (for mutations)
   */
  invalidateByPattern(pattern: string | RegExp): void {
    const regex = typeof pattern === 'string' ? new RegExp(pattern) : pattern;
    for (const key of this.cache.keys()) {
      if (regex.test(key)) {
        this.cache.delete(key);
      }
    }
  }

  /**
   * Clear all cache
   */
  clear(): void {
    this.cache.clear();
  }
}

export const apiCache = new ApiCache();

// Cache TTL constants (in milliseconds)
export const CACHE_TTL = {
  SHORT: 1 * 60 * 1000,      // 1 minute
  MEDIUM: 5 * 60 * 1000,     // 5 minutes
  LONG: 30 * 60 * 1000,      // 30 minutes
};
