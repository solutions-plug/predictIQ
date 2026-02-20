use soroban_sdk::{Env, Address, Symbol};
use crate::types::MarketStatus;
use crate::modules::markets;
use crate::errors::ErrorCode;

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
    market.dispute_snapshot_ledger = Some(e.ledger().sequence());
    market.dispute_timestamp = Some(e.ledger().timestamp());

    markets::update_market(e, market);

    e.events().publish(
        (Symbol::new(e, "market_disputed"), market_id, disciplinarian),
        (),
    );
    
    Ok(())
}

pub fn resolve_market(e: &Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
    // Admin override for NoMajorityReached cases
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    
    market.status = MarketStatus::Resolved;
    market.winning_outcome = Some(winning_outcome);

    markets::update_market(e, market);

    e.events().publish(
        (Symbol::new(e, "market_resolved"), market_id),
        winning_outcome,
    );
    
    Ok(())
}
