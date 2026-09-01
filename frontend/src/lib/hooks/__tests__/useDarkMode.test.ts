import { renderHook, act, waitFor } from '@testing-library/react';
import { darkModeInitScript } from '../../darkMode';
import { useDarkMode } from '../useDarkMode';

describe('useDarkMode', () => {
  const changeHandlers: Array<(event: { matches: boolean }) => void> = [];

  const mockMatchMedia = (matches: boolean) => {
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: jest.fn().mockImplementation((query) => ({
        matches,
        media: query,
        onchange: null,
        addListener: jest.fn(),
        removeListener: jest.fn(),
        addEventListener: jest.fn(
          (_type: string, handler: (event: { matches: boolean }) => void) => {
            changeHandlers.push(handler);
          },
        ),
        removeEventListener: jest.fn(),
        dispatchEvent: jest.fn(),
      })),
    });
  };

  beforeEach(() => {
    changeHandlers.length = 0;
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    mockMatchMedia(false);
  });

  const getTheme = () => document.documentElement.getAttribute('data-theme');

  it('should initialize with light mode by default', async () => {
    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    expect(result.current.isDarkMode).toBe(false);
    expect(localStorage.getItem('darkMode')).toBeNull();
  });

  it('should follow system dark mode when no preference is stored, without pinning the theme', async () => {
    mockMatchMedia(true);

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    expect(result.current.isDarkMode).toBe(true);
    // Following the system must NOT look like an explicit choice.
    expect(getTheme()).toBeNull();
    expect(localStorage.getItem('darkMode')).toBeNull();
  });

  it('should toggle dark mode', async () => {
    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    act(() => {
      result.current.toggleDarkMode();
    });

    expect(result.current.isDarkMode).toBe(true);
  });

  it('should persist dark mode preference to localStorage', async () => {
    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    act(() => {
      result.current.toggleDarkMode();
    });

    expect(localStorage.getItem('darkMode')).toBe('true');
  });

  it('should load dark mode preference from localStorage', async () => {
    localStorage.setItem('darkMode', 'true');

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    expect(result.current.isDarkMode).toBe(true);
    expect(getTheme()).toBe('dark');
  });

  it('should restore stored light preference over system dark mode', async () => {
    mockMatchMedia(true);
    localStorage.setItem('darkMode', 'false');

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    expect(result.current.isDarkMode).toBe(false);
    expect(getTheme()).toBe('light');
  });

  it('should apply stored dark preference before React loads', () => {
    localStorage.setItem('darkMode', 'true');

    Function(darkModeInitScript)();

    expect(getTheme()).toBe('dark');
  });

  it('should apply stored light preference before React loads', () => {
    mockMatchMedia(true);
    localStorage.setItem('darkMode', 'false');

    Function(darkModeInitScript)();

    expect(getTheme()).toBe('light');
  });

  it('should not pin the theme before React loads when no preference is stored', () => {
    mockMatchMedia(true);

    Function(darkModeInitScript)();

    // Following the system: the CSS prefers-color-scheme media query drives
    // the theme, so the init script must leave the attribute untouched.
    expect(getTheme()).toBeNull();
  });

  // ------------------------------------------------------------------
  // Bug #1159 — lazy initializer reads DOM state on first render
  // ------------------------------------------------------------------

  it('reads data-theme="dark" from DOM and returns isDarkMode:true on the very first render (before isLoaded)', () => {
    // Simulate what the inline init script does before React hydrates.
    document.documentElement.setAttribute('data-theme', 'dark');

    const { result } = renderHook(() => useDarkMode());

    // isDarkMode must be true on the FIRST render, not only after useEffect.
    expect(result.current.isDarkMode).toBe(true);
  });

  it('reads data-theme="light" from DOM and returns isDarkMode:false on the very first render', () => {
    document.documentElement.setAttribute('data-theme', 'light');

    const { result } = renderHook(() => useDarkMode());

    expect(result.current.isDarkMode).toBe(false);
  });

  it('no stale icon flash: isDarkMode is already correct before isLoaded becomes true', async () => {
    // Init script applied data-theme="dark" before React renders.
    document.documentElement.setAttribute('data-theme', 'dark');

    const { result } = renderHook(() => useDarkMode());

    // Before any effects run, the state should already reflect dark mode.
    expect(result.current.isDarkMode).toBe(true);

    // After effects run and isLoaded is true, it should still be dark.
    await waitFor(() => expect(result.current.isLoaded).toBe(true));
    expect(result.current.isDarkMode).toBe(true);
  });

  it('falls back to getDarkModePreference when no data-theme attribute is set', async () => {
    // No attribute present — should fall back to system/storage preference.
    mockMatchMedia(false);

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));
    expect(result.current.isDarkMode).toBe(false);
  });

  it('toggleDarkMode still works correctly after lazy-init from DOM attribute', async () => {
    document.documentElement.setAttribute('data-theme', 'dark');

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    act(() => {
      result.current.toggleDarkMode();
    });

    expect(result.current.isDarkMode).toBe(false);
    expect(getTheme()).toBe('light');
    expect(localStorage.getItem('darkMode')).toBe('false');
  });

  it('should set data-theme="dark" when toggling on', async () => {
    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    act(() => {
      result.current.toggleDarkMode();
    });

    expect(getTheme()).toBe('dark');
  });

  it('should set data-theme="light" when toggling off', async () => {
    localStorage.setItem('darkMode', 'true');

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    act(() => {
      result.current.toggleDarkMode();
    });

    expect(result.current.isDarkMode).toBe(false);
    expect(getTheme()).toBe('light');
    expect(localStorage.getItem('darkMode')).toBe('false');
  });

  // ------------------------------------------------------------------
  // System-following vs explicit choice
  // ------------------------------------------------------------------

  it('follows OS theme changes live when no explicit choice is stored', async () => {
    mockMatchMedia(false);

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));
    expect(result.current.isDarkMode).toBe(false);

    // OS flips to dark — the hook reacts without a page reload.
    act(() => {
      changeHandlers.forEach((handler) => handler({ matches: true }));
    });

    expect(result.current.isDarkMode).toBe(true);
    // Still following the system: no explicit theme is pinned.
    expect(getTheme()).toBeNull();
    expect(localStorage.getItem('darkMode')).toBeNull();
  });

  it('does not register an OS-change listener when an explicit choice is stored', async () => {
    mockMatchMedia(true); // OS prefers dark
    localStorage.setItem('darkMode', 'false'); // but the user chose light

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    expect(result.current.isDarkMode).toBe(false);
    expect(getTheme()).toBe('light');

    // The stored choice must never be overridden by an OS-level change.
    expect(changeHandlers).toHaveLength(0);
  });
});
