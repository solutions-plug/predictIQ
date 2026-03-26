use crate::errors::ErrorCode;
use crate::modules::{admin, markets, sac};
use crate::types::MarketStatus;
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

    e.events()
        .publish((Symbol::new(e, "market_cancelled_vote"), market_id), ());

    Ok(())
}

/// Withdraw refund for cancelled market (100% principal, zero fees).
/// `outcome` identifies which outcome position to refund. Bettors who placed
/// on multiple outcomes must call this once per outcome to reclaim all funds.
pub fn withdraw_refund(e: &Env, bettor: Address, market_id: u64, outcome: u32) -> Result<i128, ErrorCode> {
    bettor.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Cancelled {
        return Err(ErrorCode::MarketNotActive);
    }

    let bet_key = crate::modules::bets::DataKey::Bet(market_id, bettor.clone());
    let bet: crate::types::Bet = e
        .storage()
        .persistent()
    
    let bet_key = crate::modules::bets::DataKey::Bet(market_id, bettor.clone(), outcome);
    let bet: crate::types::Bet = e.storage().persistent()
        .get(&bet_key)
        .ok_or(ErrorCode::BetNotFound)?;

    let refund_amount = bet.amount;

    e.storage().persistent().remove(&bet_key);

    // Use SAC-safe transfer for refund
    sac::safe_transfer(
        e,
        &market.token_address,
        &e.current_contract_address(),
        &bettor,
        &refund_amount,
    )?;

    e.events().publish(
        (Symbol::new(e, "refund_withdrawn"), market_id, bettor),
        refund_amount,
    );

    e.current_contract_address().require_auth();
    sac::safe_transfer(e, &market.token_address, &e.current_contract_address(), &bettor, &refund_amount)?;
    
    // Emit standardized RewardsClaimed event (refund variant), aligned with bets.rs standard
    // Topics: [reward_fx, market_id, bettor]
    // Data: (refund_amount, token_address, is_refund=true)
    crate::modules::events::emit_rewards_claimed(e, market_id, bettor, refund_amount, market.token_address, true);
    
    Ok(refund_amount)
}
