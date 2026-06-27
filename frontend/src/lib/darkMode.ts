export const DARK_MODE_STORAGE_KEY = 'darkMode';

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

export function applyDarkModePreference({
  isDarkMode,
  hasStoredPreference,
}: DarkModePreference) {
  if (typeof document === 'undefined') {
    return;
  }

  document.documentElement.classList.toggle('dark-mode', isDarkMode);
  document.documentElement.classList.toggle(
    'light-mode',
    !isDarkMode && hasStoredPreference,
  );
}

export const darkModeInitScript = `
(function () {
  try {
    var stored = localStorage.getItem('${DARK_MODE_STORAGE_KEY}');
    var hasStoredPreference = stored === 'true' || stored === 'false';
    var isDarkMode = hasStoredPreference
      ? stored === 'true'
      : window.matchMedia('(prefers-color-scheme: dark)').matches;

    document.documentElement.classList.toggle('dark-mode', isDarkMode);
    document.documentElement.classList.toggle('light-mode', !isDarkMode && hasStoredPreference);
  } catch (error) {}
})();
`;
