/**
 * Tests for perf/audio-cache-is-unbounded-by-memory-size-only-by.
 *
 * AudioCache.set() previously evicted only when store.size >= maxEntries,
 * with no cap on total bytes cached. Since MAX_INPUT_LENGTH allows up to
 * 5000 characters per request, synthesized MP3 buffers can be several MB
 * each — so the default config could retain gigabytes of audio in process
 * heap with no memory-based eviction. These tests assert a configurable
 * `maxBytes` budget bounds total cached bytes and evicts the oldest entries
 * to make room.
 */

import { AudioCache } from "../TTSService";

describe("AudioCache — byte budget (perf/audio-cache-is-unbounded-by-memory-size-only-by)", () => {
  it("tracks total bytes across cached buffers in metrics", () => {
    const cache = new AudioCache({ ttlMs: 10_000, maxEntries: 10 });
    cache.set("k1", Buffer.alloc(100));
    cache.set("k2", Buffer.alloc(250));
    expect(cache.getMetrics().bytes).toBe(350);
  });

  it("evicts the oldest entries once the byte budget is exceeded, even with entries well under maxEntries", () => {
    const oneMb = 1024 * 1024;
    const cache = new AudioCache({ ttlMs: 10_000, maxEntries: 1000, maxBytes: oneMb * 2 });

    cache.set("k1", Buffer.alloc(oneMb)); // 1MB total
    cache.set("k2", Buffer.alloc(oneMb)); // 2MB total — at budget
    expect(cache.get("k1")).toBeDefined();

    cache.set("k3", Buffer.alloc(oneMb)); // pushes to 3MB — must evict k1 to fit

    expect(cache.get("k1")).toBeUndefined(); // evicted to stay within budget
    expect(cache.get("k2")).toBeDefined();
    expect(cache.get("k3")).toBeDefined();
    expect(cache.getMetrics().bytes).toBeLessThanOrEqual(oneMb * 2);
  });

  it("keeps memory usage bounded when filled with many large buffers", () => {
    const oneMb = 1024 * 1024;
    const budget = oneMb * 5;
    const cache = new AudioCache({ ttlMs: 60_000, maxEntries: 10_000, maxBytes: budget });

    // Fill with far more than the budget would allow if only maxEntries applied.
    for (let i = 0; i < 20; i++) {
      cache.set(`k${i}`, Buffer.alloc(oneMb));
    }

    const metrics = cache.getMetrics();
    expect(metrics.bytes).toBeLessThanOrEqual(budget);
    expect(metrics.evictions).toBeGreaterThan(0);
    // Only the most recent entries (within budget) should remain retrievable.
    expect(cache.get("k19")).toBeDefined();
    expect(cache.get("k0")).toBeUndefined();
  });

  it("does not cache a single buffer that alone exceeds the byte budget", () => {
    const cache = new AudioCache({ ttlMs: 10_000, maxEntries: 10, maxBytes: 1000 });
    cache.set("too-big", Buffer.alloc(2000));
    expect(cache.get("too-big")).toBeUndefined();
    expect(cache.getMetrics().bytes).toBe(0);
  });

  it("decrements tracked bytes when an entry expires via TTL", async () => {
    const cache = new AudioCache({ ttlMs: 20, maxEntries: 10, maxBytes: 10_000 });
    cache.set("k1", Buffer.alloc(500));
    expect(cache.getMetrics().bytes).toBe(500);

    await new Promise((r) => setTimeout(r, 40));
    expect(cache.get("k1")).toBeUndefined();
    expect(cache.getMetrics().bytes).toBe(0);
  });

  it("is backward compatible: without maxBytes configured, only entry-count eviction applies", () => {
    const cache = new AudioCache({ ttlMs: 10_000, maxEntries: 2 });
    cache.set("k1", Buffer.alloc(1024 * 1024 * 10)); // 10MB — no byte budget configured
    cache.set("k2", Buffer.alloc(1024 * 1024 * 10));
    cache.set("k3", Buffer.alloc(1024 * 1024 * 10)); // evicts k1 by count only

    expect(cache.get("k1")).toBeUndefined();
    expect(cache.get("k2")).toBeDefined();
    expect(cache.get("k3")).toBeDefined();
  });
});
