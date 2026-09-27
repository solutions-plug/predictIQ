import { useCallback, useRef, useState } from 'react';

/**
 * Explicit transaction lifecycle shared by all write flows (bet, create,
 * resolve, cancel) so each one doesn't reinvent its own ad hoc
 * loading/error state.
 *
 *   idle -> pending -> signed -> submitted -> confirmed
 *                    \-> rejected (user declined the signature — not an error)
 *        -> pending -> failed (signing or submission raised an error)
 */
export type TransactionStatus =
  | 'idle'
  | 'pending'
  | 'signed'
  | 'submitted'
  | 'confirmed'
  | 'failed'
  | 'rejected';

export interface UseTransactionState {
  status: TransactionStatus;
  error: Error | null;
  txHash: string | null;
}

export interface TransactionHandlers<T> {
  /** Prompts the wallet to sign the transaction and returns the signed payload. */
  sign: () => Promise<T>;
  /** Submits the signed payload to the network and returns the tx hash. */
  submit: (signed: T) => Promise<string>;
  /** Optional: waits for network confirmation of `txHash`. */
  confirm?: (txHash: string) => Promise<void>;
}

/**
 * Structured error codes/names wallets use to signal a user-declined
 * signature. Checked first so detection doesn't depend on message wording.
 */
const REJECTION_CODES = [
  'userrejected',
  'userrejectedrequest',
  'userdeclined',
  'userdeclinedrequest',
  'rejectedbyuser',
  'declinedbyuser',
  'actionrejected',
  'requestrejected',
];

/** Error names/messages wallets commonly use for a user-declined signature. */
const REJECTION_PATTERNS = [
  'user declined',
  'user rejected',
  'declined access',
  'rejected the request',
  'not allowed',
];

/**
 * Best-effort detection of "the user rejected the signature prompt" versus a
 * genuine failure.
 *
 * Detection prefers a structured signal — an error `code` or `name` exposed
 * by the wallet API (e.g. `UserRejectedRequestError`) — so it keeps working
 * regardless of the message language or phrasing. Only when no such
 * structured signal is present do we fall back to matching the message text
 * against a fixed list of English substrings; that fallback is inherently
 * brittle: a wallet that localizes its messages, or phrases rejection
 * differently, will not be recognized and the cancel will be misclassified
 * as a hard failure. This is intentionally checked before anything is
 * treated as a hard failure.
 */
export function isUserRejection(error: unknown): boolean {
  if (error && typeof error === 'object') {
    const { code, name } = error as { code?: unknown; name?: unknown };
    const signals = [code, name]
      .filter((value): value is string | number =>
        typeof value === 'string' || typeof value === 'number',
      )
      .map((value) => String(value).toLowerCase());
    if (signals.some((signal) => REJECTION_CODES.includes(signal))) {
      return true;
    }
  }

  const message = error instanceof Error ? error.message : String(error);
  const normalized = message.toLowerCase();
  return REJECTION_PATTERNS.some((pattern) => normalized.includes(pattern));
}

const INITIAL_STATE: UseTransactionState = { status: 'idle', error: null, txHash: null };

/**
 * Drives a single write transaction through pending -> signed -> submitted
 * -> confirmed/failed. A rejected signature resets cleanly back to `idle`
 * (with no `error` set) rather than landing on `failed`, so callers should
 * not show an error toast for it — check `status === 'rejected'` if a
 * distinct "you cancelled" hint is wanted.
 */
export function useTransaction<T>() {
  const [state, setState] = useState<UseTransactionState>(INITIAL_STATE);
  const runIdRef = useRef(0);

  const reset = useCallback(() => {
    runIdRef.current += 1;
    setState(INITIAL_STATE);
  }, []);

  const run = useCallback(async (handlers: TransactionHandlers<T>) => {
    const runId = ++runIdRef.current;
    const isStale = () => runIdRef.current !== runId;

    setState({ status: 'pending', error: null, txHash: null });

    let signed: T;
    try {
      signed = await handlers.sign();
    } catch (err) {
      if (isStale()) return;
      if (isUserRejection(err)) {
        // Routine user choice — reset to pre-submission state, no error toast.
        setState(INITIAL_STATE);
      } else {
        const error = err instanceof Error ? err : new Error(String(err));
        setState({ status: 'failed', error, txHash: null });
      }
      return;
    }

    if (isStale()) return;
    setState({ status: 'signed', error: null, txHash: null });

    let txHash: string;
    try {
      txHash = await handlers.submit(signed);
    } catch (err) {
      if (isStale()) return;
      const error = err instanceof Error ? err : new Error(String(err));
      setState({ status: 'failed', error, txHash: null });
      return;
    }

    if (isStale()) return;
    setState({ status: 'submitted', error: null, txHash });

    if (handlers.confirm) {
      try {
        await handlers.confirm(txHash);
      } catch (err) {
        if (isStale()) return;
        const error = err instanceof Error ? err : new Error(String(err));
        setState({ status: 'failed', error, txHash });
        return;
      }
    }

    if (isStale()) return;
    setState({ status: 'confirmed', error: null, txHash });
  }, []);

  return { ...state, run, reset };
}
