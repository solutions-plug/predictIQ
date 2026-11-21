import { renderHook, act, waitFor } from '@testing-library/react';
import { useAsync } from '../useAsync';

function deferred<T>() {
  let resolve!: (v: T) => void;
  let reject!: (e: unknown) => void;
  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });
  return { promise, resolve, reject };
}

describe('hooks/useAsync', () => {
  it('does not update state or warn when unmounted mid-flight', async () => {
    const errorSpy = jest.spyOn(console, 'error').mockImplementation(() => {});
    const d = deferred<string>();
    let signal!: AbortSignal;
    const { result, unmount } = renderHook(() =>
      useAsync((s) => {
        signal = s;
        return d.promise;
      }),
    );
    expect(result.current.loading).toBe(true);

    unmount();
    expect(signal.aborted).toBe(true);
    await act(async () => {
      d.resolve('late');
    });

    expect(result.current.data).toBeNull();
    expect(result.current.loading).toBe(true);
    expect(errorSpy).not.toHaveBeenCalled();
    errorSpy.mockRestore();
  });

  it('applies only the latest result when an earlier call resolves late', async () => {
    const first = deferred<string>();
    const second = deferred<string>();
    const fn = jest.fn().mockReturnValueOnce(first.promise).mockReturnValueOnce(second.promise);

    const { result, rerender } = renderHook(({ q }) => useAsync(fn, [q]), {
      initialProps: { q: 'a' },
    });
    rerender({ q: 'b' });
    expect(fn).toHaveBeenCalledTimes(2);

    await act(async () => {
      second.resolve('new');
    });
    await waitFor(() => expect(result.current.data).toBe('new'));

    await act(async () => {
      first.resolve('stale');
    });
    expect(result.current.data).toBe('new');
  });

  it('sets the error state when the function rejects', async () => {
    const { result } = renderHook(() => useAsync(() => Promise.reject(new Error('boom'))));
    await waitFor(() => expect(result.current.loading).toBe(false));
    expect(result.current.error?.message).toBe('boom');
    expect(result.current.data).toBeNull();
  });
});
