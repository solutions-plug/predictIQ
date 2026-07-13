/**
 * Formatting helpers shared across the app surface.
 */

import type { Market } from './types';

export function formatXLM(amount: number, opts?: { compact?: boolean }): string {
  const value =
    opts?.compact && Math.abs(amount) >= 1000
      ? new Intl.NumberFormat('en', { notation: 'compact', maximumFractionDigits: 1 }).format(amount)
      : new Intl.NumberFormat('en', { maximumFractionDigits: 2 }).format(amount);
  return `${value} XLM`;
}

export function formatNumber(n: number, compact = false): string {
  return new Intl.NumberFormat('en', {
    notation: compact ? 'compact' : 'standard',
    maximumFractionDigits: compact ? 1 : 0,
  }).format(n);
}

export function formatDate(iso: string): string {
  return new Date(iso).toLocaleDateString('en', {
    year: 'numeric',
    month: 'short',
    day: 'numeric',
  });
}

/** Human "time left" until an ISO timestamp, e.g. "3d left", "Ended". */
export function timeUntil(iso: string): string {
  const ms = new Date(iso).getTime() - Date.now();
  if (ms <= 0) return 'Ended';
  const days = Math.floor(ms / 86_400_000);
  if (days >= 1) return `${days}d left`;
  const hours = Math.floor(ms / 3_600_000);
  if (hours >= 1) return `${hours}h left`;
  const mins = Math.max(1, Math.floor(ms / 60_000));
  return `${mins}m left`;
}

/** Implied probability (0-100) for an outcome from pool sizes. */
export function outcomeOdds(market: Market, outcomeId: number): number {
  const total = market.totalVolume;
  if (total <= 0) {
    // No liquidity yet: split evenly across outcomes.
    return Math.round(100 / Math.max(1, market.outcomes.length));
  }
  const pool = market.poolByOutcome[outcomeId] ?? 0;
  return Math.round((pool / total) * 100);
}

/** Shorten a Stellar address: GABC…WXYZ. */
export function shortAddress(address: string, chars = 4): string {
  if (address.length <= chars * 2 + 1) return address;
  return `${address.slice(0, chars)}…${address.slice(-chars)}`;
}
