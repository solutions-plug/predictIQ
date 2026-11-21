use crate::errors::ErrorCode;
use crate::modules::{markets, sac};
use crate::types::{Bet, MarketStatus, BET_TTL_HIGH_THRESHOLD, BET_TTL_LOW_THRESHOLD};
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
    /// Issue #1686/#1687: accumulated referral reward credited for this
    /// (market, bettor, outcome). Stored so cancellation can reverse exactly
    /// what was added, against the referrer that actually earned it.
    BetReferralReward(u64, Address, u32),
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

    // Issue #1898: enforce the circuit breaker on the bet path. Reject when
    // Open, and count HalfOpen probes so the breaker can transition correctly.
    crate::modules::circuit_breaker::require_not_paused_for_high_risk(e)?;

    if amount <= 0 {
        return Err(ErrorCode::InvalidAmount);
    }

    // Issue #1681: enforce the configurable minimum bet floor so dust bets are rejected.
    if amount < get_minimum_bet_amount(e) {
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
        return Err(ErrorCode::InvalidToken);
    }

    // Check if user's tokens are frozen for SAC-wrapped assets
    sac::check_token_not_frozen(e, &token_address, &bettor)?;

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
    let fee = crate::modules::fees::calculate_tiered_fee(e, amount, &market.tier)?;
    let net_amount = amount - fee;

    if fee > 0 {
        crate::modules::fees::collect_fee(e, token_address.clone(), fee)?;
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
    existing_bet.amount = existing_bet
        .amount
        .checked_add(net_amount)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    existing_bet.fee_paid = existing_bet
        .fee_paid
        .checked_add(fee)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    existing_bet.outcome = outcome;
    market.total_staked = market
        .total_staked
        .checked_add(net_amount)
        .ok_or(ErrorCode::ArithmeticOverflow)?;

    let outcome_stake = markets::get_outcome_stake(&market, outcome);
    markets::set_outcome_stake(
        &mut market,
        outcome,
        outcome_stake
            .checked_add(net_amount)
            .ok_or(ErrorCode::ArithmeticOverflow)?,
    );

    // Issue #24: Maintain actual winner count per outcome
    let is_new_bettor = existing_bet.amount == net_amount; // first bet on this outcome
    if is_new_bettor {
        markets::increment_outcome_bet_count(&mut market, outcome);
    }

    e.storage().persistent().set(&bet_key, &existing_bet);
    bump_bet_ttl(e, &bet_key); // Issue #100: ensure record survives full market lifecycle
    markets::update_market(e, market);
    markets::bump_market_ttl(e, market_id);

    // Track referral reward — a share of the protocol fee goes to the referrer.
    //
    // Issue #1686: the referrer is locked on the first bet for a
    // (market, bettor, outcome) key. Later bets that pass a different referrer
    // do not overwrite it, so cancellation reverses the reward against the
    // referrer that actually earned it.
    //
    // Issue #1687: the exact reward credited is accumulated in
    // `BetReferralReward` so reversal subtracts precisely what was added,
    // avoiding rounding drift from recomputing the share on the aggregate fee.
    let referrer_key = DataKey::BetReferrer(market_id, bettor.clone(), outcome);
    let locked_referrer: Option<Address> = e.storage().persistent().get(&referrer_key);

    let effective_referrer = match locked_referrer {
        Some(r) => Some(r),
        None => {
            if let Some(ref r) = referrer {
                e.storage().persistent().set(&referrer_key, r);
                bump_bet_ttl(e, &referrer_key);
                Some(r.clone())
            } else {
                None
            }
        }
    };

    if let Some(r) = effective_referrer {
        if fee > 0 {
            let reward = crate::modules::fees::add_referral_reward(e, &r, &token_address, fee)?;
            if reward > 0 {
                let reward_key = DataKey::BetReferralReward(market_id, bettor.clone(), outcome);
                let accumulated: i128 = e.storage().persistent().get(&reward_key).unwrap_or(0);
                let new_total = accumulated
                    .checked_add(reward)
                    .ok_or(E