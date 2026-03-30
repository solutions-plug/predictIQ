use crate::errors::ErrorCode;
use crate::modules::{markets, sac};
use crate::types::{Bet, MarketStatus, BET_TTL_LOW_THRESHOLD, BET_TTL_HIGH_THRESHOLD};
use soroban_sdk::{contracttype, Address, Env};

/// TTL Strategy for per-user bet records (Issue #100)
///
/// Bet records (DataKey::Bet) are stored in persistent storage and MUST outlive
/// the entire market lifecycle:
///
///   place_bet  →  market Active  →  PendingResolution  →  [Disputed 72h]  →  Resolved  →  claim_winnings
///
/// Worst-case timeline:
///   - Market deadline:          up to any future date
///   - Dispute window:           48 hours (resolution.rs)
///   - Voting period:            72 hours (resolution.rs)
///   - Admin fallback window:    configurable
///   - Prune grace period:       30 days after resolution
///
/// To guarantee a bet record is readable at claim time we set:
///   BET_TTL_HIGH_THRESHOLD = ~180 days  (target lifetime after each bump)
///   BET_TTL_LOW_THRESHOLD  = ~90 days   (trigger a refresh when below this)
///
/// Bumps are applied at:
///   1. place_bet        — establishes the initial 180-day window
///   2. claim_winnings   — refreshes before the read so a long-lived market
///                         cannot cause the record to expire mid-dispute
///   3. withdraw_refund  — same protection for cancelled-market refunds
///
/// Claimed(u64, Address) sentinel records use the same TTL so the
/// AlreadyClaimed guard remains valid for the full prune grace period.


#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    Bet(u64, Address, u32),         // market_id, bettor, outcome
    Claimed(u64, Address),          // market_id, bettor — set after claim
    BetReferrer(u64, Address, u32), // market_id, bettor, outcome — referrer at bet time
}

/// Extend the TTL of a bet record to BET_TTL_HIGH_THRESHOLD.
/// Called at write time and again before any read that could race with expiry.
fn bump_bet_ttl(e: &Env, key: &DataKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, BET_TTL_LOW_THRESHOLD, BET_TTL_HIGH_THRESHOLD);
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

    // Deduct protocol fee from the bet amount before crediting the pool.
    // This ensures total_staked always reflects the net distributable pool,
    // so the parimutuel formula pays out the correct proportional share.
    let fee = crate::modules::fees::calculate_tiered_fee(e, amount, &market.tier);
    let net_amount = amount - fee;

    if fee > 0 {
        crate::modules::fees::collect_fee(e, token_address.clone(), fee);
    }

    let bet_key = DataKey::Bet(market_id, bettor.clone(), outcome);
    let mut existing_bet: Bet = e.storage().persistent().get(&bet_key).unwrap_or(Bet {
        market_id,
        bettor: bettor.clone(),
        outcome,
        amount: 0,
        fee_paid: 0,
    });

    // Store the net (post-fee) amount so the payout formula is always correct.
    existing_bet.amount = existing_bet.amount.checked_add(net_amount).ok_or(ErrorCode::ArithmeticOverflow)?;
    existing_bet.fee_paid += fee;
    existing_bet.outcome = outcome;
    market.total_staked = market.total_staked.checked_add(net_amount).ok_or(ErrorCode::ArithmeticOverflow)?;

    let outcome_stake = markets::get_outcome_stake(e, market_id, outcome);
    markets::set_outcome_stake(e, market_id, outcome, outcome_stake.checked_add(net_amount).ok_or(ErrorCode::ArithmeticOverflow)?);
    markets::increment_outcome_bet_count(e, market_id, outcome);

    // Issue #24: Maintain actual winner count per outcome
    let is_new_bettor = existing_bet.amount == net_amount; // first bet on this outcome
    if is_new_bettor {
        let current_count = market.winner_counts.get(outcome).unwrap_or(0);
        market.winner_counts.set(outcome, current_count + 1);
    }

    e.storage().persistent().set(&bet_key, &existing_bet);
    bump_bet_ttl(e, &bet_key); // Issue #100: ensure record survives full market lifecycle
    markets::update_market(e, market);
    markets::bump_market_ttl(e, market_id);

    // Track referral reward — 10% of the protocol fee goes to the referrer.
    if let Some(ref r) = referrer {
        if fee > 0 {
            crate::modules::fees::add_referral_reward(e, r, &token_address, fee);
        }
        // Store referrer so cancellation can reverse the reward if needed.
        let referrer_key = DataKey::BetReferrer(market_id, bettor.clone(), outcome);
        e.storage().persistent().set(&referrer_key, r);
        bump_bet_ttl(e, &referrer_key);
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

/// Returns the referrer stored at bet-placement time, if any.
pub fn get_bet_referrer(e: &Env, market_id: u64, bettor: Address, outcome: u32) -> Option<Address> {
    let key = DataKey::BetReferrer(market_id, bettor, outcome);
    e.storage().persistent().get(&key)
}

/// Removes the referrer record — called during refund to clean up storage.
pub fn remove_bet_referrer(e: &Env, market_id: u64, bettor: &Address, outcome: u32) {
    let key = DataKey::BetReferrer(market_id, bettor.clone(), outcome);
    e.storage().persistent().remove(&key);
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
        // Issue #100: keep the AlreadyClaimed sentinel alive for the full prune
        // grace period so double-claim attempts are rejected even after resolution.
        bump_bet_ttl(e, key);
    }
    e.storage().persistent().remove(bet_key);

    if !is_refund {
        if let Some(mut market) = markets::get_market(e, market_id) {
            market.total_claimed = market.total_claimed.saturating_add(amount);
            markets::update_market(e, market);
        }
    }

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

    // Issue #100: refresh TTL before read — a long dispute window could otherwise
    // cause the record to expire between bet placement and claim.
    bump_bet_ttl(e, &bet_key);

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
    
    // Issue #192: Use checked arithmetic to prevent overflow in high-inflation scenarios
    let winnings = bet.amount
        .checked_mul(market.total_staked)
        .and_then(|product| product.checked_div(winning_outcome_stake))
        .ok_or(ErrorCode::ArithmeticOverflow)?;

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

    // Issue #93: Refunds are outbound token movements and must respect the
    // circuit breaker just like place_bet. A paused contract must not allow
    // any token egress — including refunds — to prevent exploitation during
    // an active incident.
    crate::modules::circuit_breaker::require_not_paused_for_high_risk(e)?;

    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Cancelled {
        return Err(ErrorCode::MarketNotActive);
    }

    if token_address != market.token_address {
        return Err(ErrorCode::InvalidBetAmount);
    }

    let bet_key = DataKey::Bet(market_id, bettor.clone(), outcome);

    // Issue #100: refresh TTL before read — cancelled markets may sit idle
    // for extended periods before bettors claim their refunds.
    bump_bet_ttl(e, &bet_key);

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
    crate::modules::admin::require_admin(e)?;
    e.storage()
        .persistent()
        .set(&crate::types::ConfigKey::MinimumBetAmount, &amount);
    Ok(())
}
