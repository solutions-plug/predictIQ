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
 * Extracts a human-readable message from a failed HTTP response.
 *
 * Tries to parse the response body as JSON and pull out a `message` (or
 * `error`) field, falling back to the raw text, and finally to the status
 * text when the body is empty or not JSON.
 */
export async function readErrorMessage(
  response: Response,
  fallback?: string,
): Promise<string> {
  const statusText = fallback ?? `Request failed (${response.status})`;

  let body: string;
  try {
    body = await response.text();
  } catch {
    return statusText;
  }

  if (!body) {
    return statusText;
  }

  try {
    const parsed: unknown = JSON.parse(body);
    if (parsed && typeof parsed === 'object') {
      const record = parsed as Record<string, unknown>;
      const message = record.message ?? record.error;
      if (typeof message === 'string' && message.trim()) {
        return message;
      }
    }
  } catch {
    // Body is not JSON; fall through to the status text.
  }

  return statusText;
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
