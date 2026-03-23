use crate::errors::ErrorCode;
use crate::modules::{markets, sac};
use crate::types::{Bet, MarketStatus};
use soroban_sdk::{contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Bet(u64, Address),     // market_id, bettor
    Claimed(u64, Address), // market_id, bettor — set after claim
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

    if amount <= 0 {
        return Err(ErrorCode::InvalidAmount);
    }

    // Reject self-referral
    if let Some(ref r) = referrer {
        if r == &bettor {
            return Err(ErrorCode::InvalidReferrer);
        }
    }

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Active {
        return Err(ErrorCode::MarketClosed);
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
        return Err(ErrorCode::MarketClosed);
    }

    if outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }

    // Validate token_address matches market's configured asset
    if token_address != market.token_address {
        return Err(ErrorCode::InvalidBetAmount);
    }

    // Transfer tokens from bettor to contract using SAC-safe transfer
    sac::safe_transfer(
        e,
        &token_address,
        &bettor,
        &e.current_contract_address(),
        &amount,
    )?;

    let bet_key = DataKey::Bet(market_id, bettor.clone());
    let mut existing_bet: Bet = e.storage().persistent().get(&bet_key).unwrap_or(Bet {
        market_id,
        bettor: bettor.clone(),
        outcome,
        amount: 0,
    });

    existing_bet.amount += amount;
    existing_bet.outcome = outcome;
    market.total_staked += amount;

    let outcome_stake = market.outcome_stakes.get(outcome).unwrap_or(0);
    market.outcome_stakes.set(outcome, outcome_stake + amount);

    e.storage().persistent().set(&bet_key, &existing_bet);
    markets::update_market(e, market);

    // Bump TTL for market data to prevent state expiration
    markets::bump_market_ttl(e, market_id);

    // Track referral reward
    if let Some(ref r) = referrer {
        let fee = crate::modules::fees::calculate_fee(e, amount);
        if fee > 0 {
            crate::modules::fees::add_referral_reward(e, r, fee);
        }
    }

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
        return Err(ErrorCode::MarketNotResolved);
    }

    let winning_outcome = market
        .winning_outcome
        .ok_or(ErrorCode::MarketNotResolved)?;

    let bet_key = DataKey::Bet(market_id, bettor.clone());
    let claimed_key = DataKey::Claimed(market_id, bettor.clone());

    if e.storage().persistent().has(&claimed_key) {
        return Err(ErrorCode::AlreadyClaimed);
    }

    let bet: Bet = e
        .storage()
        .persistent()
        .get(&bet_key)
        .ok_or(ErrorCode::NoWinnings)?;

    if bet.outcome != winning_outcome {
        return Err(ErrorCode::NoWinnings);
    }

    // Parimutuel payout: winner's proportional share of the total pool.
    // winnings = (bet.amount * total_staked) / winning_outcome_stake
    // Integer division truncates down, favouring the protocol.
    let winning_outcome_stake = market
        .outcome_stakes
        .get(winning_outcome)
        .unwrap_or(bet.amount); // fallback: return stake if no pool data
    let winnings = (bet.amount * market.total_staked) / winning_outcome_stake;

    // Transfer winnings to bettor
    let client = token::Client::new(e, &token_address);
    client.transfer(&e.current_contract_address(), &bettor, &winnings);

    // Mark as claimed and remove bet record
    e.storage().persistent().set(&claimed_key, &true);
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
