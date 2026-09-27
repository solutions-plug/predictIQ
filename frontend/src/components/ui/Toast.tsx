'use client';

import React, { useEffect, useState } from 'react';
import {
  onRateLimited,
  rateLimitRemainingSeconds,
} from '../../lib/api/rateLimit';

// === useRateLimited hook

interface RateLimitState {
  isRateLimited: boolean;
  secondsRemaining: number;
}

/**
 * Subscribe to the shared rate-limit cooldown. Action buttons can disable
 * themselves while `isRateLimited` is true and re-enable at zero.
 */
export function useRateLimited(): RateLimitState {
  const [secondsRemaining, setSecondsRemaining] = useState<number>(() =>
    rateLimitRemainingSeconds(),
  );

  useEffect(() => {
    const unsubscribe = onRateLimited((seconds) => setSecondsRemaining(seconds));
    return unsubscribe;
  }, []);

  useEffect(() => {
    if (secondsRemaining <= 0) return;
    const id = setInterval(() => {
      setSecondsRemaining(rateLimitRemainingSeconds());
    }, 1000);
    return () => clearInterval(id);
  }, [secondsRemaining]);

  // Backgrounded tabs throttle setInterval, so the countdown can drift from the
  // real cooldown. Recompute immediately whenever the tab becomes visible again
  // instead of waiting for the next (possibly delayed) interval tick.
  useEffect(() => {
    if (typeof document === 'undefined') return;
    const handleVisibilityChange = () => {
      if (document.visibilityState === 'visible') {
        setSecondsRemaining(rateLimitRemainingSeconds());
      }
    };
    document.addEventListener('visibilitychange', handleVisibilityChange);
    return () =>
      document.removeEventListener('visibilitychange', handleVisibilityChange);
  }, []);

  return { isRateLimited: secondsRemaining > 0, secondsRemaining };
}

// === RateLimitToast
//
// One persistent (non-auto-dismissing) toast for the whole app. Coalescing is
// handled in rateLimit.ts, so mounting this once is enough - parallel 429s never
// stack a second toast.

export const RateLimitToast: React.FC = () => {
  const { isRateLimited, secondsRemaining } = useRateLimited();

  if (!isRateLimited) return null;

  return (
    <div
      className="toast toast--rate-limit"
      role="status"
      aria-live="polite"
    >
      <p className="toast__title">You are sending requests too quickly.</p>
      <p className="toast__body">
        Please wait{' '}
        <span className="toast__countdown" aria-label={`${secondsRemaining} seconds remaining`}>
          {secondsRemaining}s
        </span>{' '}
        before trying again.
      </p>
    </div>
  );
};

export default RateLimitToast;
