/**
 * Rate-limit signal bus (#1339).
 *
 * `API_SPEC.md`: 1000 req/min per key; a 429 carries `Retry-After` (seconds). The
 * client reports every 429 here; the UI subscribes to show one shared countdown.
 *
 * Coalescing: while a cooldown window is active, further 429s from parallel requests
 * do not open a second window - they only extend the current one if their
 * `Retry-After` reaches further out. Listeners are notified once per window opening
 * or extension, never once per 429.
 */

type Listener = (secondsRemaining: number) => void;

const listeners = new Set<Listener>();
let activeUntil = 0; // epoch ms; 0 = no active cooldown

/** Subscribe to cooldown windows. Returns an unsubscribe function. */
export function onRateLimited(listener: Listener): () => void {
  listeners.add(listener);
  return () => {
    listeners.delete(listener);
  };
}

/** Seconds left in the current cooldown window, or 0 if none is active. */
export function rateLimitRemainingSeconds(): number {
  return Math.max(0, Math.ceil((activeUntil - Date.now()) / 1000));
}

/** True while a cooldown window is active. */
export function isRateLimited(): boolean {
  return rateLimitRemainingSeconds() > 0;
}

/**
 * Report a 429. Opens a cooldown window of `retryAfterSeconds`, or extends the
 * current one when the new window ends later. No-op when the current window
 * already covers it.
 */
export function reportRateLimited(retryAfterSeconds: number): void {
  const seconds = Number.isFinite(retryAfterSeconds) && retryAfterSeconds > 0
    ? Math.ceil(retryAfterSeconds)
    : 1;
  const until = Date.now() + seconds * 1000;
  if (until <= activeUntil) return; // already coalesced into the active window

  activeUntil = until;
  const remaining = rateLimitRemainingSeconds();
  for (const listener of listeners) listener(remaining);
}

/** Test helper: forget any active window and all listeners. */
export function _resetRateLimitForTests(): void {
  activeUntil = 0;
  listeners.clear();
}
