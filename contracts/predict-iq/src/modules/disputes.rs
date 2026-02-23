use crate::errors::ErrorCode;
use crate::modules::markets;
use crate::types::{MarketStatus, PayoutMode};
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
#[derive(Clone, Debug)]
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
    
    // Check if still within 24h dispute window
    let pending_ts = market.pending_resolution_timestamp.ok_or(ErrorCode::ResolutionNotReady)?;
    if e.ledger().timestamp() >= pending_ts + 86400 {
        return Err(ErrorCode::DisputeWindowClosed);
    }

    market.status = MarketStatus::Disputed;
    // Extend resolution deadline for voting period
    market.resolution_deadline += 86400 * 3; // 3 days extension
    let new_deadline = market.resolution_deadline;

    markets::update_market(e, market);

    // Emit standardized DisputeFiled event
    // Topics: [DisputeFiled, market_id, disciplinarian]
    crate::modules::events::emit_dispute_filed(e, market_id, disciplinarian, new_deadline);

    Ok(())
}

// Gas-optimized resolution with automatic payout mode selection
pub fn resolve_market(e: &Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    // Validate outcome
    if winning_outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }

    // Estimate winner count (in production, maintain a counter)
    let estimated_winners = estimate_winner_count(e, market_id, winning_outcome);

    // Automatically select payout mode based on winner count
    if estimated_winners > crate::types::MAX_PUSH_PAYOUT_WINNERS {
        market.payout_mode = PayoutMode::Pull;
    } else {
        market.payout_mode = PayoutMode::Push;
    }

    market.status = MarketStatus::Resolved;
    market.winning_outcome = Some(winning_outcome);

    markets::update_market(e, market);

    // Emit standardized ResolutionFinalized event
    // Topics: [ResolutionFinalized, market_id, resolver (admin)]
    let admin = crate::modules::admin::get_admin(e).unwrap_or(e.current_contract_address());
    crate::modules::events::emit_resolution_finalized(
        e,
        market_id,
        admin,
        winning_outcome,
        0, // Total payout tracked separately by indexer
    );

    Ok(())
}

// Helper function to estimate winner count without iterating all bets
fn estimate_winner_count(e: &Env, market_id: u64, outcome: u32) -> u32 {
    // In production, maintain a counter per outcome during bet placement
    // For now, use the tally weight as a proxy
    let tally = crate::modules::voting::get_tally(e, market_id, outcome);

    // Rough estimate: assume average bet is 100 units
    if tally > 0 {
        (tally / 100).max(1) as u32
    } else {
        0
    }
}

// Batch resolution metrics for monitoring
pub fn get_resolution_metrics(e: &Env, market_id: u64, outcome: u32) -> ResolutionMetrics {
    let winner_count = estimate_winner_count(e, market_id, outcome);
    let total_stake = crate::modules::voting::get_tally(e, market_id, outcome);

    // Estimate gas based on winner count
    // Base cost + (per-winner cost * count)
    let gas_estimate = 100_000 + (winner_count as u64 * 50_000);

    ResolutionMetrics {
        winner_count,
        total_winning_stake: total_stake,
        gas_estimate,
    }
}
