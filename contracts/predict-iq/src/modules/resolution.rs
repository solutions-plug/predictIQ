use soroban_sdk::{Env, Symbol};
use crate::types::MarketStatus;
use crate::modules::{markets, oracles, voting};
use crate::errors::ErrorCode;

/// Issue #8: Increased from 24h to 48h for global participation.
const DISPUTE_WINDOW_SECONDS: u64 = 172_800; // 48 hours
const VOTING_PERIOD_SECONDS: u64 = 259_200;  // 72 hours
const MAJORITY_THRESHOLD_BPS: i128 = 6000;   // 60%

pub fn attempt_oracle_resolution(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Active {
        return Err(ErrorCode::MarketNotActive);
    }

    if e.ledger().timestamp() < market.resolution_deadline {
        return Err(ErrorCode::ResolutionNotReady);
    }

    // Issue #9: pass oracle_id = 0 for primary oracle
    if let Some(oracle_outcome) = oracles::get_oracle_result(e, market_id, 0) {
        market.status = MarketStatus::PendingResolution;
        market.winning_outcome = Some(oracle_outcome);
        market.pending_resolution_timestamp = Some(e.ledger().timestamp());

        markets::update_market(e, market);

        e.events().publish(
            (Symbol::new(e, "oracle_resolved"), market_id),
            oracle_outcome,
        );

        Ok(())
    } else {
        Err(ErrorCode::OracleFailure)
    }
}

pub fn finalize_resolution(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    match market.status {
        MarketStatus::PendingResolution => {
            let pending_ts = market
                .pending_resolution_timestamp
                .ok_or(ErrorCode::ResolutionNotReady)?;
            if e.ledger().timestamp() < pending_ts + DISPUTE_WINDOW_SECONDS {
                return Err(ErrorCode::DisputeWindowStillOpen);
            }

            let winning_outcome = market.winning_outcome.unwrap();
            market.status = MarketStatus::Resolved;
            market.resolved_at = Some(e.ledger().timestamp());
            markets::update_market(e, market);

            e.events().publish(
                (Symbol::new(e, "market_finalized"), market_id),
                winning_outcome,
            );

            Ok(())
        }
        MarketStatus::Disputed => {
            let dispute_ts = market
                .dispute_timestamp
                .ok_or(ErrorCode::MarketNotDisputed)?;
            if e.ledger().timestamp() < dispute_ts + VOTING_PERIOD_SECONDS {
                return Err(ErrorCode::VotingNotStarted);
            }

            let winning_outcome = calculate_voting_outcome(e, &market)?;

            market.status = MarketStatus::Resolved;
            market.winning_outcome = Some(winning_outcome);
            market.resolved_at = Some(e.ledger().timestamp());
            markets::update_market(e, market);

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

/// Single-pass O(n) tally. n is bounded by MAX_OUTCOMES_PER_MARKET (32).
fn calculate_voting_outcome(e: &Env, market: &crate::types::Market) -> Result<u32, ErrorCode> {
    let num_outcomes = market.options.len();

    if num_outcomes > crate::types::MAX_OUTCOMES_PER_MARKET {
        return Err(ErrorCode::TooManyOutcomes);
    }

    let mut total_votes: i128 = 0;
    let mut max_outcome = 0u32;
    let mut max_votes = 0i128;

    for outcome in 0..num_outcomes {
        let tally = voting::get_tally(e, market.id, outcome);
        total_votes += tally;
        if tally > max_votes {
            max_votes = tally;
            max_outcome = outcome;
        }
    }

    if total_votes == 0 {
        return Err(ErrorCode::NoMajorityReached);
    }

    let majority_pct = (max_votes * 10000) / total_votes;
    if majority_pct >= MAJORITY_THRESHOLD_BPS {
        Ok(max_outcome)
    } else {
        Err(ErrorCode::NoMajorityReached)
    }
}
