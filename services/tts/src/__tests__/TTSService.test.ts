/**
 * Tests for the synchronous `generate()` wait timeout (issue #1136).
 *
 * Previously `_waitForJob` hardcoded a 60,000ms timeout, independent of the
 * configurable retry policy (maxRetries x maxDelayMs per attempt). Under
 * sustained provider degradation, the synchronous endpoint could reject with
 * a timeout error while `_process(job)` kept running in the background and
 * eventually completed, with no way for the original caller to know.
 *
 * These tests verify:
 *  - The wait timeout is derived from the configured retry budget (via
 *    `deriveSyncWaitTimeoutMs`), not a fixed constant.
 *  - A job that outlives a short, explicitly configured sync wait timeout is
 *    still retrievable via `getJobAsync` once it eventually completes.
 */

import { TTSService, VOICES, deriveSyncWaitTimeoutMs, RetryConfig } from "../TTSService";

const VOICE = VOICES["el-rachel-en"];
const AUDIO_BYTES = Buffer.from("fake-mp3-bytes");

function audioResponse() {
  return {
    ok: true,
    arrayBuffer: async () =>
      AUDIO_BYTES.buffer.slice(AUDIO_BYTES.byteOffset, AUDIO_BYTES.byteOffset + AUDIO_BYTES.byteLength),
  };
}

/** Simulates a provider that eventually succeeds, but only after `delayMs`. */
function mockDelayedSuccess(delayMs: number) {
  (global as any).fetch = jest.fn().mockImplementation(
    () => new Promise((resolve) => setTimeout(() => resolve(audioResponse()), delayMs))
  );
}

describe("TTSService — synchronous wait timeout derived from retry budget (issue #1136)", () => {
  afterEach(() => {
    jest.restoreAllMocks();
    (global as any).fetch = undefined;
  });

  describe("deriveSyncWaitTimeoutMs", () => {
    it("scales with maxRetries, maxDelayMs, provider timeout, and provider count", () => {
      const retry: RetryConfig = { maxRetries: 2, maxDelayMs: 5_000 };
      // attemptsPerProvider = maxRetries + 1 = 3
      // perProviderBudgetMs = 3 * (5000 + 1000) = 18000
      // 1 provider -> 18000, 2 providers -> 36000
      expect(deriveSyncWaitTimeoutMs(retry, 1, 1_000)).toBe(18_000);
      expect(deriveSyncWaitTimeoutMs(retry, 2, 1_000)).toBe(36_000);
    });

    it("is not hardcoded to 60,000ms — a small retry budget yields a small timeout", () => {
      const retry: RetryConfig = { maxRetries: 0, maxDelayMs: 100 };
      expect(deriveSyncWaitTimeoutMs(retry, 1, 50)).toBeLessThan(60_000);
    });
  });

  it("derives the service's synchronous wait timeout from the configured retry budget by default", () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir: "/tmp/tts-timeout-test",
      retry: { maxRetries: 1, maxDelayMs: 5_000 },
      circuitBreaker: { timeoutMs: 2_000 },
    });

    // 1 provider, attemptsPerProvider = 2, perProviderBudgetMs = 2 * (5000 + 2000) = 14000
    expect((svc as any)._syncWaitTimeoutMs()).toBe(14_000);
  });

  it("honors an explicit syncWaitTimeoutMs override instead of deriving one", () => {
    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir: "/tmp/tts-timeout-test",
      retry: { maxRetries: 5, maxDelayMs: 60_000 },
      syncWaitTimeoutMs: 250,
    });

    expect((svc as any)._syncWaitTimeoutMs()).toBe(250);
  });

  it("keeps a job queryable via getJobAsync after the synchronous caller times out", async () => {
    // The provider takes 400ms to respond (simulating sustained degradation),
    // but the synchronous wait is configured far shorter than that — and far
    // shorter than the long retry budget background processing is allowed to
    // use. The HTTP caller should see a timeout, but the job must keep
    // processing and remain retrievable once it finishes.
    mockDelayedSuccess(400);

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir: "/tmp/tts-timeout-test",
      retry: { maxRetries: 3, maxDelayMs: 60_000 }, // long configured retry budget
      syncWaitTimeoutMs: 100, // independently short synchronous wait
    });

    let timeoutMessage = "";
    try {
      await svc.generate("hello world", VOICE, "elevenlabs");
      throw new Error("expected generate() to time out");
    } catch (err) {
      timeoutMessage = err instanceof Error ? err.message : String(err);
    }

    const match = timeoutMessage.match(/^Job (.+) timed out after 100ms$/);
    expect(match).not.toBeNull();
    const jobId = match![1];

    // The synchronous caller already gave up, but background processing is
    // still running — poll until it completes, then confirm it's queryable.
    const start = Date.now();
    let job = await svc.getJobAsync(jobId);
    while (job && job.status !== "done" && job.status !== "error" && Date.now() - start < 5_000) {
      await new Promise((r) => setTimeout(r, 25));
      job = await svc.getJobAsync(jobId);
    }

    expect(job).toBeDefined();
    expect(job!.status).toBe("done");
    expect(job!.outputPath).toBeTruthy();
  });
});
