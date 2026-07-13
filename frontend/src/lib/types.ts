/**
 * Shared domain types for the PredictIQ app surface.
 * These are the single source of truth consumed by every feature area.
 */

export type MarketStatus = 'open' | 'resolved' | 'closed';

export interface Outcome {
  /** Stable id, unique within a market (e.g. 0 = No, 1 = Yes). */
  id: number;
  label: string;
}

export interface Market {
  id: string;
  title: string;
  description: string;
  category: string;
  outcomes: Outcome[];
  /** outcomeId -> total XLM staked on that outcome. */
  poolByOutcome: Record<number, number>;
  /** Sum of all pools, in XLM. */
  totalVolume: number;
  /** ISO timestamp when betting closes. */
  endsAt: string;
  status: MarketStatus;
  /** Winning outcome id once resolved. */
  resolvedOutcome?: number | null;
  /** Wallet address of the creator. */
  createdBy?: string;
  createdAt: string;
}

export interface Bet {
  id: string;
  marketId: string;
  outcomeId: number;
  /** Stake in XLM. */
  amount: number;
  /** Wallet address of the bettor. */
  user: string;
  placedAt: string;
  claimed: boolean;
  /** Payout in XLM once the market resolves in the bettor's favour. */
  payout?: number | null;
  txHash?: string;
}

export interface CreateMarketInput {
  title: string;
  description: string;
  category: string;
  outcomes: string[];
  endsAt: string;
  createdBy?: string;
}

export interface PlaceBetInput {
  marketId: string;
  outcomeId: number;
  amount: number;
  user: string;
  txHash?: string;
}

export const MARKET_CATEGORIES = [
  'Crypto',
  'Politics',
  'Sports',
  'Economics',
  'Technology',
  'Culture',
] as const;

export type MarketCategory = (typeof MARKET_CATEGORIES)[number];
