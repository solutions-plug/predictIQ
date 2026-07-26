/**
 * API client for admin/internal tooling: market resolution, blockchain
 * inspection, and email operations. Not imported by the landing page — see
 * `client.ts` for the small surface that app actually needs. Both share the
 * low-level request/retry/cache logic in `request.ts`.
 *
 * Run `npm run generate-client` to regenerate `schema.d.ts` after API changes.
 */

import { request, ApiError, CacheTag } from './request';
import { CACHE_TTL } from './cache';

export { ApiError };

/**
 * Maps Soroban contract error codes (u32) to localized user-facing messages.
 *
 * When the API returns a CONTRACT_ERROR, read `details.contract_code` and pass
 * it to `getContractErrorMessage` to get a display-ready string.
 *
 * Source of truth: contracts/predict-iq/src/errors.rs
 * Full reference:  docs/CONTRACT_ERRORS.md
 */
export const CONTRACT_ERROR_MESSAGES: Record<number, string> = {
  // Authorization & Setup
  100: "This contract has already been set up.",
  101: "You are not authorized to perform this action.",
  120: "No admin has been configured for this contract.",
  121: "The platform is currently paused. Please try again later.",
  122: "No guardian has been configured for this contract.",
  146: "The governance token contract has not been configured.",

  // Market Lifecycle
  102: "Market not found.",
  103: "This market is closed and no longer accepts activity.",
  104: "This market is still active and cannot be finalized yet.",
  115: "This market is not currently active.",
  116: "The deadline for this market has passed.",
  148: "The provided deadline is invalid.",

  // Betting
  105: "The selected outcome is not valid for this market.",
  106: "The bet amount is invalid. Please enter a valid amount.",
  107: "Insufficient balance to complete this transaction.",
  126: "Your deposit is below the minimum required amount.",
  142: "Bet not found.",
  145: "The amount provided is invalid.",

  // Resolution & Disputes
  108: "The oracle failed to provide a result. Please try again later.",
  110: "The dispute window for this market has closed.",
  117: "The outcome for this market has already been set.",
  118: "This market is not in a disputed state.",
  119: "This market is not pending resolution.",
  133: "The parent market has not been resolved yet.",
  134: "The parent market outcome does not satisfy this market's condition.",
  135: "Resolution conditions have not been met yet. Please try again later.",
  136: "The dispute window is still open. Resolution must wait.",
  137: "No majority outcome was reached. Resolution is inconclusive.",
  138: "Price data is stale. A fresh oracle feed is required.",
  139: "Oracle confidence is too low to resolve this market.",
  141: "This market was not cancelled.",
  147: "This market has not been resolved yet.",

  // Voting & Governance
  111: "Voting on this market has not started yet.",
  112: "The voting period for this market has ended.",
  113: "You have already voted on this market.",
  114: "The requested fee is too high.",
  129: "Not enough governance votes to approve this action.",
  130: "You have already voted on this upgrade.",
  140: "Your governance token balance is too low to vote.",

  // Upgrades
  127: "A timelock is active. Please wait before retrying.",
  128: "No upgrade has been initiated.",
  131: "The provided WASM hash is invalid.",
  132: "The contract upgrade failed.",
  143: "An upgrade is already pending. Only one upgrade can be in progress at a time.",
  144: "This WASM hash is in cooldown. Please wait before reusing it.",

  // System
  109: "The system circuit breaker is open. Operations are temporarily halted.",
  123: "Too many outcomes provided for this market.",
  124: "Too many winners specified for payout calculation.",
  125: "This payout mode is not supported.",
};

/**
 * Returns a user-facing message for a contract error code.
 * Falls back to a generic message if the code is not recognized.
 */
export function getContractErrorMessage(code: number): string {
  return CONTRACT_ERROR_MESSAGES[code] ?? `An unexpected contract error occurred (code ${code}).`;
}

export const api = {
  health: (signal?: AbortSignal) => request<string>("GET", "/health", { signal }),

  getFeaturedMarkets: (signal?: AbortSignal) =>
    request<
      Array<{
        id: number;
        title: string;
        volume: number;
        ends_at: string;
        onchain_volume: string;
        resolved_outcome?: number | null;
      }>
    >("GET", "/api/markets/featured", {
      cacheTtl: CACHE_TTL.SHORT,
      cacheTags: [CacheTag.MARKETS],
      signal,
    }),

  getContent: (params?: { page?: number; page_size?: number }, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/content", { params, cacheTtl: CACHE_TTL.MEDIUM, signal }),

  // Blockchain
  getBlockchainHealth: (signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/blockchain/health", {
      cacheTtl: CACHE_TTL.SHORT,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  getBlockchainMarket: (marketId: number | string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/markets/${marketId}`, {
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.BLOCKCHAIN, CacheTag.MARKETS],
      signal,
    }),

  getBlockchainStats: (signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/blockchain/stats", {
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  getUserBets: (user: string, params?: { page?: number; page_size?: number }, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/users/${user}/bets`, {
      params,
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  getOracleResult: (marketId: number | string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/oracle/${marketId}`, {
      cacheTtl: CACHE_TTL.LONG,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  getTransactionStatus: (txHash: string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/blockchain/tx/${txHash}`, {
      cacheTtl: CACHE_TTL.LONG,
      cacheTags: [CacheTag.BLOCKCHAIN],
      signal,
    }),

  // Newsletter (admin/lifecycle operations beyond the landing page's signup form)
  newsletterConfirm: (token: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("GET", `/api/v1/newsletter/confirm`, {
      params: { token },
      cacheTags: [CacheTag.NEWSLETTER],
      signal,
    }),

  newsletterUnsubscribe: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/unsubscribe", {
      body: { email },
      cacheTags: [CacheTag.NEWSLETTER, CacheTag.STATISTICS],
      signal,
    }),

  newsletterGdprExport: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; data: Record<string, unknown> }>(
      "GET",
      "/api/v1/newsletter/gdpr/export",
      { params: { email }, cacheTags: [CacheTag.NEWSLETTER], signal }
    ),

  newsletterGdprDelete: (email: string, signal?: AbortSignal) =>
    request<{ success: boolean; message: string }>("DELETE", "/api/v1/newsletter/gdpr/delete", {
      body: { email },
      cacheTags: [CacheTag.NEWSLETTER],
      signal,
    }),

  // Admin / email
  resolveMarket: (marketId: number | string, signal?: AbortSignal) =>
    request<{ invalidated_keys: number }>("POST", `/api/markets/${marketId}/resolve`, {
      cacheTags: [CacheTag.MARKETS, CacheTag.BLOCKCHAIN, CacheTag.STATISTICS],
      signal,
    }),

  emailPreview: (templateName: string, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", `/api/v1/email/preview/${templateName}`, {
      cacheTtl: CACHE_TTL.LONG,
      cacheTags: [CacheTag.EMAIL],
      signal,
    }),

  emailSendTest: (body: { recipient: string; template_name: string }, signal?: AbortSignal) =>
    request<{ success: boolean; message: string; message_id: string }>(
      "POST",
      "/api/v1/email/test",
      { body, cacheTags: [CacheTag.EMAIL], signal }
    ),

  getEmailAnalytics: (params?: { template_name?: string; days?: number }, signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/v1/email/analytics", {
      params,
      cacheTtl: CACHE_TTL.MEDIUM,
      cacheTags: [CacheTag.EMAIL],
      signal,
    }),

  getEmailQueueStats: (signal?: AbortSignal) =>
    request<Record<string, unknown>>("GET", "/api/v1/email/queue/stats", {
      cacheTtl: CACHE_TTL.SHORT,
      cacheTags: [CacheTag.EMAIL],
      signal,
    }),
};
