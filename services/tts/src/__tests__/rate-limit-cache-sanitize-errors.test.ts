/**
 * Tests for issues #531, #532, #533, #534:
 *  - Rate limiting
 *  - Audio caching
 *  - Error handling & fallback
 *  - Input sanitization
 */

import {
  RateLimiter,
  RateLimitError,
  AudioCache,
  sanitizeInput,
  InputValidationError,
  MAX_INPUT_LENGTH,
  TTSProviderError,
  TTSService,
  VOICES,
} from "../TTSService";

const VOICE = VOICES["el-rachel-en"];

// ---------------------------------------------------------------------------
// Issue #531 — Rate limiting
// ---------------------------------------------------------------------------

describe("RateLimiter", () => {
  it("allows requests within the limit", () => {
    const rl = new RateLimiter({ maxRequests: 3, windowMs: 10_000 });
    expect(() => { rl.check("ip:1"); rl.check("ip:1"); rl.check("ip:1"); }).not.toThrow();
  });

  it("throws RateLimitError when limit is exceeded", () => {
    const rl = new RateLimiter({ maxRequests: 2, windowMs: 10_000 });
    rl.check("ip:1");
    rl.check("ip:1");
    expect(() => rl.check("ip:1")).toThrow(RateLimitError);
  });

  it("RateLimitError has statusCode 429", () => {
    const rl = new RateLimiter({ maxRequests: 0, windowMs: 10_000 });
    try { rl.check("ip:1"); } catch (e) {
      expect((e as RateLimitError).statusCode).toBe(429);
    }
  });

  it("resets counter after window expires", async () => {
    const rl = new RateLimiter({ maxRequests: 1, windowMs: 50 });
    rl.check("ip:1");
    await new Promise((r) => setTimeout(r, 60));
    expect(() => rl.check("ip:1")).not.toThrow();
  });

  it("tracks metrics", () => {
    const rl = new RateLimiter({ maxRequests: 1, windowMs: 10_000 });
    rl.check("ip:2");
    try { rl.check("ip:2"); } catch { /* expected */ }
    const m = rl.getMetrics();
    expect(m.totalChecks).toBe(2);
    expect(m.totalExceeded).toBe(1);
  });

  it("isolates limits per key", () => {
    const rl = new RateLimiter({ maxRequests: 1, windowMs: 10_000 });
    rl.check("ip:A");
    expect(() => rl.check("ip:B")).not.toThrow();
  });

  it("TTSService enforces rate limit via enqueue", () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "k" },
      outputDir: "/tmp",
      rateLimit: { maxRequests: 1, windowMs: 10_000 },
    });
    svc.enqueue("hello", VOICE, undefined, undefined, "ip:x");
    expect(() => svc.enqueue("hello", VOICE, undefined, undefined, "ip:x")).toThrow(RateLimitError);
  });

  it("TTSService exposes rate limit metrics", () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "k" },
      outputDir: "/tmp",
      rateLimit: { maxRequests: 5, windowMs: 10_000 },
    });
    svc.enqueue("hello", VOICE, undefined, undefined, "user:1");
    const m = svc.getRateLimitMetrics();
    expect(m).not.toBeNull();
    expect(m!.totalChecks).toBe(1);
  });

  it("returns null metrics when rate limiting is disabled", () => {
    const svc = new TTSService({ provider: "elevenlabs", elevenlabs: { apiKey: "k" }, outputDir: "/tmp" });
    expect(svc.getRateLimitMetrics()).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Issue #532 — Audio caching
// ---------------------------------------------------------------------------

describe("AudioCache", () => {
  it("returns undefined on cache miss", () => {
    const cache = new AudioCache({ ttlMs: 10_000, maxEntries: 10 });
    expect(cache.get("nonexistent")).toBeUndefined();
  });

  it("returns cached buffer on hit", () => {
    const cache = new AudioCache({ ttlMs: 10_000, maxEntries: 10 });
    const buf = Buffer.from("audio");
    cache.set("k1", buf);
    expect(cache.get("k1")).toEqual(buf);
  });

  it("expires entries after TTL", async () => {
    const cache = new AudioCache({ ttlMs: 30, maxEntries: 10 });
    cache.set("k1", Buffer.from("audio"));
    await new Promise((r) => setTimeout(r, 50));
    expect(cache.get("k1")).toBeUndefined();
  });

  it("evicts oldest entry when maxEntries is reached", () => {
    const cache = new AudioCache({ ttlMs: 10_000, maxEntries: 2 });
    cache.set("k1", Buffer.from("a"));
    cache.set("k2", Buffer.from("b"));
    cache.set("k3", Buffer.from("c")); // should evict k1
    expect(cache.get("k1")).toBeUndefined();
    expect(cache.get("k2")).toBeDefined();
    expect(cache.get("k3")).toBeDefined();
  });

  it("tracks hit/miss/eviction metrics", () => {
    const cache = new AudioCache({ ttlMs: 10_000, maxEntries: 1 });
    cache.get("miss");                        // miss
    cache.set("k1", Buffer.from("a"));
    cache.get("k1");                          // hit
    cache.set("k2", Buffer.from("b"));        // evicts k1
    const m = cache.getMetrics();
    expect(m.hits).toBe(1);
    expect(m.misses).toBe(1);
    expect(m.evictions).toBe(1);
  });

  it("generates consistent cache keys", () => {
    const k1 = AudioCache.key("hello", "voice1", "elevenlabs");
    const k2 = AudioCache.key("hello", "voice1", "elevenlabs");
    const k3 = AudioCache.key("hello", "voice1", "google");
    expect(k1).toBe(k2);
    expect(k1).not.toBe(k3);
  });

  it("TTSService exposes cache metrics", () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "k" },
      outputDir: "/tmp",
      cache: { ttlMs: 60_000, maxEntries: 100 },
    });
    const m = svc.getCacheMetrics();
    expect(m).not.toBeNull();
    expect(m!.hits).toBe(0);
  });

  it("returns null cache metrics when caching is disabled", () => {
    const svc = new TTSService({ provider: "elevenlabs", elevenlabs: { apiKey: "k" }, outputDir: "/tmp" });
    expect(svc.getCacheMetrics()).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Issue #534 — Input sanitization
// ---------------------------------------------------------------------------

describe("sanitizeInput", () => {
  it("returns clean text unchanged", () => {
    expect(sanitizeInput("Hello world")).toBe("Hello world");
  });

  it("strips SSML/XML tags", () => {
    expect(sanitizeInput("<speak>Hello <break time='1s'/> world</speak>")).toBe("Hello world");
  });

  it("strips nested tags", () => {
    expect(sanitizeInput("<b><i>text</i></b>")).toBe("text");
  });

  it("normalizes whitespace", () => {
    expect(sanitizeInput("  hello   world  ")).toBe("hello world");
  });

  it("throws InputValidationError for empty string", () => {
    expect(() => sanitizeInput("")).toThrow(InputValidationError);
    expect(() => sanitizeInput("   ")).toThrow(InputValidationError);
  });

  it("throws InputValidationError for non-string input", () => {
    expect(() => sanitizeInput(null as unknown as string)).toThrow(InputValidationError);
  });

  it("throws InputValidationError when input exceeds MAX_INPUT_LENGTH", () => {
    const long = "a".repeat(MAX_INPUT_LENGTH + 1);
    expect(() => sanitizeInput(long)).toThrow(InputValidationError);
  });

  it("accepts input exactly at MAX_INPUT_LENGTH", () => {
    const exact = "a".repeat(MAX_INPUT_LENGTH);
    expect(() => sanitizeInput(exact)).not.toThrow();
  });

  it("InputValidationError has statusCode 400", () => {
    try { sanitizeInput(""); } catch (e) {
      expect((e as InputValidationError).statusCode).toBe(400);
    }
  });

  it("TTSService sanitizes input before enqueue", () => {
    const svc = new TTSService({ provider: "elevenlabs", elevenlabs: { apiKey: "k" }, outputDir: "/tmp" });
    expect(() => svc.enqueue("", VOICE)).toThrow(InputValidationError);
  });

  it("TTSService strips SSML tags from enqueued text", () => {
    const svc = new TTSService({ provider: "elevenlabs", elevenlabs: { apiKey: "k" }, outputDir: "/tmp" });
    const id = svc.enqueue("<speak>Hello</speak>", VOICE);
    const job = svc.getJob(id);
    expect(job?.text).toBe("Hello");
  });
});

// ---------------------------------------------------------------------------
// Issue #533 — Error handling & fallback
// ---------------------------------------------------------------------------

describe("TTSProviderError", () => {
  it("has correct name and provider", () => {
    const err = new TTSProviderError("elevenlabs", "quota exceeded");
    expect(err.name).toBe("TTSProviderError");
    expect(err.provider).toBe("elevenlabs");
    expect(err.message).toContain("quota exceeded");
  });

  it("defaults to statusCode 502", () => {
    expect(new TTSProviderError("google", "fail").statusCode).toBe(502);
  });

  it("accepts custom statusCode", () => {
    expect(new TTSProviderError("elevenlabs", "not found", 404).statusCode).toBe(404);
  });
});
