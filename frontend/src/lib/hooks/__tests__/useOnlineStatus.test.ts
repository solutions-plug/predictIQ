import { act, renderHook } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { useOnlineStatus } from '../useOnlineStatus';

describe('useOnlineStatus', () => {
  const originalOnLine = navigator.onLine;

  const setOnLine = (value: boolean) => {
    Object.defineProperty(navigator, 'onLine', {
      configurable: true,
      get: () => value,
    });
  };

  beforeEach(() => {
    vi.useFakeTimers();
    setOnLine(true);
  });

  afterEach(() => {
    vi.runOnlyPendingTimers();
    vi.useRealTimers();
    Object.defineProperty(navigator, 'onLine', {
      configurable: true,
      get: () => originalOnLine,
    });
  });

  it('returns the initial online state', () => {
    const { result } = renderHook(() => useOnlineStatus());
    expect(result.current).toBe(true);
  });

  it('returns the initial offline state', () => {
    setOnLine(false);
    const { result } = renderHook(() => useOnlineStatus());
    expect(result.current).toBe(false);
  });

  it('debounces the offline transition', () => {
    const { result } = renderHook(() => useOnlineStatus());

    act(() => {
      setOnLine(false);
      window.dispatchEvent(new Event('offline'));
    });

    expect(result.current).toBe(true);

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(result.current).toBe(false);
  });

  it('debounces the online transition', () => {
    setOnLine(false);
    const { result } = renderHook(() => useOnlineStatus());

    act(() => {
      setOnLine(true);
      window.dispatchEvent(new Event('online'));
    });

    expect(result.current).toBe(false);

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(result.current).toBe(true);
  });

  it('cancels a pending transition when the state flips back', () => {
    const { result } = renderHook(() => useOnlineStatus());

    act(() => {
      setOnLine(false);
      window.dispatchEvent(new Event('offline'));
    });

    act(() => {
      setOnLine(true);
      window.dispatchEvent(new Event('online'));
    });

    act(() => {
      vi.advanceTimersByTime(1000);
    });

    expect(result.current).toBe(true);
  });

  it('removes event listeners on unmount', () => {
    const removeSpy = vi.spyOn(window, 'removeEventListener');
    const { unmount } = renderHook(() => useOnlineStatus());

    unmount();

    expect(removeSpy).toHaveBeenCalledWith('online', expect.any(Function));
    expect(removeSpy).toHaveBeenCalledWith('offline', expect.any(Function));
    removeSpy.mockRestore();
  });
});
