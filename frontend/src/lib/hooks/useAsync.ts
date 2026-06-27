import { useState, useEffect, useCallback, useRef } from 'react';

interface UseAsyncState<T> {
  data: T | null;
  loading: boolean;
  error: Error | null;
}

interface UseAsyncOptions {
  immediate?: boolean;
}

export function useAsync<T>(
  asyncFunction: (signal: AbortSignal) => Promise<T>,
  options: UseAsyncOptions = {}
): UseAsyncState<T> & { execute: () => Promise<void> } {
  const [state, setState] = useState<UseAsyncState<T>>({
    data: null,
    loading: false,
    error: null,
  });

  const abortControllerRef = useRef<AbortController | null>(null);
  const isMountedRef = useRef(true);

  const execute = useCallback(async () => {
    abortControllerRef.current?.abort();
    abortControllerRef.current = new AbortController();

    if (!isMountedRef.current) return;
    setState(prev => ({ ...prev, loading: true, error: null }));

    try {
      const data = await asyncFunction(abortControllerRef.current.signal);
      if (isMountedRef.current) {
        setState({ data, loading: false, error: null });
      }
    } catch (error) {
      if (isMountedRef.current && !(error instanceof DOMException && error.name === 'AbortError')) {
        const normalized = error instanceof Error ? error : new Error(String(error));
        setState({ data: null, loading: false, error: normalized });
      }
    }
  }, [asyncFunction]);

  useEffect(() => {
    isMountedRef.current = true;
    if (options.immediate) {
      execute();
    }

    return () => {
      isMountedRef.current = false;
      abortControllerRef.current?.abort();
    };
  }, [execute, options.immediate]);

  return { ...state, execute };
}