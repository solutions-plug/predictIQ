/**
 * Shared, action-aware presentation for Soroban contract errors.
 *
 * `getContractErrorMessage` (admin-client.ts) maps a raw contract error code
 * to a generic, action-agnostic message. Several write flows (bet, create,
 * cancel, resolve) all surface error 101 (`NotAuthorized`) but each needs the
 * message to name the specific action the user attempted — without every
 * flow re-implementing its own copy of the error-101 special case.
 *
 * This module is the single place that decides what to show for a contract
 * error in a write flow. Components should call `getWriteActionErrorMessage`
 * (or render `<ApiError>` and pass it through `contractErrorFromApiError`)
 * instead of reading `CONTRACT_ERROR_MESSAGES` directly.
 */

import { ApiError, getContractErrorMessage } from './admin-client';

/** The write flows that route through the shared contract-error presentation. */
export type WriteAction = 'bet' | 'create' | 'cancel' | 'resolve';

const NOT_AUTHORIZED_CODE = 101;

const NOT_AUTHORIZED_MESSAGES: Record<WriteAction, string> = {
  bet: 'You are not authorized to place this bet.',
  create: 'You are not authorized to create this market.',
  cancel: 'You are not authorized to cancel this market.',
  resolve: 'You are not authorized to resolve this market.',
};

/**
 * Returns the display-ready message for a contract error raised while
 * performing `action`. Error 101 gets an action-specific "not authorized"
 * message; every other code falls back to the shared generic mapping so the
 * table in admin-client.ts stays the single source of truth for wording.
 */
export function getWriteActionErrorMessage(code: number, action: WriteAction): string {
  if (code === NOT_AUTHORIZED_CODE) {
    return NOT_AUTHORIZED_MESSAGES[action];
  }
  return getContractErrorMessage(code);
}

/**
 * Pulls the Soroban contract error code out of an `ApiError`, if present.
 * The API surfaces contract errors as `code: "CONTRACT_ERROR"` with the raw
 * numeric code in `details.contract_code`.
 */
export function getContractErrorCode(error: unknown): number | null {
  if (!(error instanceof ApiError)) return null;
  const raw = error.details?.contract_code;
  return typeof raw === 'number' ? raw : null;
}

/**
 * One-stop helper for write-flow catch blocks: given whatever was thrown and
 * the action being attempted, returns the message to show the user. Falls
 * back to the error's own message (or a generic string) for non-contract
 * errors so unrelated failures (network, validation) aren't mislabeled.
 */
export function describeWriteError(error: unknown, action: WriteAction): string {
  const contractCode = getContractErrorCode(error);
  if (contractCode !== null) {
    return getWriteActionErrorMessage(contractCode, action);
  }
  if (error instanceof Error) {
    return error.message;
  }
  return 'Something went wrong. Please try again.';
}

/** True when `error` is specifically the shared "not authorized" (101) contract error. */
export function isNotAuthorizedError(error: unknown): boolean {
  return getContractErrorCode(error) === NOT_AUTHORIZED_CODE;
}

// ---------------------------------------------------------------------------
// Short UI labels (#1338)
//
// `getContractErrorMessage` (admin-client.ts) gives the long explanatory
// sentence; this map gives the short label for compact surfaces (toasts,
// badges, table cells). Generated from the `Variant` column of
// docs/CONTRACT_ERRORS.md - a drift test keeps the two in lockstep.
// ---------------------------------------------------------------------------

const CONTRACT_ERROR_LABELS: Record<number, string> = {
  100: 'Already initialized',
  101: 'Not authorized',
  102: 'Market not found',
  103: 'Market closed',
  104: 'Market still active',
  105: 'Invalid outcome',
  106: 'Invalid bet amount',
  107: 'Insufficient balance',
  108: 'Oracle failure',
  109: 'Circuit breaker open',
  110: 'Dispute window closed',
  111: 'Voting not started',
  112: 'Voting ended',
  113: 'Already voted',
  114: 'Fee too high',
  115: 'Market not active',
  116: 'Deadline passed',
  117: 'Cannot change outcome',
  118: 'Market not disputed',
  119: 'Market not pending resolution',
  120: 'Admin not set',
  121: 'Contract paused',
  122: 'Guardian not set',
  123: 'Too many outcomes',
  124: 'Too many winners',
  125: 'Payout mode not supported',
  126: 'Insufficient deposit',
  127: 'Timelock active',
  128: 'Upgrade not initiated',
  129: 'Insufficient votes',
  130: 'Already voted on upgrade',
  131: 'Invalid wasm hash',
  132: 'Upgrade failed',
  133: 'Parent market not resolved',
  134: 'Parent market invalid outcome',
  135: 'Resolution not ready',
  136: 'Dispute window still open',
  137: 'No majority reached',
  138: 'Stale price',
  139: 'Confidence too low',
  140: 'Insufficient voting weight',
  141: 'Market not cancelled',
  142: 'Bet not found',
  143: 'Upgrade already pending',
  144: 'Upgrade hash in cooldown',
  145: 'Invalid amount',
  146: 'Governance token not set',
  147: 'Market not resolved',
  148: 'Invalid deadline',
  149: 'Pending transfer not found',
  150: 'Not pending owner',
  151: 'Token frozen',
  152: 'Migration validation error',
  153: 'Asset clawed back',
  154: 'Arithmetic overflow',
  155: 'Already claimed',
  156: 'No winnings',
  157: 'Invalid referrer',
  158: 'Resolution deadline passed',
  159: 'Overflow',
  160: 'Invalid time range',
};

export interface ContractError {
  code: number;
  /** Short label, e.g. "Market not found". */
  label: string;
  /** Full explanatory sentence for the user. */
  message: string;
}

/**
 * Resolve a numeric contract error code to a display-ready `{ label, message }`.
 * An unmapped code returns a generic-but-honest pair - never throws, never
 * yields `undefined`.
 */
export function getContractError(code: number): ContractError {
  return {
    code,
    label: CONTRACT_ERROR_LABELS[code] ?? 'Contract error',
    message: getContractErrorMessage(code),
  };
}

/** Numeric codes this module knows a label for. Used by the drift test. */
export function knownContractErrorCodes(): number[] {
  return Object.keys(CONTRACT_ERROR_LABELS).map(Number).sort((a, b) => a - b);
}
