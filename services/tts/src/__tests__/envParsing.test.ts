/**
 * Tests for parsePositiveIntEnv in envParsing.ts.
 *
 * Covers:
 *   - A complete valid environment parses into the expected config.
 *   - A missing required variable yields the documented default (the function
 *     returns a default value rather than throwing when a variable is absent).
 *   - An invalid value (non-numeric, e.g. a non-integer port) is rejected
 *     with a descriptive error.
 *
 * process.env is restored after each test to prevent cross-test contamination.
 *
 * Issue #1939: Add tests for the TTS env parsing.
 */

import { parsePositiveIntEnv } from '../envParsing';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

type Env = NodeJS.ProcessEnv;

function envWith(overrides: Record<string, string | undefined>): Env {
  return { ...process.env, ...overrides };
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

describe('parsePositiveIntEnv', () => {
  // Save and restore process.env after each test so mutations do not leak.
  const originalEnv = process.env;

  afterEach(() => {
    process.env = originalEnv;
  });

  // -------------------------------------------------------------------------
  // Valid environment — expected config
  // -------------------------------------------------------------------------

  describe('valid environment', () => {
    it('parses a valid port value and returns the integer', () => {
      const result = parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: '200' }));

      expect(result).toBe(200);
    });

    it('parses TTS_RATE_LIMIT_WINDOW_MS as a positive integer', () => {
      const result = parsePositiveIntEnv(
        'TTS_RATE_LIMIT_WINDOW_MS',
        60000,
        envWith({ TTS_RATE_LIMIT_WINDOW_MS: '30000' }),
      );

      expect(result).toBe(30000);
    });

    it('parses TTS_CACHE_MAX_ENTRIES correctly', () => {
      const result = parsePositiveIntEnv(
        'TTS_CACHE_MAX_ENTRIES',
        1000,
        envWith({ TTS_CACHE_MAX_ENTRIES: '500' }),
      );

      expect(result).toBe(500);
    });

    it('parses TTS_MAX_INPUT_LENGTH correctly', () => {
      const result = parsePositiveIntEnv(
        'TTS_MAX_INPUT_LENGTH',
        5000,
        envWith({ TTS_MAX_INPUT_LENGTH: '1000' }),
      );

      expect(result).toBe(1000);
    });

    it('parses a value with surrounding whitespace by trimming it', () => {
      const result = parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: '  150  ' }));

      expect(result).toBe(150);
    });
  });

  // -------------------------------------------------------------------------
  // Missing variable — documented default
  // -------------------------------------------------------------------------

  describe('missing required variable', () => {
    it('returns the documented default (100) for TTS_RATE_LIMIT_MAX when unset', () => {
      const result = parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: undefined }));

      expect(result).toBe(100);
    });

    it('returns the documented default (60000) for TTS_RATE_LIMIT_WINDOW_MS when unset', () => {
      const result = parsePositiveIntEnv('TTS_RATE_LIMIT_WINDOW_MS', 60000, envWith({ TTS_RATE_LIMIT_WINDOW_MS: undefined }));

      expect(result).toBe(60000);
    });

    it('returns the documented default (86400000) for TTS_CACHE_TTL_MS when unset', () => {
      const result = parsePositiveIntEnv('TTS_CACHE_TTL_MS', 86400000, envWith({ TTS_CACHE_TTL_MS: undefined }));

      expect(result).toBe(86400000);
    });

    it('returns the documented default (1000) for TTS_CACHE_MAX_ENTRIES when unset', () => {
      const result = parsePositiveIntEnv('TTS_CACHE_MAX_ENTRIES', 1000, envWith({ TTS_CACHE_MAX_ENTRIES: undefined }));

      expect(result).toBe(1000);
    });

    it('returns the documented default (5000) for TTS_MAX_INPUT_LENGTH when unset', () => {
      const result = parsePositiveIntEnv('TTS_MAX_INPUT_LENGTH', 5000, envWith({ TTS_MAX_INPUT_LENGTH: undefined }));

      expect(result).toBe(5000);
    });

    it('returns the default when the variable is set to an empty string', () => {
      const result = parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: '' }));

      expect(result).toBe(100);
    });

    it('returns the default when the variable is set to only whitespace', () => {
      const result = parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: '   ' }));

      expect(result).toBe(100);
    });
  });

  // -------------------------------------------------------------------------
  // Invalid value — rejected with a descriptive error
  // -------------------------------------------------------------------------

  describe('invalid value', () => {
    it('throws a descriptive error when the value is not a number', () => {
      expect(() =>
        parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: 'not-a-number' })),
      ).toThrow(/Invalid TTS_RATE_LIMIT_MAX/);
    });

    it('throws when the value is a float (non-integer)', () => {
      expect(() =>
        parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: '10.5' })),
      ).toThrow(/Invalid TTS_RATE_LIMIT_MAX/);
    });

    it('throws when the value is zero (not a positive integer)', () => {
      expect(() =>
        parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: '0' })),
      ).toThrow(/Invalid TTS_RATE_LIMIT_MAX/);
    });

    it('throws when the value is negative', () => {
      expect(() =>
        parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: '-5' })),
      ).toThrow(/Invalid TTS_RATE_LIMIT_MAX/);
    });

    it('includes the offending value in the error message', () => {
      expect(() =>
        parsePositiveIntEnv('TTS_RATE_LIMIT_MAX', 100, envWith({ TTS_RATE_LIMIT_MAX: 'bad-port' })),
      ).toThrow(/"bad-port"/);
    });

    it('throws for TTS_RATE_LIMIT_WINDOW_MS when given a non-numeric string', () => {
      expect(() =>
        parsePositiveIntEnv(
          'TTS_RATE_LIMIT_WINDOW_MS',
          60000,
          envWith({ TTS_RATE_LIMIT_WINDOW_MS: 'invalid' }),
        ),
      ).toThrow(/Invalid TTS_RATE_LIMIT_WINDOW_MS/);
    });

    it('throws for TTS_MAX_INPUT_LENGTH when given a non-numeric port-like string', () => {
      expect(() =>
        parsePositiveIntEnv(
          'TTS_MAX_INPUT_LENGTH',
          5000,
          envWith({ TTS_MAX_INPUT_LENGTH: 'abc' }),
        ),
      ).toThrow(/Invalid TTS_MAX_INPUT_LENGTH/);
    });
  });
});
