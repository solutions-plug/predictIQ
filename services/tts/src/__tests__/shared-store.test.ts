import { RedisJobStore, RedisRateLimitStore, RedisCacheStore } from "../SharedStore";
import type { TTSJob } from "../TTSService";

/** Minimal in-memory ioredis fake with TTL-based and capacity-based eviction. */
class FakeRedis {
  private data = new Map<string, { v: string | Buffer; exp?: number }>();
  constructor(private capacity = Infinity) {}

  private live(key: string) {
    const e = this.data.get(key);
    if (e && e.exp !== undefined && e.exp <= Date.now()) {
      this.data.delete(key);
      return undefined;
    }
    return e;
  }
  private put(key: string, v: string | Buffer, exp?: number) {
    this.data.delete(key); // re-insert so insertion order == age
    this.data.set(key, { v, exp });
    while (this.data.size > this.capacity) {
      this.data.delete(this.data.keys().next().value as string); // evict oldest
    }
  }
  async get(k: string) {
    await Promise.resolve();
    const e = this.live(k);
    return e ? String(e.v) : null;
  }
  async getBuffer(k: string) {
    await Promise.resolve();
    return (this.live(k)?.v as Buffer | undefined) ?? null;
  }
  async set(k: string, v: string | Buffer, _px: string, ttl: number) {
    await Promise.resolve();
    this.put(k, v, Date.now() + ttl);
    return "OK";
  }
  async incr(k: string) {
    await Promise.resolve();
    const n = Number(this.live(k)?.v ?? 0) + 1;
    const exp = this.live(k)?.exp;
    this.put(k, String(n), exp);
    return n;
  }
  async pexpire(k: string, ms: number) {
    const e = this.live(k);
    if (e) e.exp = Date.now() + ms;
    return e ? 1 : 0;
  }
  async keys(pattern: string) {
    const prefix = pattern.replace(/\*$/, "");
    return [...this.data.keys()].filter((k) => k.startsWith(prefix) && this.live(k));
  }
  async mget(...keys: string[]) {
    return keys.map((k) => {
      const e = this.live(k);
      return e ? String(e.v) : null;
    });
  }
  size() {
    return this.data.size;
  }
}

const job = (id: string, status: TTSJob["status"] = "pending"): TTSJob =>
  ({ id, status, createdAt: new Date(), updatedAt: new Date() } as unknown as TTSJob);

describe("SharedStore (Redis-backed) behaviour", () => {
  it("loses no writes under interleaved concurrent reads and writes", async () => {
    const store = new RedisJobStore(new FakeRedis() as any);
    const ids = Array.from({ length: 50 }, (_, i) => `job-${i}`);

    const results = await Promise.all(
      ids.flatMap((id) => [store.setJob(job(id)), store.getJob(id), store.listJobs()])
    );
    expect(results.length).toBe(150);

    const listed = await store.listJobs();
    expect(listed.map((j) => j.id).sort()).toEqual([...ids].sort());
    const fetched = await Promise.all(ids.map((id) => store.getJob(id)));
    fetched.forEach((j, i) => expect(j?.id).toBe(ids[i]));
  });

  it("increments the rate-limit counter atomically under concurrency", async () => {
    const store = new RedisRateLimitStore(new FakeRedis() as any);
    const counts = await Promise.all(Array.from({ length: 20 }, () => store.incr("k", 1000)));
    expect([...counts].sort((a, b) => a - b)).toEqual(Array.from({ length: 20 }, (_, i) => i + 1));
  });

  it("evicts the oldest entry once capacity is exceeded", async () => {
    const redis = new FakeRedis(3);
    const store = new RedisCacheStore(redis as any);
    for (const k of ["a", "b", "c", "d"]) await store.set(k, Buffer.from(k), 60_000);

    expect(redis.size()).toBe(3);
    expect(await store.get("a")).toBeUndefined();
    expect((await store.get("b"))?.toString()).toBe("b");
    expect((await store.get("d"))?.toString()).toBe("d");
  });

  it("does not serve expired entries", async () => {
    const store = new RedisCacheStore(new FakeRedis() as any);
    await store.set("x", Buffer.from("x"), 1);
    await new Promise((r) => setTimeout(r, 10));
    expect(await store.get("x")).toBeUndefined();
  });

  it("returns a defined empty result for never-written keys instead of throwing", async () => {
    const redis = new FakeRedis() as any;
    await expect(new RedisJobStore(redis).getJob("nope")).resolves.toBeUndefined();
    await expect(new RedisJobStore(redis).listJobs()).resolves.toEqual([]);
    await expect(new RedisCacheStore(redis).get("nope")).resolves.toBeUndefined();
  });
});
