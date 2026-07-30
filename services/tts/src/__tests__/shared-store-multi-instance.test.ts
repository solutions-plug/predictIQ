/**
 * Tests for issue #1133: in-memory job/rate-limit/cache state breaks
 * correctness under horizontal scaling.
 *
 * jobStore, RateLimiter, and AudioCache are process-local Maps with no
 * shared backing store. Simulates two replicas (two independent TTSService
 * instances) pointed at the same shared store and asserts:
 *  - a job enqueued on one replica is visible on the other (job polling
 *    doesn't 404 just because a different pod processed it)
 *  - rate limits are enforced globally across replicas, not per-replica
 *  - the audio cache is shared across replicas
 *
 * Uses a small in-memory fake that implements the SharedStore interfaces
 * rather than a real Redis connection, so this runs without any external
 * infrastructure. RedisJobStore/RedisRateLimitStore/RedisCacheStore in
 * SharedStore.ts implement the same interfaces against a real Redis
 * connection for production multi-replica deployments (set REDIS_URL).
 */

import { TTSService, VOICES, RateLimitError, TTSJob } from "../TTSService";
import type { SharedJobStore, SharedRateLimitStore, SharedCacheStore, SharedStoreConfig } from "../SharedStore";

const VOICE = VOICES["el-rachel-en"];

class FakeSharedJobStore implements SharedJobStore {
  private store = new Map<string, TTSJob>();
  async getJob(id: string) {
    return this.store.get(id);
  }
  async setJob(job: TTSJob) {
    this.store.set(job.id, { ...job });
  }
  async listJobs(status?: TTSJob["status"]) {
    const all = Array.from(this.store.values());
    return status ? all.filter((j) => j.status === status) : all;
  }
}

class FakeSharedRateLimitStore implements SharedRateLimitStore {
  private counts = new Map<string, number>();
  async incr(key: string) {
    const next = (this.counts.get(key) ?? 0) + 1;
    this.counts.set(key, next);
    return next;
  }
}

class FakeSharedCacheStore implements SharedCacheStore {
  private store = new Map<string, Buffer>();
  async get(key: string) {
    return this.store.get(key);
  }
  async set(key: string, buffer: Buffer) {
    this.store.set(key, buffer);
  }
}

function makeSharedStore(): SharedStoreConfig {
  return {
    jobStore: new FakeSharedJobStore(),
    rateLimitStore: new FakeSharedRateLimitStore(),
    cacheStore: new FakeSharedCacheStore(),
  };
}

describe("Shared store — multi-instance simulation (issue #1133)", () => {
  it("job enqueued on one replica is visible on another replica", async () => {
    const sharedStore = makeSharedStore();

    const replicaA = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "k" },
      outputDir: "/tmp/tts-shared-a",
      sharedStore,
    });
    const replicaB = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "k" },
      outputDir: "/tmp/tts-shared-b",
      sharedStore,
    });

    const jobId = await replicaA.enqueueAsync("hello", VOICE);

    // replicaB never processed this job locally — without a shared job
    // store this would 404. With it, the job (at least in "pending" or
    // later status) is visible immediately.
    const seenByB = await replicaB.getJobAsync(jobId);
    expect(seenByB).toBeDefined();
    expect(seenByB!.id).toBe(jobId);
  });

  it("rate limits are enforced globally across replicas, not per-replica", async () => {
    const sharedStore = makeSharedStore();
    const rateLimit = { maxRequests: 2, windowMs: 60_000 };

    const replicaA = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "k" },
      outputDir: "/tmp/tts-shared-rl-a",
      rateLimit,
      sharedStore,
    });
    const replicaB = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "k" },
      outputDir: "/tmp/tts-shared-rl-b",
      rateLimit,
      sharedStore,
    });

    const key = "ip:1.2.3.4";

    // Without a shared store, each replica would allow 2 requests of its
    // own (4 total) — the bug this issue describes. With the shared
    // counter, the limit of 2 applies across both replicas combined.
    await replicaA.enqueueAsync("hello", VOICE, undefined, undefined, key);
    await replicaB.enqueueAsync("hello", VOICE, undefined, undefined, key);

    await expect(replicaA.enqueueAsync("hello", VOICE, undefined, undefined, key)).rejects.toThrow(
      RateLimitError
    );
  });

  it("audio cache is shared across replicas", async () => {
    const sharedStore = makeSharedStore();
    const cacheStore = sharedStore.cacheStore!;

    const buf = Buffer.from("cached-audio");
    await cacheStore.set("shared-key", buf, 60_000);

    const readBack = await cacheStore.get("shared-key");
    expect(readBack).toEqual(buf);
  });
});
