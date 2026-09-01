export const DARK_MODE_STORAGE_KEY = 'darkMode';
export const THEME_ATTRIBUTE = 'data-theme';

export interface DarkModePreference {
  isDarkMode: boolean;
  hasStoredPreference: boolean;
}

export function getDarkModePreference(): DarkModePreference {
  if (typeof window === 'undefined') {
    return {
      isDarkMode: false,
      hasStoredPreference: false,
    };
  }

  try {
    const stored = window.localStorage.getItem(DARK_MODE_STORAGE_KEY);

    if (stored === 'true' || stored === 'false') {
      return {
        isDarkMode: stored === 'true',
        hasStoredPreference: true,
      };
    }
  } catch {
    // Ignore storage access errors and fall back to system preference.
  }

  return {
    isDarkMode: window.matchMedia('(prefers-color-scheme: dark)').matches,
    hasStoredPreference: false,
  };
}

/**
 * Apply a preference to <html>.
 *
 * - Explicit user choice → set data-theme="dark" | "light". The attribute
 *   outranks the prefers-color-scheme media query in tokens.css (which only
 *   matches :root:not([data-theme])), so an OS-level theme change can never
 *   override a stored choice.
 * - Following the system → remove the attribute. The CSS media query then
 *   tracks prefers-color-scheme live, with no page reload required.
 */
export function applyDarkModePreference({
  isDarkMode,
  hasStoredPreference,
}: DarkModePreference) {
  if (typeof document === 'undefined') {
    return;
  }

  if (hasStoredPreference) {
    document.documentElement.setAttribute(
      THEME_ATTRIBUTE,
      isDarkMode ? 'dark' : 'light',
    );
  } else {
    document.documentElement.removeAttribute(THEME_ATTRIBUTE);
  }
}

export const darkModeInitScript = `
(function () {
  try {
    var stored = localStorage.getItem('${DARK_MODE_STORAGE_KEY}');
    var hasStoredPreference = stored === 'true' || stored === 'false';
    // Only an explicit choice pins the data-theme attribute. Without one the
    // CSS prefers-color-scheme media query drives the theme (and keeps
    // following OS changes live), so nothing is written here.
    if (hasStoredPreference) {
      document.documentElement.setAttribute('${THEME_ATTRIBUTE}', stored === 'true' ? 'dark' : 'light');
    }
  } catch (error) {}
})();
`;
