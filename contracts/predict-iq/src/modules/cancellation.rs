use crate::errors::ErrorCode;
use crate::modules::{admin, markets, sac};
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

/// Withdraw refund for cancelled market (100% principal, zero fees).
/// `outcome` identifies which outcome position to refund. Bettors who placed
/// on multiple outcomes must call this once per outcome to reclaim all funds.
/// Issue #51: If the caller is the market creator, also refunds the creation deposit.
pub fn withdraw_refund(
    e: &Env,
    bettor: Address,
    market_id: u64,
    outcome: u32,
) -> Result<i128, ErrorCode> {
    bettor.require_auth();

    // Issue #93: Refunds are outbound token movements and must respect the
    // circuit breaker just like place_bet. A paused contract must not allow
    // any token egress — including refunds — to prevent exploitation during
    // an active incident.
    crate::modules::circuit_breaker::require_not_paused_for_high_risk(e)?;

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Cancelled {
        return Err(ErrorCode::MarketNotActive);
    }

    // Issue #51: Creator reclaims their locked creation deposit (once only).
    if bettor == market.creator && market.creation_deposit > 0 {
        let deposit = market.creation_deposit;
        market.creation_deposit = 0;
        markets::update_market(e, market.clone());
        sac::safe_transfer(
            e,
            &market.token_address,
            &e.current_contract_address(),
            &bettor,
            &deposit,
        )?;
        e.events().publish(
            (Symbol::new(e, "deposit_refunded"), market_id, bettor.clone()),
            deposit,
        );
        // If the creator also placed bets, fall through to refund those too.
    }

    let bet_key = crate::modules::bets::DataKey::Bet(market_id, bettor.clone(), outcome);
    let bet: crate::types::Bet = match e.storage().persistent().get(&bet_key) {
        Some(b) => b,
        None => return Ok(0), // creator with no bet on this outcome — deposit already refunded
    };

    // Gross refund = net amount + fee that was deducted at bet time.
    // The bettor paid `amount` originally; the contract kept `fee_paid` as
    // protocol revenue. On cancellation both must be returned.
    let refund_amount = bet.amount.checked_add(bet.fee_paid).ok_or(crate::errors::ErrorCode::ArithmeticOverflow)?;
    let fee_paid = bet.fee_paid;
    e.storage().persistent().remove(&bet_key);

    // Reverse the protocol fee revenue so accounting stays consistent.
    crate::modules::fees::reverse_fee(e, market.token_address.clone(), fee_paid);

    // Reverse any referral reward that was credited when this bet was placed.
    // The referrer only earns rewards from markets that complete — not cancelled ones.
    if let Some(referrer) = crate::modules::bets::get_bet_referrer(e, market_id, bettor.clone(), outcome) {
        crate::modules::fees::reverse_referral_reward(e, &referrer, &market.token_address, fee_paid);
        crate::modules::bets::remove_bet_referrer(e, market_id, &bettor, outcome);
    }

    sac::safe_transfer(
        e,
        &market.token_address,
        &e.current_contract_address(),
        &bettor,
        &refund_amount,
    )?;

    crate::modules::events::emit_rewards_claimed(
        e,
        market_id,
        bettor,
        refund_amount,
        market.token_address,
        true,
    );

    Ok(refund_amount)
}
