use crate::errors::ErrorCode;
use crate::modules::{markets, oracles, voting};
use crate::types::MarketStatus;
use soroban_sdk::{Env, Symbol};

pub const DEFAULT_DISPUTE_WINDOW_SECONDS: u64 = 259_200; // 72 hours
pub const MIN_DISPUTE_WINDOW_SECONDS: u64 = 3_600; // 1 hour
pub const MAX_DISPUTE_WINDOW_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days
const VOTING_PERIOD_SECONDS: u64 = 259200; // 72 hours
const MAJORITY_THRESHOLD_BPS: i128 = 6000; // 60%

pub fn get_dispute_window() -> u64 {
    DEFAULT_DISPUTE_WINDOW_SECONDS
}

pub fn get_default_dispute_window(e: &Env) -> u64 {
    e.storage()
        .persistent()
        .get(&crate::types::ConfigKey::DefaultDisputeWindow)
        .unwrap_or(DEFAULT_DISPUTE_WINDOW_SECONDS)
}

pub fn get_dispute_window_bounds(e: &Env) -> (u64, u64) {
    let min = e
        .storage()
        .persistent()
        .get(&crate::types::ConfigKey::MinDisputeWindow)
        .unwrap_or(MIN_DISPUTE_WINDOW_SECONDS);
    let max = e
        .storage()
        .persistent()
        .get(&crate::types::ConfigKey::MaxDisputeWindow)
        .unwrap_or(MAX_DISPUTE_WINDOW_SECONDS);
    (min, max)
}

pub fn set_dispute_window(e: &Env, seconds: u64) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    validate_dispute_window(e, seconds)?;
    e.storage()
        .persistent()
        .set(&crate::types::ConfigKey::DefaultDisputeWindow, &seconds);
    Ok(())
}

pub fn set_dispute_window_bounds(
    e: &Env,
    min_seconds: u64,
    max_seconds: u64,
) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    if min_seconds == 0 || min_seconds > max_seconds {
        return Err(ErrorCode::InvalidAmount);
    }

    let default_window = get_default_dispute_window(e);
    if default_window < min_seconds || default_window > max_seconds {
        return Err(ErrorCode::InvalidAmount);
    }

    e.storage()
        .persistent()
        .set(&crate::types::ConfigKey::MinDisputeWindow, &min_seconds);
    e.storage()
        .persistent()
        .set(&crate::types::ConfigKey::MaxDisputeWindow, &max_seconds);
    Ok(())
}

pub fn resolve_market_dispute_window(
    e: &Env,
    dispute_window_seconds: Option<u64>,
) -> Result<u64, ErrorCode> {
    let window = dispute_window_seconds.unwrap_or_else(|| get_default_dispute_window(e));
    validate_dispute_window(e, window)?;
    Ok(window)
}

fn validate_dispute_window(e: &Env, seconds: u64) -> Result<(), ErrorCode> {
    let (min, max) = get_dispute_window_bounds(e);
    if seconds < min || seconds > max {
        return Err(ErrorCode::InvalidAmount);
    }
    Ok(())
}

/// T+0: Attempt oracle resolution at resolution deadline
pub fn attempt_oracle_resolution(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Active {
        return Err(ErrorCode::MarketNotActive);
    }

    if e.ledger().timestamp() < market.resolution_deadline {
        return Err(ErrorCode::ResolutionNotReady);
    }

    // Issue #508: Validate oracle staleness before resolution
    oracles::validate_oracle_staleness(e, market_id, &market.oracle_config)?;

    // Attempt oracle resolution
    if let Some(oracle_outcome) = oracles::get_oracle_result(e, market_id, 0) {
        let old_status = soroban_sdk::String::from_slice(e, "Active");
        let new_status = soroban_sdk::String::from_slice(e, "PendingResolution");

        market.status = MarketStatus::PendingResolution;
        market.winning_outcome = Some(oracle_outcome);
        market.pending_resolution_timestamp = Some(e.ledger().timestamp());

        markets::update_market(e, market);

        // Emit market state change event for indexing
        crate::modules::events::emit_market_state_changed(
            e,
            market_id,
            old_status,
            new_status,
            e.ledger().timestamp(),
        );

        e.events().publish(
            (Symbol::new(e, "oracle_resolved"), market_id),
            oracle_outcome,
        );

        Ok(())
    } else {
        Err(ErrorCode::OracleFailure)
    }
}

/// T+24h: Finalize resolution if no dispute filed
pub fn finalize_resolution(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    match market.status {
        MarketStatus::PendingResolution => {
            // Check if 24h dispute window has passed
            let pending_ts = market
                .pending_resolution_timestamp
                .ok_or(ErrorCode::ResolutionNotReady)?;
            let dispute_window = markets::get_market_dispute_window(e, market_id);
            if e.ledger().timestamp() < pending_ts + dispute_window {
                return Err(ErrorCode::DisputeWindowStillOpen);
            }

            // No dispute filed, finalize with oracle result
            let winning_outcome = market.winning_outcome.unwrap();
            let old_status = soroban_sdk::String::from_slice(e, "PendingResolution");
            let new_status = soroban_sdk::String::from_slice(e, "Resolved");

            market.status = MarketStatus::Resolved;
            market.resolved_at = Some(e.ledger().timestamp());
            markets::update_market(e, market);

            // Emit market state change event for indexing
            crate::modules::events::emit_market_state_changed(
                e,
                market_id,
                old_status,
                new_status,
                e.ledger().timestamp(),
            );

            e.events().publish(
                (Symbol::new(e, "market_finalized"), market_id),
                winning_outcome,
            );

            Ok(())
        }
        MarketStatus::Disputed => {
            // Check if 72h voting period has passed since dispute was filed
            // dispute sets resolution_deadline += 3 days; use pending_resolution_timestamp as base
            let dispute_ts = market
                .pending_resolution_timestamp
                .ok_or(ErrorCode::MarketNotDisputed)?;
            if e.ledger().timestamp() < dispute_ts + VOTING_PERIOD_SECONDS {
                return Err(ErrorCode::VotingNotStarted);
            }

            // Calculate voting outcome
            let winning_outcome = calculate_voting_outcome(e, &market)?;
            let old_status = soroban_sdk::String::from_slice(e, "Disputed");
            let new_status = soroban_sdk::String::from_slice(e, "Resolved");

            market.status = MarketStatus::Resolved;
            market.winning_outcome = Some(winning_outcome);
            market.resolved_at = Some(e.ledger().timestamp());
            markets::update_market(e, market);

            // Emit market state change event for indexing
            crate::modules::events::emit_market_state_changed(
                e,
                market_id,
                old_status,
                new_status,
                e.ledger().timestamp(),
            );

            e.events().publish(
                (Symbol::new(e, "dispute_resolved"), market_id),
                winning_outcome,
            );

            Ok(())
        }
        MarketStatus::Resolved => Err(ErrorCode::CannotChangeOutcome),
        _ => Err(ErrorCode::ResolutionNotReady),
    }
}

/// Calculate voting outcome with 60% majority requirement
fn calculate_voting_outcome(e: &Env, market: &crate::types::Market) -> Result<u32, ErrorCode> {
    let mut total_votes: i128 = 0;
    let mut tallies: soroban_sdk::Vec<(u32, i128)> = soroban_sdk::Vec::new(e);

    for outcome in 0..market.options.len() {
        let tally = voting::get_tally(e, market.id, outcome);
        total_votes += tally;
        tallies.push_back((outcome, tally));
    }

    if total_votes == 0 {
        return Err(ErrorCode::NoMajorityReached);
    }

    // Find outcome with highest votes
    let mut max_outcome = 0u32;
    let mut max_votes = 0i128;

    for i in 0..tallies.len() {
        let (outcome, votes) = tallies.get(i).unwrap();
        if votes > max_votes {
            max_votes = votes;
            max_outcome = outcome;
        }
    }

    // Check if majority exceeds 60%
    let majority_pct = (max_votes * 10000) / total_votes;
    if majority_pct >= MAJORITY_THRESHOLD_BPS {
        Ok(max_outcome)
    } else {
        Err(ErrorCode::NoMajorityReached)
    }
}
