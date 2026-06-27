import { renderHook, act, waitFor } from '@testing-library/react';
import { darkModeInitScript } from '../../darkMode';
import { useDarkMode } from '../useDarkMode';

describe('useDarkMode', () => {
  const mockMatchMedia = (matches: boolean) => {
    Object.defineProperty(window, 'matchMedia', {
      writable: true,
      value: jest.fn().mockImplementation((query) => ({
        matches,
        media: query,
        onchange: null,
        addListener: jest.fn(),
        removeListener: jest.fn(),
        addEventListener: jest.fn(),
        removeEventListener: jest.fn(),
        dispatchEvent: jest.fn(),
      })),
    });
  };

  beforeEach(() => {
    localStorage.clear();
    document.documentElement.classList.remove('dark-mode');
    document.documentElement.classList.remove('light-mode');
    mockMatchMedia(false);
  });

  it('should initialize with light mode by default', async () => {
    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    expect(result.current.isDarkMode).toBe(false);
    expect(localStorage.getItem('darkMode')).toBeNull();
  });

  it('should use system dark mode when no preference is stored', async () => {
    mockMatchMedia(true);

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    expect(result.current.isDarkMode).toBe(true);
    expect(document.documentElement.classList.contains('dark-mode')).toBe(true);
    expect(document.documentElement.classList.contains('light-mode')).toBe(false);
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
    expect(document.documentElement.classList.contains('dark-mode')).toBe(true);
    expect(document.documentElement.classList.contains('light-mode')).toBe(false);
  });

  it('should restore stored light preference over system dark mode', async () => {
    mockMatchMedia(true);
    localStorage.setItem('darkMode', 'false');

    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));

    expect(result.current.isDarkMode).toBe(false);
    expect(document.documentElement.classList.contains('dark-mode')).toBe(false);
    expect(document.documentElement.classList.contains('light-mode')).toBe(true);
  });

  it('should apply stored dark preference before React loads', () => {
    localStorage.setItem('darkMode', 'true');

    Function(darkModeInitScript)();

    expect(document.documentElement.classList.contains('dark-mode')).toBe(true);
    expect(document.documentElement.classList.contains('light-mode')).toBe(false);
  });

  it('should apply stored light preference before React loads', () => {
    mockMatchMedia(true);
    localStorage.setItem('darkMode', 'false');

    Function(darkModeInitScript)();

    expect(document.documentElement.classList.contains('dark-mode')).toBe(false);
    expect(document.documentElement.classList.contains('light-mode')).toBe(true);
  });

  it('should add dark-mode class to document element', async () => {
    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));
    
    act(() => {
      result.current.toggleDarkMode();
    });
    
    expect(document.documentElement.classList.contains('dark-mode')).toBe(true);
  });

  it('should remove dark-mode class when toggling off', async () => {
    localStorage.setItem('darkMode', 'true');
    
    const { result } = renderHook(() => useDarkMode());

    await waitFor(() => expect(result.current.isLoaded).toBe(true));
    
    act(() => {
      result.current.toggleDarkMode();
    });
    
    expect(document.documentElement.classList.contains('dark-mode')).toBe(false);
    expect(document.documentElement.classList.contains('light-mode')).toBe(true);
    expect(localStorage.getItem('darkMode')).toBe('false');
  });
});
