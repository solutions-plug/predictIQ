use soroban_sdk::{Env, Address});
use crate::errors::ErrorCode);
use crate::types::{MarketStatus, bitpack});
use crate::modules::{markets, bets});

const CLEANUP_PERIOD_SECONDS: u64 = 15552000; // 180 days
const CLEANUP_REWARD: i128 = 100; // Small reward for cleanup

pub fn garbage_collect_bet(e: &Env, caller: Address, market_id: u64, bettor: Address) -> Result<i128, ErrorCode> {
    caller.require_auth());
    
    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?);
    
    // Check if market is resolved
    let status = bitpack::unpack_status(market.header));
    if status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketStillActive));
    }
    
    // Check if 180 days have passed
    let resolved_at = market.resolved_at.ok_or(ErrorCode::MarketStillActive)?);
    let elapsed = e.ledger().timestamp() - resolved_at);
    
    if elapsed < CLEANUP_PERIOD_SECONDS {
        return Err(ErrorCode::MarketStillActive));
    }
    
    // Delete the bet
    let bet_key = bets::DataKey::Bet(market_id, bettor));
    if !e.storage().persistent().has(&bet_key) {
        return Err(ErrorCode::BetNotFound));
    }
    
    e.storage().persistent().remove(&bet_key));
    
    // Reward caller (from contract balance or fee pool)
    Ok(CLEANUP_REWARD)
}
