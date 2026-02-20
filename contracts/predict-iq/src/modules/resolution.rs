use soroban_sdk::{Env, Symbol};
use crate::types::MarketStatus;
use crate::modules::{markets, oracles, voting};
use crate::errors::ErrorCode;

const DISPUTE_WINDOW_SECONDS: u64 = 86400; // 24 hours
const VOTING_PERIOD_SECONDS: u64 = 259200; // 72 hours
const MAJORITY_THRESHOLD_BPS: i128 = 6000; // 60%

/// T+0: Attempt oracle resolution at resolution deadline
pub fn attempt_oracle_resolution(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    
    if market.status != MarketStatus::Active {
        return Err(ErrorCode::MarketNotActive);
    }
    
    if e.ledger().timestamp() < market.resolution_deadline {
        return Err(ErrorCode::ResolutionNotReady);
    }
    
    // Attempt oracle resolution
    if let Some(oracle_outcome) = oracles::get_oracle_result(e, market_id, &market.oracle_config) {
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

/// T+24h: Finalize resolution if no dispute filed
pub fn finalize_resolution(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    
    match market.status {
        MarketStatus::PendingResolution => {
            // Check if 24h dispute window has passed
            let pending_ts = market.pending_resolution_timestamp.ok_or(ErrorCode::ResolutionNotReady)?;
            if e.ledger().timestamp() < pending_ts + DISPUTE_WINDOW_SECONDS {
                return Err(ErrorCode::DisputeWindowStillOpen);
            }
            
            // No dispute filed, finalize with oracle result
            let winning_outcome = market.winning_outcome.unwrap();
            market.status = MarketStatus::Resolved;
            markets::update_market(e, market);
            
            e.events().publish(
                (Symbol::new(e, "market_finalized"), market_id),
                winning_outcome,
            );
            
            Ok(())
        },
        MarketStatus::Disputed => {
            // Check if 72h voting period has passed
            let dispute_ts = market.dispute_timestamp.ok_or(ErrorCode::MarketNotDisputed)?;
            if e.ledger().timestamp() < dispute_ts + VOTING_PERIOD_SECONDS {
                return Err(ErrorCode::VotingNotStarted);
            }
            
            // Calculate voting outcome
            let winning_outcome = calculate_voting_outcome(e, &market)?;
            
            market.status = MarketStatus::Resolved;
            market.winning_outcome = Some(winning_outcome);
            markets::update_market(e, market);
            
            e.events().publish(
                (Symbol::new(e, "dispute_resolved"), market_id),
                winning_outcome,
            );
            
            Ok(())
        },
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
