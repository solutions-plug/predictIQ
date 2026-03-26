use crate::errors::ErrorCode;
use crate::types::{
    ConfigKey, CreatorReputation, Market, MarketStatus, MarketTier, OracleConfig,
    PRUNE_GRACE_PERIOD, TTL_HIGH_THRESHOLD, TTL_LOW_THRESHOLD,
};
use soroban_sdk::{contracttype, token, Address, Env, String, Vec};

#[contracttype]
pub enum DataKey {
    Market(u64),
    MarketCount,
    CreatorReputation(Address),
    OutcomeStake(u64, u32), // market_id, outcome
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

    crate::modules::circuit_breaker::require_closed(e)?;

    // Issue #176: Validate minimum options - require at least 2 options (implicitly > 0)
    // Gas optimization: Limit number of outcomes to prevent excessive iteration
    if options.len() < 2 {
        return Err(ErrorCode::InvalidOutcome);
    }
    if options.len() > crate::types::MAX_OUTCOMES_PER_MARKET {
        return Err(ErrorCode::TooManyOutcomes);
    }

    // Issue #177: Validate deadlines are in the future
    // Validate deadlines: deadline must be in the future, resolution_deadline must be after deadline
    if deadline <= e.ledger().timestamp() || resolution_deadline <= deadline {
        return Err(ErrorCode::InvalidDeadline);
    }

    // Validate parent market if this is a conditional market
    if parent_id > 0 {
        validate_parent_market(e, parent_id, parent_outcome_idx)?;

        // Also verify parent_outcome_idx is within parent's options range
        let parent_market = get_market(e, parent_id).ok_or(ErrorCode::MarketNotFound)?;
        let parent_market = get_market(e, parent_id).ok_or(ErrorCode::MarketNotFound)?;

        // Validate parent_outcome_idx is within parent's options range
        if parent_outcome_idx >= parent_market.options.len() {
            return Err(ErrorCode::InvalidOutcome);
        }

        // Allow advance creation while the parent is still Active (e.g. tournament
        // brackets).  If the parent has already resolved, gate on the outcome so we
        // never create child markets for outcomes that can never be reached.
        // Any other parent status (PendingResolution, Disputed, Cancelled) is rejected.
        match parent_market.status {
            MarketStatus::Active => {
                // Parent not yet resolved — child created in advance.
                // Betting on this child is gated at place_bet time.
            }
            MarketStatus::Resolved => {
                let parent_winning_outcome = parent_market
                    .winning_outcome
                    .ok_or(ErrorCode::ParentMarketNotResolved)?;
                if parent_winning_outcome != parent_outcome_idx {
                    return Err(ErrorCode::ParentMarketInvalidOutcome);
                }
            }
            _ => {
                // PendingResolution, Disputed, Cancelled — cannot act as a parent.
                return Err(ErrorCode::ParentMarketNotResolved);
            }
        }
    }

    let reputation = get_creator_reputation(e, &creator);
    let base_deposit = get_creation_deposit(e);

    let deposit_required = !matches!(
        reputation,
        CreatorReputation::Pro | CreatorReputation::Institutional
    );

    if adjusted_deposit > 0 {
        let token_client = token::Client::new(e, &native_token);
        let balance = token_client.balance(&creator);

        if balance < adjusted_deposit {
            return Err(ErrorCode::InsufficientDeposit);
        }

        token_client.transfer(&creator, &e.current_contract_address(), &creation_deposit);
    }

    let count = allocate_market_id(e)?;

    let num_outcomes = options.len() as u32;

    // Pre-initialize outcome_stakes map with 0 for all outcomes to optimize gas
    for i in 0..num_outcomes {
        set_outcome_stake(e, count, i, 0);
    }

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
        pending_resolution_timestamp: None,
        dispute_snapshot_ledger: None,
        dispute_timestamp: None,
    };

    e.storage()
        .persistent()
        .set(&DataKey::Market(count), &market);

    // Set initial TTL for the market data
    e.storage().persistent().extend_ttl(
        &DataKey::Market(count),
        TTL_LOW_THRESHOLD,
        TTL_HIGH_THRESHOLD,
    );

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

pub fn allocate_market_id(e: &Env) -> Result<u64, ErrorCode> {
    let current_count: u64 = e
        .storage()
        .instance()
        .get(&DataKey::MarketCount)
        .unwrap_or(0);

    let next_id = current_count
        .checked_add(1)
        .ok_or(ErrorCode::MarketIdOverflow)?;

    if e.storage().persistent().has(&DataKey::Market(next_id)) {
        return Err(ErrorCode::MarketIdCollision);
    }

    e.storage().instance().set(&DataKey::MarketCount, &next_id);

    Ok(next_id)
/// Validates that a parent market exists, is resolved, and resolved to the required outcome.
/// Called by both create_market and place_bet to enforce consistent conditional market rules.
pub fn validate_parent_market(
    e: &Env,
    parent_id: u64,
    required_outcome: u32,
) -> Result<(), ErrorCode> {
    let parent_market = get_market(e, parent_id).ok_or(ErrorCode::MarketNotFound)?;

    if parent_market.status != MarketStatus::Resolved {
        return Err(ErrorCode::ParentMarketNotResolved);
    }

    let parent_winning_outcome = parent_market
        .winning_outcome
        .ok_or(ErrorCode::ParentMarketNotResolved)?;

    if parent_winning_outcome != required_outcome {
        return Err(ErrorCode::ParentMarketInvalidOutcome);
    }

    Ok(())
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
        .unwrap_or(CreatorReputation { score: 0 })
}

pub fn set_creator_reputation(
    e: &Env,
    creator: Address,
    reputation: CreatorReputation,
) -> Result<(), ErrorCode> {
    crate::modules::admin::require_market_admin(e)?;
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
    crate::modules::admin::require_market_admin(e)?;
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
    e.storage().persistent().extend_ttl(
        &DataKey::Market(market_id),
        TTL_LOW_THRESHOLD,
        TTL_HIGH_THRESHOLD,
    );
}

/// Issue #17: Guard prune with total_claimed check.
/// Issue #47: Permissionless — anyone can call after grace period.
pub fn prune_market(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;

    let market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotResolved);
    }

    // Check if 30 days have passed since resolution
    let resolved_at = market.resolved_at.ok_or(ErrorCode::MarketNotResolved)?;
    let current_time = e.ledger().timestamp();

    if current_time < resolved_at + PRUNE_GRACE_PERIOD {
        return Err(ErrorCode::GracePeriodActive);
    }

    // Issue #17: Ensure all winnings have been claimed before pruning
    if market.total_claimed < market.total_staked {
        return Err(ErrorCode::InsufficientBalance);
    }

    e.storage().persistent().remove(&DataKey::Market(market_id));

    // Remove outcome stakes
    for i in 0..market.options.len() as u32 {
        e.storage()
            .persistent()
            .remove(&DataKey::OutcomeStake(market_id, i));
    }

    Ok(())
}
