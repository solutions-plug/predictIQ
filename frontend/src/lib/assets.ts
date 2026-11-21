/**
 * Asset and amount helpers shared by the bet form and tests.
 */

/** Contract minimum bet amount (in whole units). */
export const MIN_BET_AMOUNT = 0.0000001;

/** Maximum number of decimal places the contract accepts. */
export const MAX_BET_DECIMALS = 7;

export type BetAmountErrorCode =
  | "EMPTY"
  | "EXPONENT_NOTATION"
  | "TOO_MANY_DECIMALS"
  | "BELOW_MINIMUM"
  | "NOT_A_NUMBER";

export interface BetAmountError {
  code: BetAmountErrorCode;
  message: string;
}

export type BetAmountResult =
  | { ok: true; value: number }
  | { ok: false; error: BetAmountError };

/**
 * Validate a bet amount string against the contract rules.
 *
 * Rejects exponent notation, more than seven decimals, and values below the
 * contract minimum. Returns a structured result so callers can surface the
 * specific message.
 */
export function validateBetAmount(input: string): BetAmountResult {
  const raw = (input ?? "").trim();

  if (raw.length === 0) {
    return {
      ok: false,
      error: { code: "EMPTY", message: "Enter a bet amount" },
    };
  }

  if (/[eE]/.test(raw)) {
    return {
      ok: false,
      error: {
        code: "EXPONENT_NOTATION",
        message: "Exponent notation is not allowed",
      },
    };
  }

  const decimalMatch = raw.match(/^\d*(\.(\d*))?$/);
  if (!decimalMatch) {
    return {
      ok: false,
      error: { code: "NOT_A_NUMBER", message: "Enter a valid number" },
    };
  }

  const fraction = decimalMatch[2] ?? "";
  if (fraction.length > MAX_BET_DECIMALS) {
    return {
      ok: false,
      error: {
        code: "TOO_MANY_DECIMALS",
        message: `Use at most ${MAX_BET_DECIMALS} decimal places`,
      },
    };
  }

  const value = Number(raw);
  if (!Number.isFinite(value)) {
    return {
      ok: false,
      error: { code: "NOT_A_NUMBER", message: "Enter a valid number" },
    };
  }

  if (value < MIN_BET_AMOUNT) {
    return {
      ok: false,
      error: {
        code: "BELOW_MINIMUM",
        message: `Amount must be at least ${MIN_BET_AMOUNT}`,
      },
    };
  }

  return { ok: true, value };
}
