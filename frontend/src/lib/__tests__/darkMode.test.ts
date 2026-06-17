import {
  getDarkModePreference,
  applyDarkModePreference,
  DARK_MODE_STORAGE_KEY,
} from '../darkMode';

const mockMatchMedia = (matches: boolean) => {
  Object.defineProperty(window, 'matchMedia', {
    writable: true,
    value: jest.fn().mockImplementation((query: string) => ({
      matches,
      media: query,
      onchange: null,
      addEventListener: jest.fn(),
      removeEventListener: jest.fn(),
      dispatchEvent: jest.fn(),
    })),
  });
};

describe('getDarkModePreference', () => {
  beforeEach(() => {
    localStorage.clear();
    mockMatchMedia(false);
  });

  it('returns system preference when no value is stored', () => {
    mockMatchMedia(true);
    const pref = getDarkModePreference();
    expect(pref.isDarkMode).toBe(true);
    expect(pref.hasStoredPreference).toBe(false);
  });

  it('returns stored true preference', () => {
    localStorage.setItem(DARK_MODE_STORAGE_KEY, 'true');
    const pref = getDarkModePreference();
    expect(pref.isDarkMode).toBe(true);
    expect(pref.hasStoredPreference).toBe(true);
  });

  it('returns stored false preference', () => {
    localStorage.setItem(DARK_MODE_STORAGE_KEY, 'false');
    const pref = getDarkModePreference();
    expect(pref.isDarkMode).toBe(false);
    expect(pref.hasStoredPreference).toBe(true);
  });

  it('ignores unrecognised stored values and falls back to system', () => {
    localStorage.setItem(DARK_MODE_STORAGE_KEY, 'yes');
    mockMatchMedia(true);
    const pref = getDarkModePreference();
    expect(pref.isDarkMode).toBe(true);
    expect(pref.hasStoredPreference).toBe(false);
  });
});

describe('applyDarkModePreference', () => {
  beforeEach(() => {
    document.documentElement.classList.remove('dark-mode', 'light-mode');
  });

  it('adds dark-mode class when isDarkMode is true', () => {
    applyDarkModePreference({ isDarkMode: true, hasStoredPreference: true });
    expect(document.documentElement.classList.contains('dark-mode')).toBe(true);
    expect(document.documentElement.classList.contains('light-mode')).toBe(false);
  });

  it('adds light-mode class when explicitly stored as light', () => {
    applyDarkModePreference({ isDarkMode: false, hasStoredPreference: true });
    expect(document.documentElement.classList.contains('dark-mode')).toBe(false);
    expect(document.documentElement.classList.contains('light-mode')).toBe(true);
  });

  it('adds neither class when following system default (no stored preference)', () => {
    applyDarkModePreference({ isDarkMode: false, hasStoredPreference: false });
    expect(document.documentElement.classList.contains('dark-mode')).toBe(false);
    expect(document.documentElement.classList.contains('light-mode')).toBe(false);
  });

  it('removes stale dark-mode class when switching to explicit light', () => {
    document.documentElement.classList.add('dark-mode');
    applyDarkModePreference({ isDarkMode: false, hasStoredPreference: true });
    expect(document.documentElement.classList.contains('dark-mode')).toBe(false);
    expect(document.documentElement.classList.contains('light-mode')).toBe(true);
  });
});
