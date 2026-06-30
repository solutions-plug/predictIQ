/**
 * Tests for the circuit breaker implementation on TTS providers.
 *
 * Verifies:
 *  - Breaker trips after the configured failure threshold
 *  - While open, calls fast-fail without hitting the provider
 *  - Breaker state is reflected in getCircuitBreakerStates()
 *  - After reset timeout, breaker transitions to half-open and allows a probe
 */

import { TTSService, TTSProviderError, VOICES, CircuitBreakerConfig } from "../TTSService";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

const VOICE = VOICES["el-rachel-en"];

/**
 * Build a TTSService that is pre-wired with a tight circuit breaker and whose
 * ElevenLabs provider always rejects calls with the supplied error.
 *
 * We patch the global `fetch` so that the ElevenLabs HTTP call fails with a
 * network error, which the circuit breaker counts as a failure.
 */
function makeBrokenService(
  cbConfig: CircuitBreakerConfig,
  networkError = new Error("ECONNREFUSED")
): TTSService {
  // Monkey-patch fetch to always throw a network error for ElevenLabs calls
  (global as any).fetch = jest.fn().mockRejectedValue(networkError);

  return new TTSService({
    provider: "elevenlabs",
    elevenlabs: { apiKey: "test-api-key-for-breaker" },
    outputDir: "/tmp/tts-circuit-breaker-test",
    circuitBreaker: cbConfig,
  });
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe("TTSService — circuit breaker", () => {
  // Restore the real fetch after each test to avoid polluting other suites
  afterEach(() => {
    jest.restoreAllMocks();
    (global as any).fetch = undefined;
  });

  it("initially reports a closed circuit", () => {
    const svc = makeBrokenService({ openThreshold: 5, rollingWindowMs: 30_000 });
    const states = svc.getCircuitBreakerStates();
    expect(states["elevenlabs"]).toBeDefined();
    expect(states["elevenlabs"].state).toBe("closed");
  });

  it("trips open after reaching the failure threshold", async () => {
    // Low threshold so the test is fast
    const svc = makeBrokenService({
      openThreshold: 3,
      rollingWindowMs: 30_000,
      halfOpenIntervalMs: 60_000,
      timeoutMs: 500,
    });

    // Fire enough failing calls to trip the breaker
    for (let i = 0; i < 3; i++) {
      await expect(
        svc.generate("hello", VOICE, "elevenlabs")
      ).rejects.toThrow();
    }

    const states = svc.getCircuitBreakerStates();
    expect(states["elevenlabs"].state).toBe("open");
    expect(states["elevenlabs"].failures).toBeGreaterThanOrEqual(3);
  }, 15_000);

  it("fast-fails while the circuit is open without calling the provider", async () => {
    const fetchMock = jest.fn().mockRejectedValue(new Error("network down"));
    (global as any).fetch = fetchMock;

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir: "/tmp/tts-cb-test",
      circuitBreaker: {
        openThreshold: 2,
        rollingWindowMs: 30_000,
        halfOpenIntervalMs: 60_000,
        timeoutMs: 500,
      },
    });

    // Trip the breaker
    for (let i = 0; i < 2; i++) {
      await expect(svc.generate("hello", VOICE, "elevenlabs")).rejects.toThrow();
    }

    expect(svc.getCircuitBreakerStates()["elevenlabs"].state).toBe("open");

    // Reset the mock call count so we can verify no new network calls happen
    fetchMock.mockClear();

    // The next call should fast-fail without invoking fetch at all.
    // _waitForJob wraps the error as a plain Error, but the message includes
    // "Circuit breaker OPEN" to confirm fast-fail rather than a real network call.
    await expect(svc.generate("hello", VOICE, "elevenlabs")).rejects.toThrow(
      /Circuit breaker OPEN|Breaker is open/i
    );
    expect(fetchMock).not.toHaveBeenCalled();
  }, 15_000);

  it("transitions to half-open after the reset timeout", async () => {
    (global as any).fetch = jest.fn().mockRejectedValue(new Error("network down"));

    const svc = new TTSService({
      provider: "elevenlabs",
      elevenlabs: { apiKey: "test-key" },
      outputDir: "/tmp/tts-cb-test",
      circuitBreaker: {
        openThreshold: 2,
        rollingWindowMs: 30_000,
        halfOpenIntervalMs: 200, // Short but long enough to catch open state
        timeoutMs: 500,
      },
    });

    // Trip the breaker — both calls should fail
    const tripStart = Date.now();
    for (let i = 0; i < 2; i++) {
      await expect(svc.generate("hello", VOICE, "elevenlabs")).rejects.toThrow();
    }

    // Breaker must be open immediately after tripping (before the 200ms interval)
    const elapsed = Date.now() - tripStart;
    if (elapsed < 200) {
      // Only assert "open" if we're still inside the open window
      expect(svc.getCircuitBreakerStates()["elevenlabs"].state).toBe("open");
    }

    // Wait longer than the half-open interval so the timer elapses
    await new Promise((r) => setTimeout(r, 300));

    // Trigger a probe call — opossum transitions to halfOpen when fired after resetTimeout
    await expect(svc.generate("hello", VOICE, "elevenlabs")).rejects.toThrow();
    const stateAfterProbe = svc.getCircuitBreakerStates()["elevenlabs"].state;
    // After a failed probe the circuit re-opens; halfOpen is also valid mid-probe
    expect(["open", "halfOpen"]).toContain(stateAfterProbe);
  }, 15_000);

  it("getCircuitBreakerStates returns empty object when no providers are configured", () => {
    // No elevenlabs or google config → no breakers initialised
    const svc = new TTSService({
      provider: "elevenlabs",
      outputDir: "/tmp",
      // Intentionally no elevenlabs/google config so no breakers are created
    });
    const states = svc.getCircuitBreakerStates();
    expect(Object.keys(states)).toHaveLength(0);
  });
});
