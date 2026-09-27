use crate::errors::ErrorCode;
use crate::modules::{markets, oracles, voting};
use crate::types::MarketStatus;
use soroban_sdk::{Env, Symbol};

pub const DEFAULT_DISPUTE_WINDOW_SECONDS: u64 = 259_200; // 72 hours
pub const MIN_DISPUTE_WINDOW_SECONDS: u64 = 3_600; // 1 hour
pub const MAX_DISPUTE_WINDOW_SECONDS: u64 = 30 * 24 * 60 * 60; // 30 days
const VOTING_PERIOD_SECONDS: u64 = 259200; // 72 hours
const MAJORITY_THRESHOLD_BPS: i128 = 6000; // 60%

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

            // No dispute filed, finalize with oracle result. Guard against a
            // market that reached PendingResolution without a winning_outcome
            // being set (e.g. migration or admin override) so we return a typed
            // error instead of panicking the whole transaction.
            let winning_outcome = market
                .winning_outcome
                .ok_or(ErrorCode::ResolutionNotReady)?;
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
            // Check if 72h voting period has passed since the dispute was actually
            // filed. Must use dispute_timestamp (set in file_dispute), not
            // pending_resolution_timestamp (set earlier at oracle resolution) —
            // otherwise a late-filed dispute inherits a stale deadline and the
            // voting window can be bypassed.
            let dispute_ts = market
                .dispute_timestamp
                .ok_or(ErrorCode::MarketNotDisputed)?;
            if e.ledger().timestamp() < dispute_ts + VOTING_PERIOD_SECONDS {
                return Err(ErrorCode::TimelockActive);
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

    // Aggregate vote weights per outcome
    let votes = voting::get_market_votes(e, market.id);
    for vote in votes.iter() {
        total_votes = total_votes
            .checked_add(vote.weight)
            .ok_or(ErrorCode::ArithmeticOverflow)?;

        let mut found = false;
        for i in 0..tallies.len() {
            let (outcome, weight) = tallies.get(i).unwrap();
            if outcome == vote.outcome {
                let new_weight = weight
                    .checked_add(vote.weight)
                    .ok_or(ErrorCode::ArithmeticOverflow)?;
                tallies.set(i, (outcome, new_weight));
                found = true;
                break;
            }
        }
        if !found {
            tallies.push_back((vote.outcome, vote.weight));
        }
    }

    if total_votes == 0 {
        return Err(ErrorCode::NoVotesCast);
    }

    // Find the outcome with the most votes
    let mut max_votes: i128 = 0;
    let mut winning_outcome: u32 = 0;
    for i in 0..tallies.len() {
        let (outcome, weight) = tallies.get(i).unwrap();
        if weight > max_votes {
            max_votes = weight;
            winning_outcome = outcome;
        }
    }

    // Guard against i128 overflow when scaling the majority percentage, matching
    // the checked_mul/checked_div pattern used in cancellation::cancel_market_vote.
    let majority_pct = max_votes
        .checked_mul(10000)
        .ok_or(ErrorCode::ArithmeticOverflow)?
        .checked_div(total_votes)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    if majority_pct < MAJORITY_THRESHOLD_BPS {
        return Err(ErrorCode::NoMajority);
    }

    Ok(winning_outcome)
}

/// Get resolution metrics for batch payout planning
pub fn get_resolution_metrics(e: &Env, market_id: u64) -> Result<(u32, u64), ErrorCode> {
    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    let winning_outcome = market.winning_outcome.ok_or(ErrorCode::ResolutionNotReady)?;

    // Issue #1535: use the per-outcome unique-bettor counter maintained by
    // `markets::increment_outcome_bet_count` instead of the broken stub that
    // always returned 0 or 1.
    let winner_count = markets::count_bets_for_outcome(e, market_id, winning_outcome);

    // Estimate gas: base cost + per-winner cost
    let gas_estimate = 100_000 + (winner_count as u64 * 50_000);

    Ok((winner_count, gas_estimate))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Market;
    use soroban_sdk::testutils::Address as _;
    use soroban_sdk::{Address, Env};

    fn setup_market(e: &Env) -> Market {
        let creator = Address::generate(e);
        Market {
            id: 1,
            creator,
            status: MarketStatus::Disputed,
            winning_outcome: None,
            resolved_at: None,
            dispute_timestamp: Some(0),
            pending_resolution_timestamp: None,
            resolution_deadline: 0,
            oracle_config: crate::types::OracleConfig::default(),
        }
    }

    #[test]
    fn test_calculate_voting_outcome_overflow_returns_typed_error() {
        let e = Env::default();
        let market = setup_market(&e);

        // A vote weight large enough that `max_votes * 10000` overflows i128.
        let huge_weight: i128 = i128::MAX / 100 + 1;
        voting::set_market_votes_for_test(&e, market.id, huge_weight);

        let result = calculate_voting_outcome(&e, &market);
        assert_eq!(result, Err(ErrorCode::ArithmeticOverflow));
    }
}
