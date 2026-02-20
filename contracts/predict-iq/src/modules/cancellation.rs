use soroban_sdk::{Env, Address, Symbol, token};
use crate::types::MarketStatus;
use crate::modules::{markets, admin};
use crate::errors::ErrorCode;

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
    
    e.events().publish(
        (Symbol::new(e, "market_cancelled"), market_id),
        (),
    );
    
    Ok(())
}

/// Community vote to cancel a market (requires 75% threshold)
pub fn cancel_market_vote(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    
    if market.status != MarketStatus::Disputed {
        return Err(ErrorCode::MarketNotDisputed);
    }
    
    // Calculate if cancellation threshold is met
    let cancel_votes = crate::modules::voting::get_tally(e, market_id, u32::MAX);
    let mut total_votes = cancel_votes;
    
    for outcome in 0..market.options.len() {
        total_votes += crate::modules::voting::get_tally(e, market_id, outcome);
    }
    
    if total_votes == 0 {
        return Err(ErrorCode::InsufficientVotingWeight);
    }
    
    let cancel_pct = (cancel_votes * 10000) / total_votes;
    if cancel_pct < FAILED_MARKET_THRESHOLD_BPS {
        return Err(ErrorCode::InsufficientVotingWeight);
    }
    
    market.status = MarketStatus::Cancelled;
    markets::update_market(e, market);
    
    e.events().publish(
        (Symbol::new(e, "market_cancelled_vote"), market_id),
        (),
    );
    
    Ok(())
}

/// Withdraw refund for cancelled market (100% principal, zero fees)
pub fn withdraw_refund(e: &Env, bettor: Address, market_id: u64) -> Result<i128, ErrorCode> {
    bettor.require_auth();
    
    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    
    if market.status != MarketStatus::Cancelled {
        return Err(ErrorCode::MarketNotCancelled);
    }
    
    let bet_key = crate::modules::bets::DataKey::Bet(market_id, bettor.clone());
    let bet: crate::types::Bet = e.storage().persistent()
        .get(&bet_key)
        .ok_or(ErrorCode::BetNotFound)?;
    
    let refund_amount = bet.amount;
    
    e.storage().persistent().remove(&bet_key);
    
    let client = token::Client::new(e, &market.token_address);
    client.transfer(&e.current_contract_address(), &bettor, &refund_amount);
    
    e.events().publish(
        (Symbol::new(e, "refund_withdrawn"), market_id, bettor),
        refund_amount,
    );
    
    Ok(refund_amount)
}
