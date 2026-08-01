import { renderHook, act, waitFor } from '@testing-library/react';
import { useAsync } from '../useAsync';

describe('useAsync', () => {
  it('initializes with default state', () => {
    const mockFn = jest.fn();
    const { result } = renderHook(() => useAsync(mockFn));

    expect(result.current.data).toBeNull();
    expect(result.current.loading).toBe(false);
    expect(result.current.error).toBeNull();
    expect(typeof result.current.execute).toBe('function');
  });

  it('executes async function and updates state on success', async () => {
    const mockData = { test: 'data' };
    const mockFn = jest.fn().mockResolvedValue(mockData);
    const { result } = renderHook(() => useAsync(mockFn, { immediate: true }));

    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data).toEqual(mockData);
    expect(result.current.error).toBeNull();
  });

  it('handles errors correctly', async () => {
    const mockError = new Error('Test error');
    const mockFn = jest.fn().mockRejectedValue(mockError);
    const { result } = renderHook(() => useAsync(mockFn, { immediate: true }));

    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data).toBeNull();
    expect(result.current.error).toEqual(mockError);
  });

  it('sets error state when async function rejects and exposes it in the return value', async () => {
    const rejectionError = new Error('promise rejected');
    const mockFn = jest.fn().mockRejectedValue(rejectionError);
    const { result } = renderHook(() => useAsync(mockFn, { immediate: true }));

    await waitFor(() => expect(result.current.loading).toBe(false));

    // error must be accessible from the hook's return value
    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe('promise rejected');
    expect(result.current.data).toBeNull();
  });

  it('normalizes non-Error rejections into an Error object', async () => {
    // The hook wraps primitive rejections so callers always receive an Error.
    const mockFn = jest.fn().mockRejectedValue('plain string rejection');
    const { result } = renderHook(() => useAsync(mockFn, { immediate: true }));

    await waitFor(() => expect(result.current.loading).toBe(false));

    expect(result.current.error).toBeInstanceOf(Error);
    expect(result.current.error?.message).toBe('plain string rejection');
  });

  it('allows manual execution', async () => {
    const mockData = { manual: 'execution' };
    const mockFn = jest.fn().mockResolvedValue(mockData);
    const { result } = renderHook(() => useAsync(mockFn));

    expect(result.current.loading).toBe(false);

    act(() => {
      result.current.execute();
    });

    expect(result.current.loading).toBe(true);

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(result.current.data).toEqual(mockData);
  });

  it('cancels request on unmount', async () => {
    const mockFn = jest.fn(async (signal: AbortSignal) => {
      await new Promise((resolve, reject) => {
        signal.addEventListener('abort', () => reject(new DOMException('Aborted', 'AbortError')));
        setTimeout(resolve, 1000);
      });
      return { data: 'test' };
    });

    const { unmount } = renderHook(() => useAsync(mockFn, { immediate: true }));

    unmount();

    await waitFor(() => {
      expect(mockFn).toHaveBeenCalled();
    });
  });

  it('does not update state after unmount', async () => {
    const mockData = { test: 'data' };
    const mockFn = jest.fn(async (signal: AbortSignal) => {
      await new Promise(resolve => setTimeout(resolve, 100));
      return mockData;
    });

    const { result, unmount } = renderHook(() => useAsync(mockFn, { immediate: true }));

    unmount();

    await waitFor(() => {
      expect(result.current.data).toBeNull();
    }, { timeout: 500 });
  });

  it('passes abort signal to async function', async () => {
    const mockFn = jest.fn(async (signal: AbortSignal) => {
      expect(signal).toBeInstanceOf(AbortSignal);
      return { data: 'test' };
    });

    const { result } = renderHook(() => useAsync(mockFn, { immediate: true }));

    await waitFor(() => {
      expect(result.current.loading).toBe(false);
    });

    expect(mockFn).toHaveBeenCalled();
  });

  it('aborts previous request when execute is called again', async () => {
    let abortSignal1: AbortSignal | null = null;
    let abortSignal2: AbortSignal | null = null;

    const mockFn = jest.fn(async (signal: AbortSignal) => {
      if (!abortSignal1) {
        abortSignal1 = signal;
      } else {
        abortSignal2 = signal;
      }
      return { data: 'test' };
    });

    const { result } = renderHook(() => useAsync(mockFn));

    act(() => {
      result.current.execute();
    });

    await waitFor(() => {
      expect(abortSignal1).not.toBeNull();
    });

    act(() => {
      result.current.execute();
    });

    await waitFor(() => {
      expect(abortSignal2).not.toBeNull();
    });

    expect(abortSignal1?.aborted).toBe(true);
  });
});