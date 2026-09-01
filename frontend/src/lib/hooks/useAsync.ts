import { useCallback, useEffect, useRef, useState } from 'react';

export type AsyncStatus = 'idle' | 'loading' | 'success' | 'error';

interface UseAsyncState<T> {
  data: T | null;
  error: Error | null;
  status: AsyncStatus;
}

interface UseAsyncOptions {
  immediate?: boolean;
  /** Number of automatic retry attempts after a failure (default: 0, i.e. no auto-retry). */
  retries?: number;
  /**
   * Delay before each automatic retry, in ms. Accepts a fixed number or a
   * function of the attempt number (1-based) for pluggable backoff, e.g.
   * `attempt => attempt * 500` for linear backoff or
   * `attempt => 2 ** attempt * 100` for exponential backoff.
   */
  retryDelayMs?: number | ((attempt: number) => number);
}

function wait(ms: number, signal: AbortSignal): Promise<void> {
  if (ms <= 0) return Promise.resolve();
  return new Promise((resolve, reject) => {
    const timer = setTimeout(resolve, ms);
    signal.addEventListener('abort', () => {
      clearTimeout(timer);
      reject(new DOMException('Aborted', 'AbortError'));
    });
  });
}

export function useAsync<T>(
  asyncFunction: (signal: AbortSignal) => Promise<T>,
  options: UseAsyncOptions = {}
): UseAsyncState<T> & { retry: () => Promise<void> } {
  const { immediate, retries = 0, retryDelayMs = 0 } = options;

  const [state, setState] = useState<UseAsyncState<T>>({
    data: null,
    error: null,
    status: 'idle',
  });

  const abortControllerRef = useRef<AbortController | null>(null);
  const isMountedRef = useRef(true);

  const retry = useCallback(async () => {
    abortControllerRef.current?.abort();
    const controller = new AbortController();
    abortControllerRef.current = controller;

    if (!isMountedRef.current) return;
    // Stale-while-revalidating: keep the last-successful data on screen while
    // (re)fetching so a transient blip never blanks already-rendered values.
    setState((prev) => ({ ...prev, error: null, status: 'loading' }));

    let attempt = 0;
    for (;;) {
      try {
        const data = await asyncFunction(controller.signal);
        if (isMountedRef.current && !controller.signal.aborted) {
          setState({ data, error: null, status: 'success' });
        }
        return;
      } catch (error) {
        const isAbort = error instanceof DOMException && error.name === 'AbortError';
        if (isAbort || controller.signal.aborted) return;

        if (attempt < retries) {
          attempt += 1;
          const delay =
            typeof retryDelayMs === 'function' ? retryDelayMs(attempt) : retryDelayMs;
          try {
            await wait(delay, controller.signal);
          } catch {
            return;
          }
          continue;
        }

        if (isMountedRef.current) {
          const normalized = error instanceof Error ? error : new Error(String(error));
          setState((prev) => ({ ...prev, error: normalized, status: 'error' }));
        }
        return;
      }
    }
  }, [asyncFunction, retries, retryDelayMs]);

  useEffect(() => {
    isMountedRef.current = true;
    if (immediate) {
      void retry();
    }

    return () => {
      isMountedRef.current = false;
      abortControllerRef.current?.abort();
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [retry, immediate]);

  return { ...state, retry };
}
