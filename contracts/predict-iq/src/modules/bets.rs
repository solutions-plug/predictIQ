use crate::errors::ErrorCode;
use crate::modules::markets;
use crate::types::{Bet, MarketStatus};
use soroban_sdk::{contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Bet(u64, Address), // market_id, bettor
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

    // Check if contract is paused - high-risk operation
    crate::modules::circuit_breaker::require_not_paused_for_high_risk(e)?;

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Active {
        return Err(ErrorCode::MarketNotActive);
    }

    // Validate parent market conditions for conditional markets
    if market.parent_id > 0 {
        let parent_market =
            markets::get_market(e, market.parent_id).ok_or(ErrorCode::MarketNotFound)?;

        // Parent must be resolved
        if parent_market.status != MarketStatus::Resolved {
            return Err(ErrorCode::ParentMarketNotResolved);
        }

        // Parent must have resolved to the required outcome
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

    // Validate token_address matches market's configured asset
    if token_address != market.token_address {
        return Err(ErrorCode::InvalidBetAmount);
    }

    // Transfer tokens from bettor to contract using SAC-safe transfer
    sac::safe_transfer(e, &token_address, &bettor, &e.current_contract_address(), &amount)?;

    let bet_key = DataKey::Bet(market_id, bettor.clone());
    let mut existing_bet: Bet = e.storage().persistent().get(&bet_key).unwrap_or(Bet {
        market_id,
        bettor: bettor.clone(),
        outcome,
        amount: 0,
    });

    if existing_bet.amount > 0 && existing_bet.outcome != outcome {
        return Err(ErrorCode::CannotChangeOutcome);
    }

    existing_bet.amount += amount;
    market.total_staked += amount;
    
    let outcome_stake = market.outcome_stakes.get(outcome).unwrap_or(0);
    market.outcome_stakes.set(outcome, outcome_stake + amount);

    e.storage().persistent().set(&bet_key, &existing_bet);
    markets::update_market(e, market);

    // Emit standardized BetPlaced event
    // Topics: [BetPlaced, market_id, bettor]
    crate::modules::events::emit_bet_placed(e, market_id, bettor, outcome, amount);

    Ok(())
}

pub fn get_bet(e: &Env, market_id: u64, bettor: Address) -> Option<Bet> {
    e.storage()
        .persistent()
        .get(&DataKey::Bet(market_id, bettor))
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

    let bet_key = DataKey::Bet(market_id, bettor.clone());
    let bet: Bet = e
        .storage()
        .persistent()
        .get(&bet_key)
        .ok_or(ErrorCode::MarketNotFound)?;

    if bet.outcome != winning_outcome {
        return Err(ErrorCode::InvalidOutcome);
    }

    // Calculate winnings (simplified - in production would calculate based on pool ratios)
    let winnings = bet.amount;

    // Transfer winnings to bettor
    let client = token::Client::new(e, &token_address);
    client.transfer(&e.current_contract_address(), &bettor, &winnings);

    // Remove bet record
    e.storage().persistent().remove(&bet_key);

    // Emit standardized RewardsClaimed event
    // Topics: [RewardsClaimed, market_id, bettor]
    crate::modules::events::emit_rewards_claimed(e, market_id, bettor, winnings, false);

    Ok(winnings)
}

pub fn withdraw_refund(
    e: &Env,
    bettor: Address,
    market_id: u64,
    token_address: Address,
) -> Result<i128, ErrorCode> {
    bettor.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Cancelled {
        return Err(ErrorCode::MarketNotActive);
    }

    let bet_key = DataKey::Bet(market_id, bettor.clone());
    let bet: Bet = e
        .storage()
        .persistent()
        .get(&bet_key)
        .ok_or(ErrorCode::MarketNotFound)?;

    let refund_amount = bet.amount;

    // Transfer refund to bettor
    let client = token::Client::new(e, &token_address);
    client.transfer(&e.current_contract_address(), &bettor, &refund_amount);

    // Remove bet record
    e.storage().persistent().remove(&bet_key);

    // Emit standardized RewardsClaimed event (refund variant)
    // Topics: [RewardsClaimed, market_id, bettor]
    crate::modules::events::emit_rewards_claimed(e, market_id, bettor, refund_amount, true);

    Ok(refund_amount)
}

pub fn claim_winnings(
    e: &Env,
    bettor: Address,
    market_id: u64,
) -> Result<i128, ErrorCode> {
    bettor.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    
    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketStillActive);
    }

    let bet_key = DataKey::Bet(market_id, bettor.clone());
    let bet: Bet = e.storage().persistent().get(&bet_key).ok_or(ErrorCode::BetNotFound)?;

    let winning_outcome = market.winning_outcome.ok_or(ErrorCode::MarketStillActive)?;
    
    if bet.outcome != winning_outcome {
        return Err(ErrorCode::NotWinningOutcome);
    }

    let winning_stake = market.outcome_stakes.get(winning_outcome).unwrap_or(0);
    if winning_stake == 0 {
        return Err(ErrorCode::NotWinningOutcome);
    }

    let fee = crate::modules::fees::calculate_fee(e, market.total_staked);
    let net_pool = market.total_staked - fee;
    let payout = (bet.amount * net_pool) / winning_stake;

    e.storage().persistent().remove(&bet_key);

    // Use SAC-safe transfer for payout
    sac::safe_transfer(e, &market.token_address, &e.current_contract_address(), &bettor, &payout)?;

    e.events().publish(
        (Symbol::new(e, "winnings_claimed"), market_id, bettor),
        payout,
    );

    Ok(payout)
}
