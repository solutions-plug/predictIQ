/**
 * Tests for the getDarkModePreference and applyDarkModePreference helpers
 * in darkMode.ts, covering system preference detection, stored explicit
 * choice, and the no-preference default.
 *
 * Issue #1937: Add tests for the dark mode helper.
 */

import {
  getDarkModePreference,
  applyDarkModePreference,
  DARK_MODE_STORAGE_KEY,
  THEME_ATTRIBUTE,
} from '../darkMode';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mockMatchMedia(prefersDark: boolean) {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: jest.fn().mockImplementation((query: string) => ({
      matches: prefersDark,
      media: query,
      onchange: null,
      addListener: jest.fn(),
      removeListener: jest.fn(),
      addEventListener: jest.fn(),
      removeEventListener: jest.fn(),
      dispatchEvent: jest.fn(),
    })),
  });
}

// ---------------------------------------------------------------------------
// Setup / teardown
// ---------------------------------------------------------------------------

beforeEach(() => {
  localStorage.clear();
  document.documentElement.removeAttribute(THEME_ATTRIBUTE);
  // Default: system does NOT prefer dark
  mockMatchMedia(false);
});

// ---------------------------------------------------------------------------
// getDarkModePreference
// ---------------------------------------------------------------------------

describe('getDarkModePreference', () => {
  describe('system preference detection', () => {
    it('reports isDarkMode:true when matchMedia signals a dark preference and nothing is stored', () => {
      mockMatchMedia(true);

      const pref = getDarkModePreference();

      expect(pref.isDarkMode).toBe(true);
      expect(pref.hasStoredPreference).toBe(false);
    });

    it('reports isDarkMode:false when matchMedia signals a light preference and nothing is stored', () => {
      mockMatchMedia(false);

      const pref = getDarkModePreference();

      expect(pref.isDarkMode).toBe(false);
      expect(pref.hasStoredPreference).toBe(false);
    });
  });

  describe('stored explicit choice overrides system preference', () => {
    it('returns the stored dark choice even when the system prefers light', () => {
      mockMatchMedia(false); // OS: light
      localStorage.setItem(DARK_MODE_STORAGE_KEY, 'true');

      const pref = getDarkModePreference();

      expect(pref.isDarkMode).toBe(true);
      expect(pref.hasStoredPreference).toBe(true);
    });

    it('returns the stored light choice even when the system prefers dark', () => {
      mockMatchMedia(true); // OS: dark
      localStorage.setItem(DARK_MODE_STORAGE_KEY, 'false');

      const pref = getDarkModePreference();

      expect(pref.isDarkMode).toBe(false);
      expect(pref.hasStoredPreference).toBe(true);
    });
  });

  describe('no-preference default', () => {
    it('returns isDarkMode:false and hasStoredPreference:false when neither storage nor matchMedia indicates dark', () => {
      mockMatchMedia(false);
      // No item in storage

      const pref = getDarkModePreference();

      expect(pref.isDarkMode).toBe(false);
      expect(pref.hasStoredPreference).toBe(false);
    });

    it('ignores a stored value that is neither "true" nor "false" and falls back to the system preference', () => {
      mockMatchMedia(true);
      localStorage.setItem(DARK_MODE_STORAGE_KEY, 'invalid');

      const pref = getDarkModePreference();

      // Falls through to system preference because stored value is not canonical.
      expect(pref.isDarkMode).toBe(true);
      expect(pref.hasStoredPreference).toBe(false);
    });
  });
});

// ---------------------------------------------------------------------------
// applyDarkModePreference
// ---------------------------------------------------------------------------

describe('applyDarkModePreference', () => {
  it('sets data-theme="dark" when the stored preference is dark', () => {
    applyDarkModePreference({ isDarkMode: true, hasStoredPreference: true });

    expect(document.documentElement.getAttribute(THEME_ATTRIBUTE)).toBe('dark');
  });

  it('sets data-theme="light" when the stored preference is light', () => {
    applyDarkModePreference({ isDarkMode: false, hasStoredPreference: true });

    expect(document.documentElement.getAttribute(THEME_ATTRIBUTE)).toBe('light');
  });

  it('removes the data-theme attribute when following the system (no stored preference)', () => {
    // Pre-set the attribute to verify removal.
    document.documentElement.setAttribute(THEME_ATTRIBUTE, 'dark');

    applyDarkModePreference({ isDarkMode: true, hasStoredPreference: false });

    // No explicit choice — the CSS media query should drive the theme.
    expect(document.documentElement.getAttribute(THEME_ATTRIBUTE)).toBeNull();
  });

  it('stubs between tests: data-theme is absent at the start of each test', () => {
    // Verifies beforeEach cleanup so test cases do not leak attribute state.
    expect(document.documentElement.getAttribute(THEME_ATTRIBUTE)).toBeNull();
  });
});
