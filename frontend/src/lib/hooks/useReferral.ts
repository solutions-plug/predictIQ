import { useEffect, useState } from 'react';
import { usePathname, useSearchParams } from 'next/navigation';

/**
 * Referral-code capture (#1374).
 *
 * A referral code arrives on a URL as `?ref=<code>` (e.g. a link shared by an
 * existing user). It needs to survive client-side navigation — a visitor may
 * land on `/`, then navigate to `/markets/create` or `/signup` several clicks
 * later — so it is persisted to `localStorage` rather than kept only in memory
 * or forwarded as a query param on every internal link.
 *
 * Persistence policy — explicit "last-touch-wins":
 *   - A `ref` param present in the URL always overwrites whatever referral
 *     code (if any) was previously stored, together with a fresh
 *     `capturedAt` timestamp. This is deliberate: the most recent link a
 *     visitor actually clicked is treated as the attribution source.
 *   - Visiting a page WITHOUT a `ref` param never clears or touches the
 *     stored code. A referral captured earlier in the session must keep
 *     working right up until the visitor follows a *different* referral
 *     link (or a market-creation/signup transaction attaches and the
 *     caller explicitly clears it via `clearStoredReferralCode`).
 *
 * This intentionally means a code from an old session persists indefinitely
 * across visits until overwritten or explicitly cleared — there is no TTL.
 * That call is left to the caller / a future product decision; the module
 * only guarantees the overwrite-only-on-new-code contract described above.
 */

export const REFERRAL_STORAGE_KEY = 'predictiq:referral_code';
export const REFERRAL_QUERY_PARAM = 'ref';

export interface StoredReferral {
  code: string;
  capturedAt: number;
}

function isBrowser(): boolean {
  return typeof window !== 'undefined';
}

/** Reads the persisted referral record, or `null` if none is stored / storage is unavailable. */
export function getStoredReferral(): StoredReferral | null {
  if (!isBrowser()) return null;

  try {
    const raw = window.localStorage.getItem(REFERRAL_STORAGE_KEY);
    if (!raw) return null;

    const parsed = JSON.parse(raw) as Partial<StoredReferral>;
    if (typeof parsed.code !== 'string' || !parsed.code) return null;

    return {
      code: parsed.code,
      capturedAt: typeof parsed.capturedAt === 'number' ? parsed.capturedAt : 0,
    };
  } catch {
    // Corrupt entry or storage access denied (private browsing, etc.) — treat as absent.
    return null;
  }
}

/** Convenience accessor returning just the code string, or `null`. */
export function getStoredReferralCode(): string | null {
  return getStoredReferral()?.code ?? null;
}

/** Persists a referral code, overwriting any previously stored one (last-touch-wins). */
export function setStoredReferralCode(code: string): void {
  if (!isBrowser() || !code) return;

  try {
    const record: StoredReferral = { code, capturedAt: Date.now() };
    window.localStorage.setItem(REFERRAL_STORAGE_KEY, JSON.stringify(record));
  } catch {
    // Ignore storage failures (quota exceeded, disabled storage, etc.).
  }
}

/** Removes the stored referral code, e.g. after it has been successfully attached to a transaction. */
export function clearStoredReferralCode(): void {
  if (!isBrowser()) return;

  try {
    window.localStorage.removeItem(REFERRAL_STORAGE_KEY);
  } catch {
    // Ignore storage failures.
  }
}

/**
 * Reads the `ref` query param from the current URL and, if present, persists
 * it (last-touch-wins). Returns the effective stored code afterwards, or
 * `null` if none has ever been captured. Safe to call on every navigation.
 */
export function captureReferralFromLocation(): string | null {
  if (!isBrowser()) return null;

  try {
    const params = new URLSearchParams(window.location.search);
    const incoming = params.get(REFERRAL_QUERY_PARAM);
    if (incoming) {
      setStoredReferralCode(incoming);
    }
  } catch {
    // Malformed URL — fall through and return whatever is already stored.
  }

  return getStoredReferralCode();
}

/**
 * Merges the stored referral code into a transaction payload without
 * clobbering a `referral_code` the caller already set explicitly.
 */
export function attachReferralCode<T extends Record<string, unknown>>(
  payload: T
): T & { referral_code?: string } {
  if ('referral_code' in payload && payload['referral_code']) {
    return payload as T & { referral_code?: string };
  }

  const code = getStoredReferralCode();
  if (!code) return payload as T & { referral_code?: string };

  return { ...payload, referral_code: code };
}

/**
 * React hook: captures a `ref` query param on mount and re-captures whenever
 * the caller's route/search params change (client-side navigation), and
 * exposes the effective referral code plus an `attach` helper for building
 * transaction payloads (market creation, bet placement, signup, ...).
 */
export function useReferral(): {
  referralCode: string | null;
  attach: <T extends Record<string, unknown>>(payload: T) => T & { referral_code?: string };
  clear: () => void;
} {
  const [referralCode, setReferralCode] = useState<string | null>(null);
  const pathname = usePathname();
  const searchParams = useSearchParams();

  useEffect(() => {
    setReferralCode(captureReferralFromLocation());
  }, [pathname, searchParams]);

  return {
    referralCode,
    attach: attachReferralCode,
    clear: () => {
      clearStoredReferralCode();
      setReferralCode(null);
    },
  };
}
