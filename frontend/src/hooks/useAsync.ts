import { useCallback, useEffect, useRef, useState } from 'react';

export interface UseAsyncState<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
}

export interface UseAsyncResult<T> extends UseAsyncState<T> {
  retry: () => void;
}

/**
 * Runs an async function and tracks its loading/error/data state.
 *
 * `retry()` is a no-op while a request is already in flight, so rapid
 * repeated calls (e.g. double-clicking a retry button) only ever produce
 * a single network request.
 */
export function useAsync<T>(
  fn: (signal: AbortSignal) => Promise<T>,
  deps: unknown[] = [],
): UseAsyncResult<T> {
  const [state, setState] = useState<UseAsyncState<T>>({
    data: null,
    loading: true,
    error: null,
  });

  const controllerRef = useRef<AbortController | null>(null);
  const inFlightRef = useRef(false);
  const mountedRef = useRef(true);
  const fnRef = useRef(fn);
  fnRef.current = fn;

  const run = useCallback(() => {
    // Guard against overlapping requests: while one is in flight, further
    // invocations are ignored rather than starting a new network call.
    if (inFlightRef.current) {
      return;
    }

    controllerRef.current?.abort();
    const controller = new AbortController();
    controllerRef.current = controller;
    inFlightRef.current = true;

    setState((prev) => ({ ...prev, loading: true, error: null }));

    fnRef
      .current(controller.signal)
      .then((data) => {
        if (!mountedRef.current || controller.signal.aborted) return;
        setState({ data, loading: false, error: null });
      })
      .catch((error: unknown) => {
        if (!mountedRef.current || controller.signal.aborted) return;
        setState({
          data: null,
          loading: false,
          error: error instanceof Error ? error : new Error(String(error)),
        });
      })
      .finally(() => {
        if (controllerRef.current === controller) {
          inFlightRef.current = false;
        }
      });
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    run();

    return () => {
      mountedRef.current = false;
      controllerRef.current?.abort();
      inFlightRef.current = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps);

  return { ...state, retry: run };
}
