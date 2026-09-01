import { useState, useEffect } from 'react';
import {
  DARK_MODE_STORAGE_KEY,
  THEME_ATTRIBUTE,
  applyDarkModePreference,
  getDarkModePreference,
  type DarkModePreference,
} from '../darkMode';

/**
 * Read the theme state that the blocking inline init script already applied
 * to <html> before the first paint.  Called as a lazy useState initializer so
 * it runs synchronously on the very first render — not inside a useEffect —
 * which means the toggle icon/aria-label are correct from frame 0.
 *
 * The data-theme attribute is only ever set for an explicit user choice (both
 * the init script and the toggle write it exclusively when a preference is
 * stored), so its presence means hasStoredPreference === true. When it is
 * absent the app is following the OS preference — even if the OS happens to
 * prefer dark — so hasStoredPreference is false and live OS-following stays
 * active.
 *
 * Falls back to getDarkModePreference() in environments where the DOM is not
 * available (SSR) or where the init script did not run (no attribute present).
 */
function readInitialDarkMode(): DarkModePreference {
  if (typeof document === 'undefined') {
    // SSR: no DOM, return neutral default (server renders no toggle state).
    return { isDarkMode: false, hasStoredPreference: false };
  }

  const theme = document.documentElement.getAttribute(THEME_ATTRIBUTE);

  if (theme === 'dark') {
    return { isDarkMode: true, hasStoredPreference: true };
  }

  if (theme === 'light') {
    return { isDarkMode: false, hasStoredPreference: true };
  }

  // Init script didn't run or no explicit choice was stored — derive from
  // storage / system preference the same way getDarkModePreference() does.
  return getDarkModePreference();
}

/**
 * Returns true when the inline init script has already pinned a data-theme
 * attribute on <html>, meaning the lazy initializer already captured the
 * correct state and the mount effect should not overwrite it.
 */
function themeWasApplied(): boolean {
  if (typeof document === 'undefined') return false;
  return document.documentElement.hasAttribute(THEME_ATTRIBUTE);
}

/**
 * Hook for managing dark mode preference.
 * Respects system preference and allows manual toggle.
 * Persists preference to localStorage.
 *
 * The initial rendered state is derived from the data-theme attribute applied
 * by the blocking inline script in layout.tsx, so the toggle icon and
 * aria-label are correct on the very first paint with no flash of incorrect
 * state.
 */
export function useDarkMode() {
  // Lazy initializer: runs synchronously on first render, reads the data-theme
  // attribute that the init script applied before paint.
  const [preference, setPreference] = useState<DarkModePreference>(readInitialDarkMode);
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
    if (themeWasApplied()) {
      // The init script already pinned the explicit theme and the lazy
      // initializer already captured it — no need to re-derive or re-apply.
      // Just mark the hook as loaded.
      setIsLoaded(true);
      return;
    }

    // Fallback path: init script didn't run (e.g. hydration mismatch, test
    // environments without the script). Derive preference from storage / system
    // and apply it now.
    const initialPreference = getDarkModePreference();
    setPreference(initialPreference);
    applyDarkModePreference(initialPreference);
    setIsLoaded(true);
  }, []);

  useEffect(() => {
    if (preference.hasStoredPreference || typeof window === 'undefined') {
      return;
    }

    const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)');
    const handleChange = (event: MediaQueryListEvent) => {
      const nextPreference = {
        isDarkMode: event.matches,
        hasStoredPreference: false,
      };

      setPreference(nextPreference);
      applyDarkModePreference(nextPreference);
    };

    mediaQuery.addEventListener('change', handleChange);

    return () => {
      mediaQuery.removeEventListener('change', handleChange);
    };
  }, [preference.hasStoredPreference]);

  const toggleDarkMode = () => {
    setPreference((currentPreference) => {
      const nextPreference = {
        isDarkMode: !currentPreference.isDarkMode,
        hasStoredPreference: true,
      };

      try {
        localStorage.setItem(DARK_MODE_STORAGE_KEY, String(nextPreference.isDarkMode));
      } catch {
        // Keep the in-memory preference when storage is unavailable.
      }

      applyDarkModePreference(nextPreference);

      return nextPreference;
    });
  };

  return {
    isDarkMode: preference.isDarkMode,
    toggleDarkMode,
    isLoaded,
  };
}
