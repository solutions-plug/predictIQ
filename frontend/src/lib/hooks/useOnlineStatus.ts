'use client';

import { useEffect, useRef, useState } from 'react';

/** How long a connectivity change must hold before the UI reacts to it. */
const DEBOUNCE_MS = 500;

/**
 * Tracks browser connectivity via `navigator.onLine` plus the `online`/
 * `offline` events, debounced so a flaky connection rapidly toggling state
 * doesn't flicker dependent UI (e.g. OfflineBanner) on and off repeatedly.
 *
 * SSR-safe: `navigator` is undefined on the server, so this starts optimistic
 * (online) and only diverges once mounted in the browser.
 */
export function useOnlineStatus(): boolean {
  const [isOnline, setIsOnline] = useState(
    () => typeof navigator === 'undefined' || navigator.onLine,
  );
  const timeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const applyDebounced = (value: boolean) => {
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
      timeoutRef.current = setTimeout(() => setIsOnline(value), DEBOUNCE_MS);
    };

    const handleOnline = () => applyDebounced(true);
    const handleOffline = () => applyDebounced(false);

    window.addEventListener('online', handleOnline);
    window.addEventListener('offline', handleOffline);
    // Reconcile with the real state in case it changed before this effect ran.
    setIsOnline(navigator.onLine);

    return () => {
      window.removeEventListener('online', handleOnline);
      window.removeEventListener('offline', handleOffline);
      if (timeoutRef.current) clearTimeout(timeoutRef.current);
    };
  }, []);

  return isOnline;
}
