/**
 * Shared, cross-replica state backing for job status, rate limiting, and the
 * audio cache (issue #1133).
 *
 * jobStore, RateLimiter, and AudioCache in TTSService.ts are process-local
 * Maps. That's fine for a single instance, but breaks under horizontal
 * scaling: GET /tts/job/:id 404s when the polling request lands on a
 * different pod than the one that processed the job, rate limits are only
 * enforced per-replica (multiplying the effective global limit by replica
 * count), and cache hit rate collapses across replicas.
 *
 * These interfaces let TTSService optionally delegate to a shared backend
 * (e.g. Redis) instead of process-local memory. Omit `sharedStore` from
 * TTSConfig to keep the existing in-memory, single-instance behavior.
 */

import type { Redis as RedisClient } from "ioredis";
import type { TTSJob } from "./TTSService";

export interface SharedJobStore {
  getJob(id: string): Promise<TTSJob | undefined>;
  setJob(job: TTSJob): Promise<void>;
  listJobs(status?: TTSJob["status"]): Promise<TTSJob[]>;
}

export interface SharedRateLimitStore {
  /**
   * Atomically increment the counter for `key` within `windowMs` and return
   * the new count. The first increment in a window sets its expiry.
   */
  incr(key: string, windowMs: number): Promise<number>;
}

export interface SharedCacheStore {
  get(key: string): Promise<Buffer | undefined>;
  set(key: string, buffer: Buffer, ttlMs: number): Promise<void>;
}

export interface SharedStoreConfig {
  jobStore?: SharedJobStore;
  rateLimitStore?: SharedRateLimitStore;
  cacheStore?: SharedCacheStore;
}

function serializeJob(job: TTSJob): string {
  return JSON.stringify(job);
}

function deserializeJob(raw: string): TTSJob {
  const parsed = JSON.parse(raw);
  return { ...parsed, createdAt: new Date(parsed.createdAt), updatedAt: new Date(parsed.updatedAt) };
}

/** Redis-backed job store. Jobs expire after `ttlMs` (default 24h) so completed jobs don't accumulate forever. */
export class RedisJobStore implements SharedJobStore {
  constructor(
    private redis: RedisClient,
    private prefix = "tts:job:",
    private ttlMs = 86_400_000
  ) {}

  async getJob(id: string): Promise<TTSJob | undefined> {
    const raw = await this.redis.get(this.prefix + id);
    return raw ? deserializeJob(raw) : undefined;
  }

  async setJob(job: TTSJob): Promise<void> {
    await this.redis.set(this.prefix + job.id, serializeJob(job), "PX", this.ttlMs);
  }

  async listJobs(status?: TTSJob["status"]): Promise<TTSJob[]> {
    const keys = await this.redis.keys(`${this.prefix}*`);
    if (keys.length === 0) return [];
    const raws = await this.redis.mget(...keys);
    const jobs = raws.filter((r): r is string => !!r).map(deserializeJob);
    return status ? jobs.filter((j) => j.status === status) : jobs;
  }
}

/** Redis-backed rate limit counter using atomic INCR + PEXPIRE-on-first-hit. */
export class RedisRateLimitStore implements SharedRateLimitStore {
  constructor(private redis: RedisClient, private prefix = "tts:rl:") {}

  async incr(key: string, windowMs: number): Promise<number> {
    const redisKey = this.prefix + key;
    const count = await this.redis.incr(redisKey);
    if (count === 1) {
      await this.redis.pexpire(redisKey, windowMs);
    }
    return count;
  }
}

/** Redis-backed audio cache, storing raw buffers with a per-entry TTL. */
export class RedisCacheStore implements SharedCacheStore {
  constructor(private redis: RedisClient, private prefix = "tts:cache:") {}

  async get(key: string): Promise<Buffer | undefined> {
    const raw = await this.redis.getBuffer(this.prefix + key);
    return raw ?? undefined;
  }

  async set(key: string, buffer: Buffer, ttlMs: number): Promise<void> {
    await this.redis.set(this.prefix + key, buffer, "PX", ttlMs);
  }
}

/** Lazily constructs an ioredis client. Kept out of the module-level import graph so ioredis stays optional at runtime for single-instance deployments. */
export function createRedisClient(url: string): RedisClient {
  // eslint-disable-next-line @typescript-eslint/no-var-requires
  const IORedis = require("ioredis");
  return new IORedis(url);
}

/** Builds a full SharedStoreConfig backed by a single Redis connection. */
export function createRedisSharedStore(url: string): SharedStoreConfig {
  const redis = createRedisClient(url);
  return {
    jobStore: new RedisJobStore(redis),
    rateLimitStore: new RedisRateLimitStore(redis),
    cacheStore: new RedisCacheStore(redis),
  };
}
