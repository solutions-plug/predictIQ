use soroban_sdk::{Env, Address, Symbol, contracttype, token};
use crate::types::{Bet, MarketStatus};
use crate::modules::{markets, reentrancy};
use crate::errors::ErrorCode;
use crate::modules::{markets, sac};
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
    // Reentrancy guard
    let _guard = reentrancy::ReentrancyGuard::new(e)?;
    
    bettor.require_auth();
    
    // Check oracle freshness - prevent same-ledger manipulation
    reentrancy::check_oracle_freshness(e, market_id)?;
    
    // Enforce identity verification
    crate::modules::identity::require_verified(e, &bettor)?;

    // Check if contract is paused - high-risk operation
    crate::modules::circuit_breaker::require_not_paused_for_high_risk(e)?;

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

    // Process referral reward if referrer provided
    if let Some(ref_addr) = referrer {
        let fee = crate::modules::fees::calculate_fee(e, amount);
        crate::modules::fees::add_referral_reward(e, &ref_addr, fee);
    }

    // All storage writes BEFORE token transfer
    e.storage().persistent().set(&bet_key, &existing_bet);
    markets::update_market(e, market);

    // Token transfer MUST be last (reentrancy protection)
    let client = token::Client::new(e, &token_address);
    client.transfer(&bettor, &e.current_contract_address(), &amount);

    // Event format: (Topic, MarketID, SubjectAddr, Data)
    e.events().publish(
        (Symbol::new(e, "bet_placed"), market_id, bettor),
        amount,
    );
    
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
    // Reentrancy guard
    let _guard = reentrancy::ReentrancyGuard::new(e)?;
    
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

    // Storage write BEFORE token transfer
    e.storage().persistent().remove(&bet_key);

    e.events().publish(
        (Symbol::new(e, "winnings_claimed"), market_id, bettor.clone()),
        payout,
    );

    // Token transfer MUST be last (reentrancy protection)
    let client = token::Client::new(e, &market.token_address);
    client.transfer(&e.current_contract_address(), &bettor, &payout);

    // Emit standardized RewardsClaimed event (refund variant)
    // Topics: [RewardsClaimed, market_id, bettor]
    crate::modules::events::emit_rewards_claimed(e, market_id, bettor, refund_amount, true);

    Ok(refund_amount)
}
