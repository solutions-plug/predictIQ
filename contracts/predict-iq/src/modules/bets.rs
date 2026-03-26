use crate::errors::ErrorCode;
use crate::modules::{markets, sac};
use crate::types::{Bet, MarketStatus};
use soroban_sdk::{contracttype, token, Address, Env};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Bet(u64, Address, u32), // market_id, bettor, outcome
    Claimed(u64, Address),  // market_id, bettor — set after claim
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

    if market.parent_id > 0 {
        markets::validate_parent_market(e, market.parent_id, market.parent_outcome_idx)?;
    }

    if e.ledger().timestamp() >= market.deadline {
        return Err(ErrorCode::MarketClosed);
    }

    // Hard-stop: once the resolution window begins, betting is locked regardless of market
    // status. This closes the race window where an oracle result is known off-chain but
    // `attempt_oracle_resolution` hasn't been called yet, preventing informed bettors from
    // exploiting information asymmetry against uninformed participants.
    if e.ledger().timestamp() >= market.resolution_deadline {
        return Err(ErrorCode::ResolutionDeadlinePassed);
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
    existing_bet.outcome = outcome;
    market.total_staked += amount;

    let outcome_stake = markets::get_outcome_stake(e, market_id, outcome);
    markets::set_outcome_stake(e, market_id, outcome, outcome_stake + amount);

    // Issue #24: Maintain actual winner count per outcome
    let is_new_bettor = existing_bet.amount == amount; // first bet on this outcome
    if is_new_bettor {
        let current_count = market.winner_counts.get(outcome).unwrap_or(0);
        market.winner_counts.set(outcome, current_count + 1);
    }

    e.storage().persistent().set(&bet_key, &existing_bet);
    markets::update_market(e, market);
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

pub fn get_bet(e: &Env, market_id: u64, bettor: Address, outcome: u32) -> Option<Bet> {
    e.storage()
        .persistent()
        .get(&DataKey::Bet(market_id, bettor, outcome))
}

fn internal_claim_amount(
    e: &Env,
    market_id: u64,
    bettor: &Address,
    token_address: &Address,
    amount: i128,
    bet_key: &DataKey,
    claimed_key: Option<&DataKey>,
    is_refund: bool,
) -> Result<i128, ErrorCode> {
    // Shared high-security transfer path for both winnings and refunds.
    sac::safe_transfer(
        e,
        token_address,
        &e.current_contract_address(),
        bettor,
        &amount,
    )?;

    if let Some(key) = claimed_key {
        e.storage().persistent().set(key, &true);
    }
    e.storage().persistent().remove(bet_key);

    crate::modules::events::emit_rewards_claimed(
        e,
        market_id,
        bettor.clone(),
        amount,
        token_address.clone(),
        is_refund,
    );

    Ok(amount)
}

pub fn claim_winnings(e: &Env, bettor: Address, market_id: u64) -> Result<i128, ErrorCode> {
    bettor.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotResolved);
    }

    let winning_outcome = market.winning_outcome.ok_or(ErrorCode::MarketNotResolved)?;

    let bet_key = DataKey::Bet(market_id, bettor.clone(), winning_outcome);
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
    let winning_outcome_stake = markets::get_outcome_stake(e, market_id, winning_outcome);
    let winning_outcome_stake = if winning_outcome_stake > 0 { winning_outcome_stake } else { bet.amount };
    let winnings = (bet.amount * market.total_staked) / winning_outcome_stake;

    internal_claim_amount(
        e,
        market_id,
        &bettor,
        &market.token_address,
        winnings,
        &bet_key,
        Some(&claimed_key),
        false,
    )
}

pub fn withdraw_refund(
    e: &Env,
    bettor: Address,
    market_id: u64,
    outcome: u32,
    token_address: Address,
) -> Result<i128, ErrorCode> {
    bettor.require_auth();

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Cancelled {
        return Err(ErrorCode::MarketNotActive);
    }

    if token_address != market.token_address {
        return Err(ErrorCode::InvalidBetAmount);
    }

    let bet_key = DataKey::Bet(market_id, bettor.clone(), outcome);
    let bet: Bet = e
        .storage()
        .persistent()
        .get(&bet_key)
        .ok_or(ErrorCode::MarketNotFound)?;

    let refund_amount = bet.amount;
    let bet_outcome = bet.outcome;

    // Update market accounting to maintain accuracy
    market.total_staked = market.total_staked.saturating_sub(refund_amount);
    let outcome_stake = market.outcome_stakes.get(bet_outcome).unwrap_or(0);
    market
        .outcome_stakes
        .set(bet_outcome, outcome_stake.saturating_sub(refund_amount));
    markets::update_market(e, market);

    internal_claim_amount(
        e,
        market_id,
        &bettor,
        &token_address,
        refund_amount,
        &bet_key,
        None,
        true,
    )
}

pub fn get_minimum_bet_amount(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&crate::types::ConfigKey::MinimumBetAmount)
        .unwrap_or(1_000_000) // Default: 0.1 XLM (1,000,000 stroops) or equivalent
}

pub fn set_minimum_bet_amount(e: &Env, amount: i128) -> Result<(), ErrorCode> {
    crate::modules::admin::require_market_admin(e)?;
    e.storage()
        .persistent()
        .set(&crate::types::ConfigKey::MinimumBetAmount, &amount);
    Ok(())
}
