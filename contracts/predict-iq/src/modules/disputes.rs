use crate::errors::ErrorCode;
use crate::modules::{markets, resolution};
use crate::types::{ConfigKey, MarketStatus};
use soroban_sdk::{Address, Env};

#[contracttype]
#[derive(Clone)]
pub struct ResolutionMetrics {
    pub winner_count: u32,
    pub total_winning_stake: i128,
    pub gas_estimate: u64,
}

pub fn file_dispute(e: &Env, disciplinarian: Address, market_id: u64) -> Result<(), ErrorCode> {
    disciplinarian.require_auth();

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::PendingResolution {
        return Err(ErrorCode::MarketNotPendingResolution);
    }

    let pending_ts = market
        .pending_resolution_timestamp
        .ok_or(ErrorCode::ResolutionNotReady)?;
    if e.ledger().timestamp() >= pending_ts + resolution::get_dispute_window(e) {
        // Issue #8: window is now configurable (default 72h)
        return Err(ErrorCode::DisputeWindowClosed);
    }

    market.status = MarketStatus::Disputed;
    market.dispute_timestamp = Some(e.ledger().timestamp());
    // Extend resolution deadline by the full dispute window duration
    market.resolution_deadline += resolution::get_dispute_window(e);
    let new_deadline = market.resolution_deadline;

    markets::update_market(e, market);

    crate::modules::events::emit_dispute_filed(e, market_id, disciplinarian, new_deadline);

    Ok(())
}

/// Issue #23: payout_mode is immutable after creation — never mutated here.
/// Issue #24: Use actual winner_counts instead of heuristic.
/// Issue #35: Calculate and emit actual total payout.
pub fn resolve_market(e: &Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if winning_outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }

    // payout_mode is intentionally NOT mutated here — it is fixed at creation
    // time and must remain stable throughout PendingResolution and Disputed
    // phases so that gas and distribution path calculations are consistent.

    market.status = MarketStatus::Resolved;
    market.winning_outcome = Some(winning_outcome);
    market.resolved_at = Some(e.ledger().timestamp());

    // Issue #35: Calculate actual total payout for the event
    let winning_stake = market.outcome_stakes.get(winning_outcome).unwrap_or(0);
    let total_payout = if winning_stake > 0 {
        market.total_staked
    } else {
        0
    };

    markets::update_market(e, market);

    let admin = crate::modules::admin::get_admin(e).unwrap_or(e.current_contract_address());
    crate::modules::events::emit_resolution_finalized(
        e,
        market_id,
        admin,
        winning_outcome,
        total_payout, // Issue #35: actual payout, not hardcoded 0
    );

    Ok(())
}

pub fn set_max_push_payout_winners(e: &Env, threshold: u32) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::MaxPushPayoutWinners, &threshold);
    e.storage().persistent().extend_ttl(
        &ConfigKey::MaxPushPayoutWinners,
        crate::types::GOV_TTL_LOW_THRESHOLD,
        crate::types::GOV_TTL_HIGH_THRESHOLD,
    );
    Ok(())
}

pub fn get_max_push_payout_winners(e: &Env) -> u32 {
    e.storage()
        .persistent()
        .get(&ConfigKey::MaxPushPayoutWinners)
        .unwrap_or(crate::types::MAX_PUSH_PAYOUT_WINNERS)
}

// Batch resolution metrics for monitoring
pub fn get_resolution_metrics(e: &Env, market_id: u64, outcome: u32) -> ResolutionMetrics {
    let winner_count = markets::count_bets_for_outcome(e, market_id, outcome);
    let total_stake = match markets::get_market(e, market_id) {
        Some(m) => m.outcome_stakes.get(outcome).unwrap_or(0),
        None => 0,
    };

    let gas_estimate = 100_000 + (winner_count as u64 * 50_000);

    ResolutionMetrics {
        winner_count,
        total_winning_stake: total_stake,
        gas_estimate,
    }
}
