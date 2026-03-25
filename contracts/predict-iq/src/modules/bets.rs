use crate::errors::ErrorCode;
use crate::modules::{markets, sac};
use crate::types::{Bet, MarketStatus};
use soroban_sdk::{contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Bet(u64, Address, u32), // market_id, bettor, outcome
}

pub fn place_bet(
    e: &Env,
    bettor: Address,
    market_id: u64,
    outcome: u32,
    amount: i128,
    token_address: Address,
    referrer: Option<Address>,
) -> Result<(), ErrorCode> {
    bettor.require_auth();

    crate::modules::circuit_breaker::require_not_paused_for_high_risk(e)?;

    // Issue #21: Prevent self-referral
    if let Some(ref r) = referrer {
        if r == &bettor {
            return Err(ErrorCode::NotAuthorized);
        }
    }

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Active {
        return Err(ErrorCode::MarketNotActive);
    }

    if market.parent_id > 0 {
        let parent_market =
            markets::get_market(e, market.parent_id).ok_or(ErrorCode::MarketNotFound)?;

        if parent_market.status != MarketStatus::Resolved {
            return Err(ErrorCode::ParentMarketNotResolved);
        }

        let parent_winning_outcome = parent_market
            .winning_outcome
            .ok_or(ErrorCode::ParentMarketNotResolved)?;
        if parent_winning_outcome != market.parent_outcome_idx {
            return Err(ErrorCode::ParentMarketInvalidOutcome);
        }
    }

    if e.ledger().timestamp() >= market.deadline {
        return Err(ErrorCode::DeadlinePassed);
    }

    if outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }

    if token_address != market.token_address {
        return Err(ErrorCode::InvalidBetAmount);
    }

    sac::safe_transfer(
        e,
        &token_address,
        &bettor,
        &e.current_contract_address(),
        &amount,
    )?;

    let bet_key = DataKey::Bet(market_id, bettor.clone(), outcome);
    let mut existing_bet: Bet = e.storage().persistent().get(&bet_key).unwrap_or(Bet {
        market_id,
        bettor: bettor.clone(),
        outcome,
        amount: 0,
    });

    existing_bet.amount += amount;
    market.total_staked += amount;

    let outcome_stake = market.outcome_stakes.get(outcome).unwrap_or(0);
    market.outcome_stakes.set(outcome, outcome_stake + amount);

    // Issue #24: Maintain actual winner count per outcome
    let is_new_bettor = existing_bet.amount == amount; // first bet on this outcome
    if is_new_bettor {
        let current_count = market.winner_counts.get(outcome).unwrap_or(0);
        market.winner_counts.set(outcome, current_count + 1);
    }

    e.storage().persistent().set(&bet_key, &existing_bet);
    markets::update_market(e, market);
    markets::bump_market_ttl(e, market_id);

    // Issue #1: Referral reward keyed by (referrer, token)
    if let Some(ref r) = referrer {
        let fee = crate::modules::fees::calculate_fee(e, amount);
        if fee > 0 {
            crate::modules::fees::add_referral_reward(e, r, &token_address, fee);
        }
    }

    crate::modules::events::emit_bet_placed(e, market_id, bettor, outcome, amount);

    Ok(())
}

pub fn get_bet(e: &Env, market_id: u64, bettor: Address, outcome: u32) -> Option<Bet> {
    e.storage()
        .persistent()
        .get(&DataKey::Bet(market_id, bettor, outcome))
}

/// Issue #40: Shared internal transfer + cleanup logic.
fn internal_claim_amount(
    e: &Env,
    bettor: &Address,
    market_id: u64,
    outcome: u32,
    token_address: &Address,
    is_refund: bool,
) -> Result<i128, ErrorCode> {
    let bet_key = DataKey::Bet(market_id, bettor.clone(), outcome);
    let bet: Bet = e
        .storage()
        .persistent()
        .get(&bet_key)
        .ok_or(ErrorCode::MarketNotFound)?;

    let payout = if is_refund {
        bet.amount
    } else {
        // Issue #2: Parimutuel payout — winner's share of the total pool
        let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
        let winning_stake = market.outcome_stakes.get(outcome).unwrap_or(0);
        if winning_stake == 0 {
            return Err(ErrorCode::InvalidOutcome);
        }
        (bet.amount * market.total_staked) / winning_stake
    };

    let client = token::Client::new(e, token_address);
    client.transfer(&e.current_contract_address(), bettor, &payout);

    e.storage().persistent().remove(&bet_key);

    // Track total claimed for prune guard (Issue #17)
    if !is_refund {
        if let Some(mut market) = markets::get_market(e, market_id) {
            market.total_claimed += payout;
            markets::update_market(e, market);
        }
    }

    crate::modules::events::emit_rewards_claimed(e, market_id, bettor.clone(), payout, is_refund);

    Ok(payout)
}

pub fn claim_winnings(
    e: &Env,
    bettor: Address,
    market_id: u64,
    token_address: Address,
) -> Result<i128, ErrorCode> {
    bettor.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotPendingResolution);
    }

    let winning_outcome = market
        .winning_outcome
        .ok_or(ErrorCode::MarketNotPendingResolution)?;

    internal_claim_amount(e, &bettor, market_id, winning_outcome, &token_address, false)
}

pub fn withdraw_refund(
    e: &Env,
    bettor: Address,
    market_id: u64,
    outcome: u32,
    token_address: Address,
) -> Result<i128, ErrorCode> {
    bettor.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Cancelled {
        return Err(ErrorCode::MarketNotActive);
    }

    internal_claim_amount(e, &bettor, market_id, outcome, &token_address, true)
}
