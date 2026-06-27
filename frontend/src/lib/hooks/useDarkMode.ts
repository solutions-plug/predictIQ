import { useState, useEffect } from 'react';
import {
  DARK_MODE_STORAGE_KEY,
  applyDarkModePreference,
  getDarkModePreference,
} from '../darkMode';

const defaultPreference = {
  isDarkMode: false,
  hasStoredPreference: false,
};

/**
 * Hook for managing dark mode preference
 * Respects system preference and allows manual toggle
 * Persists preference to localStorage
 */
export function useDarkMode() {
  const [preference, setPreference] = useState(defaultPreference);
  const [isLoaded, setIsLoaded] = useState(false);

  useEffect(() => {
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
