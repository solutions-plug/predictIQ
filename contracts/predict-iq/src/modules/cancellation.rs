use crate::errors::ErrorCode;
use crate::modules::{admin, markets};
use crate::types::{MarketStatus, CANCEL_OUTCOME_INDEX};
use soroban_sdk::{Address, Env, Symbol};

const FAILED_MARKET_THRESHOLD_BPS: i128 = 7500; // 75% vote required to cancel

/// Admin override to cancel a market
pub fn cancel_market_admin(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status == MarketStatus::Resolved || market.status == MarketStatus::Cancelled {
        return Err(ErrorCode::CannotChangeOutcome);
    }

    market.status = MarketStatus::Cancelled;
    markets::update_market(e, market);

    e.events()
        .publish((Symbol::new(e, "market_cancelled"), market_id), ());

    Ok(())
}

/// Community vote to cancel a market (requires 75% threshold)
pub fn cancel_market_vote(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Disputed {
        return Err(ErrorCode::MarketNotDisputed);
    }

    // Calculate if cancellation threshold is met
    let cancel_votes = crate::modules::voting::get_tally(e, market_id, CANCEL_OUTCOME_INDEX);
    let mut total_votes = cancel_votes;

    for outcome in 0..market.options.len() {
        total_votes += crate::modules::voting::get_tally(e, market_id, outcome);
    }

    if total_votes == 0 {
        return Err(ErrorCode::InsufficientVotingWeight);
    }

    // Issue #52: Use checked_mul to prevent overflow with large voting weights.
    let cancel_pct = cancel_votes
        .checked_mul(10000)
        .and_then(|n| n.checked_div(total_votes))
        .ok_or(ErrorCode::InsufficientVotingWeight)?;
    if cancel_pct < FAILED_MARKET_THRESHOLD_BPS {
        return Err(ErrorCode::InsufficientVotingWeight);
    }

    market.status = MarketStatus::Cancelled;
    markets::update_market(e, market);

    e.events()
        .publish((Symbol::new(e, "market_cancelled_vote"), market_id), ());

    Ok(())
}

// Issue #1189: withdraw_refund used to be duplicated here, but this module's
// copy was never reachable from any public entrypoint (see #83). The real,
// reachable implementation now lives in `bets::withdraw_refund`.
