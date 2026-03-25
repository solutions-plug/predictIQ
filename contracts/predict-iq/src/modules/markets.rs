use crate::errors::ErrorCode;
use crate::types::{
    ConfigKey, CreatorReputation, Market, MarketStatus, MarketTier, OracleConfig,
    PayoutMode, TTL_LOW_THRESHOLD, TTL_HIGH_THRESHOLD, PRUNE_GRACE_PERIOD,
};
use soroban_sdk::{contracttype, token, Address, Env, Map, String, Vec};

#[contracttype]
pub enum DataKey {
    Market(u64),
    MarketCount,
    CreatorReputation(Address),
}

pub fn create_market(
    e: &Env,
    creator: Address,
    description: String,
    options: Vec<String>,
    deadline: u64,
    resolution_deadline: u64,
    oracle_config: OracleConfig,
    tier: MarketTier,
    native_token: Address,
    parent_id: u64,
    parent_outcome_idx: u32,
) -> Result<u64, ErrorCode> {
    creator.require_auth();

    if options.len() > crate::types::MAX_OUTCOMES_PER_MARKET {
        return Err(ErrorCode::TooManyOutcomes);
    }

    // Issue #6: Betting deadline must be before resolution deadline
    if deadline >= resolution_deadline {
        return Err(ErrorCode::InvalidBetAmount);
    }

    // Issue #22: Allow child market creation when parent is Active (not just Resolved).
    // Bets on the child are blocked in place_bet until parent resolves.
    if parent_id > 0 {
        let _parent_market = get_market(e, parent_id).ok_or(ErrorCode::MarketNotFound)?;
        // No status restriction here — bets are gated in bets.rs
    }

    let reputation = get_creator_reputation(e, &creator);
    let creation_deposit = get_creation_deposit(e);

    let deposit_required = !matches!(
        reputation,
        CreatorReputation::Pro | CreatorReputation::Institutional
    );

    if deposit_required && creation_deposit > 0 {
        let token_client = token::Client::new(e, &native_token);
        let balance = token_client.balance(&creator);

        if balance < creation_deposit {
            return Err(ErrorCode::InsufficientDeposit);
        }

        token_client.transfer(&creator, &e.current_contract_address(), &creation_deposit);
    }

    let mut count: u64 = e
        .storage()
        .instance()
        .get(&DataKey::MarketCount)
        .unwrap_or(0);
    count += 1;

    let num_outcomes = options.len() as u32;

    let market = Market {
        id: count,
        creator: creator.clone(),
        description: description.clone(),
        options,
        status: MarketStatus::Active,
        deadline,
        resolution_deadline,
        winning_outcome: None,
        oracle_config,
        total_staked: 0,
        // Issue #23: payout_mode is immutable after creation
        payout_mode: PayoutMode::Pull,
        tier,
        creation_deposit: if deposit_required { creation_deposit } else { 0 },
        parent_id,
        parent_outcome_idx,
        resolved_at: None,
        token_address: native_token,
        outcome_stakes: Map::new(e),
        pending_resolution_timestamp: None,
        dispute_snapshot_ledger: None,
        dispute_timestamp: None,
        total_claimed: 0,
        winner_counts: Map::new(e),
    };

    e.storage()
        .persistent()
        .set(&DataKey::Market(count), &market);
    e.storage()
        .persistent()
        .extend_ttl(&DataKey::Market(count), TTL_LOW_THRESHOLD, TTL_HIGH_THRESHOLD);
    e.storage().instance().set(&DataKey::MarketCount, &count);

    crate::modules::events::emit_market_created(
        e,
        count,
        creator,
        description,
        num_outcomes,
        deadline,
    );

    Ok(count)
}

pub fn get_market(e: &Env, id: u64) -> Option<Market> {
    e.storage().persistent().get(&DataKey::Market(id))
}

pub fn update_market(e: &Env, market: Market) {
    e.storage()
        .persistent()
        .set(&DataKey::Market(market.id), &market);
}

/// Issue #14: Proper winner count using the maintained counter.
pub fn count_bets_for_outcome(e: &Env, market_id: u64, outcome: u32) -> u32 {
    let market = match get_market(e, market_id) {
        Some(m) => m,
        None => return 0,
    };
    market.winner_counts.get(outcome).unwrap_or(0)
}

pub fn get_creator_reputation(e: &Env, creator: &Address) -> CreatorReputation {
    e.storage()
        .persistent()
        .get(&DataKey::CreatorReputation(creator.clone()))
        .unwrap_or(CreatorReputation::None)
}

pub fn set_creator_reputation(
    e: &Env,
    creator: Address,
    reputation: CreatorReputation,
) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    e.storage()
        .persistent()
        .set(&DataKey::CreatorReputation(creator), &reputation);
    Ok(())
}

pub fn get_creation_deposit(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&ConfigKey::CreationDeposit)
        .unwrap_or(0)
}

pub fn set_creation_deposit(e: &Env, amount: i128) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::CreationDeposit, &amount);
    e.storage().persistent().extend_ttl(
        &ConfigKey::CreationDeposit,
        crate::types::GOV_TTL_LOW_THRESHOLD,
        crate::types::GOV_TTL_HIGH_THRESHOLD,
    );
    Ok(())
}

/// Issue #7: Only release deposit after the dispute window has closed.
pub fn release_creation_deposit(
    e: &Env,
    market_id: u64,
    native_token: Address,
) -> Result<(), ErrorCode> {
    let market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotActive);
    }

    // Dispute window is 24h after pending_resolution_timestamp
    let pending_ts = market
        .pending_resolution_timestamp
        .ok_or(ErrorCode::ResolutionNotReady)?;
    let dispute_window_end = pending_ts + 86400;

    if e.ledger().timestamp() < dispute_window_end {
        return Err(ErrorCode::DisputeWindowStillOpen);
    }

    if market.creation_deposit > 0 {
        let token_client = token::Client::new(e, &native_token);
        token_client.transfer(
            &e.current_contract_address(),
            &market.creator,
            &market.creation_deposit,
        );
    }

    Ok(())
}

pub fn bump_market_ttl(e: &Env, market_id: u64) {
    e.storage()
        .persistent()
        .extend_ttl(&DataKey::Market(market_id), TTL_LOW_THRESHOLD, TTL_HIGH_THRESHOLD);
}

/// Issue #17: Guard prune with total_claimed check.
/// Issue #47: Permissionless — anyone can call after grace period.
pub fn prune_market(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotActive);
    }

    let resolved_at = market.resolved_at.ok_or(ErrorCode::MarketNotActive)?;
    let current_time = e.ledger().timestamp();

    if current_time < resolved_at + PRUNE_GRACE_PERIOD {
        return Err(ErrorCode::MarketNotActive);
    }

    // Issue #17: Ensure all winnings have been claimed before pruning
    if market.total_claimed < market.total_staked {
        return Err(ErrorCode::InsufficientBalance);
    }

    e.storage().persistent().remove(&DataKey::Market(market_id));

    Ok(())
}
