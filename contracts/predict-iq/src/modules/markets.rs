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
    MarketDisputeWindow(u64),
    CreatorReputation(Address),
    /// Presence key for the status index.
    /// `StatusIndex(market_id, status)` exists iff market `market_id` currently
    /// has `status`.  Querying by status probes these keys instead of loading
    /// every market record, reducing per-call gas from O(total) to O(limit).
    StatusIndex(u64, MarketStatus),
}

/// Returns true if the status-index entry for `(market_id, status)` exists.
pub fn has_status_index(e: &Env, market_id: u64, status: &MarketStatus) -> bool {
    e.storage()
        .persistent()
        .has(&DataKey::StatusIndex(market_id, status.clone()))
}

/// Write the status-index entry for `market_id` with `new_status`,
/// removing the entry for `old_status` when it differs.
pub fn update_status_index(
    e: &Env,
    market_id: u64,
    old_status: &MarketStatus,
    new_status: &MarketStatus,
) {
    if old_status != new_status {
        e.storage()
            .persistent()
            .remove(&DataKey::StatusIndex(market_id, old_status.clone()));
    }
    e.storage()
        .persistent()
        .set(&DataKey::StatusIndex(market_id, new_status.clone()), &true);
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
    create_market_with_dispute_window(
        e,
        creator,
        description,
        options,
        deadline,
        resolution_deadline,
        oracle_config,
        tier,
        native_token,
        parent_id,
        parent_outcome_idx,
        None,
    )
}

pub fn create_market_with_dispute_window(
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
    dispute_window_seconds: Option<u64>,
) -> Result<u64, ErrorCode> {
    creator.require_auth();

    // Issue #512: Check circuit breaker - prevent market creation during emergency pause
    crate::modules::circuit_breaker::require_not_paused_for_high_risk(e)?;

    // Issue #510: Validate market deadlines
    let current_time = e.ledger().timestamp();

    // end_time (deadline) must be strictly greater than current ledger time
    if deadline <= current_time {
        return Err(ErrorCode::InvalidTimeRange);
    }

    // end_time (resolution_deadline) must be strictly greater than start_time (deadline)
    if resolution_deadline <= deadline {
        return Err(ErrorCode::InvalidTimeRange);
    }

    // Enforce minimum deadline gap (24 hours = 86400 seconds)
    const MIN_DEADLINE_GAP: u64 = 86400;
    if resolution_deadline - deadline < MIN_DEADLINE_GAP {
        return Err(ErrorCode::InvalidTimeRange);
    }

    // Gas optimization: Limit number of outcomes to prevent excessive iteration
    if options.len() > crate::types::MAX_OUTCOMES_PER_MARKET {
        return Err(ErrorCode::TooManyOutcomes);
    }

    // Validate parent market if this is a conditional market
    if parent_id > 0 {
        let parent_market = get_market(e, parent_id).ok_or(ErrorCode::MarketNotFound)?;

        // Parent must be resolved
        if parent_market.status != MarketStatus::Resolved {
            return Err(ErrorCode::ParentMarketNotResolved);
        }

        // Parent must have resolved to the required outcome
        let parent_winning_outcome = parent_market
            .winning_outcome
            .ok_or(ErrorCode::ParentMarketNotResolved)?;
        if parent_winning_outcome != parent_outcome_idx {
            return Err(ErrorCode::ParentMarketInvalidOutcome);
        }

        // Validate parent_outcome_idx is within parent's options range
        if parent_outcome_idx >= parent_market.options.len() {
            return Err(ErrorCode::InvalidOutcome);
        }

        // Issue #069: Conditional market inherits parent constraints.
        // The conditional market's deadline must not exceed the parent's resolution_deadline
        // to ensure the conditional market cannot outlive its parent context.
        if deadline > parent_market.resolution_deadline {
            return Err(ErrorCode::DeadlinePassed);
        }
    }

    let reputation = get_creator_reputation(e, &creator);

    // Issue #070: Enforce tier access control — creator reputation must meet or exceed the
    // requested market tier.
    //
    // Tier requirements:
    //   Basic       — any reputation (None, Basic, Pro, Institutional)
    //   Pro         — requires Pro or Institutional reputation
    //   Institutional — requires Institutional reputation
    //
    // Upgrade path: admin calls set_creator_reputation() to elevate a creator's reputation.
    let tier_allowed = match &tier {
        MarketTier::Basic => true,
        MarketTier::Pro => matches!(
            reputation,
            CreatorReputation::Pro | CreatorReputation::Institutional
        ),
        MarketTier::Institutional => matches!(reputation, CreatorReputation::Institutional),
    };
    if !tier_allowed {
        return Err(ErrorCode::InsufficientReputation);
    }

    let creation_deposit = get_creation_deposit(e);
    let creation_fee = get_creation_fee(e);

    // Check if deposit is required based on reputation
    let deposit_required = !matches!(
        reputation,
        CreatorReputation::Pro | CreatorReputation::Institutional
    );

    let token_client = token::Client::new(e, &native_token);
    let balance = token_client.balance(&creator);

    // Calculate total amount needed (deposit + fee)
    let total_required = if deposit_required {
        creation_deposit
    } else {
        0
    } + creation_fee;

    if total_required > 0 && balance < total_required {
        return Err(ErrorCode::InsufficientDeposit);
    }

    // Collect creation fee to protocol treasury
    if creation_fee > 0 {
        let treasury = get_protocol_treasury(e);
        token_client.transfer(&creator, &treasury, &creation_fee);

        // Emit fee collection event
        crate::modules::events::emit_fee_collected(e, 0, treasury, creation_fee);
    }

    // Lock deposit if required
    if deposit_required && creation_deposit > 0 {
        token_client.transfer(&creator, &e.current_contract_address(), &creation_deposit);
    }

    let mut count: u64 = e
        .storage()
        .instance()
        .get(&DataKey::MarketCount)
        .unwrap_or(0);
    count += 1;

    let num_outcomes = options.len() as u32;
    let dispute_window =
        crate::modules::resolution::resolve_market_dispute_window(e, dispute_window_seconds)?;

    let market = Market {
        id: count,
        creator: creator.clone(),
        description,
        options,
        status: MarketStatus::Active,
        deadline,
        resolution_deadline,
        winning_outcome: None,
        oracle_config,
        total_staked: 0,
        payout_mode: crate::types::PayoutMode::Pull,
        tier,
        creation_deposit: if deposit_required {
            creation_deposit
        } else {
            0
        },
        parent_id,
        parent_outcome_idx,
        resolved_at: None,
        token_address: native_token,
        outcome_stakes: soroban_sdk::Map::new(e),
        pending_resolution_timestamp: None,
        dispute_snapshot_ledger: None,
    };

    e.storage()
        .persistent()
        .set(&DataKey::Market(count), &market);
    e.storage()
        .persistent()
        .set(&DataKey::MarketDisputeWindow(count), &dispute_window);

    // Set initial TTL for the market data
    e.storage().persistent().extend_ttl(
        &DataKey::Market(count),
        TTL_LOW_THRESHOLD,
        TTL_HIGH_THRESHOLD,
    );
    e.storage().persistent().extend_ttl(
        &DataKey::MarketDisputeWindow(count),
        TTL_LOW_THRESHOLD,
        TTL_HIGH_THRESHOLD,
    );

    // Maintain status index so get_markets_by_status can probe O(limit) keys.
    e.storage()
        .persistent()
        .set(&DataKey::StatusIndex(count, MarketStatus::Active), &true);

    e.storage().instance().set(&DataKey::MarketCount, &count);

    // Emit standardized MarketCreated event
    // Topics: [MarketCreated, market_id, creator]
    crate::modules::events::emit_market_created(
        e,
        count,
        creator.clone(),
        market.description.clone(),
        num_outcomes,
        deadline,
    );

    Ok(count)
}

pub fn get_market_dispute_window(e: &Env, market_id: u64) -> u64 {
    e.storage()
        .persistent()
        .get(&DataKey::MarketDisputeWindow(market_id))
        .unwrap_or_else(|| crate::modules::resolution::get_default_dispute_window(e))
}

pub fn get_market(e: &Env, id: u64) -> Option<Market> {
    e.storage().persistent().get(&DataKey::Market(id))
}

pub fn update_market(e: &Env, market: Market) {
    // Keep the status index in sync when the market's status changes.
    if let Some(old) = get_market(e, market.id) {
        update_status_index(e, market.id, &old.status, &market.status);
    }
    e.storage()
        .persistent()
        .set(&DataKey::Market(market.id), &market);
}

pub fn set_payout_mode(
    e: &Env,
    market_id: u64,
    mode: crate::types::PayoutMode,
) -> Result<(), ErrorCode> {
    let mut market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    // Only allow changing payout mode before resolution
    if market.status == MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotActive);
    }

    market.payout_mode = mode;
    update_market(e, market);

    Ok(())
}

// Gas-optimized market count for specific outcome
pub fn count_bets_for_outcome(e: &Env, market_id: u64, _outcome: u32) -> u32 {
    // This would need a separate index in production
    // For now, return estimate based on storage patterns
    let key = crate::modules::bets::DataKey::Bet(market_id, e.current_contract_address());
    if e.storage().persistent().has(&key) {
        1
    } else {
        0
    }
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
    Ok(())
}

/// Issue #507: Get market creation fee (configurable by admin)
pub fn get_creation_fee(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&ConfigKey::CreationFee)
        .unwrap_or(0)
}

/// Issue #507: Set market creation fee (admin only)
pub fn set_creation_fee(e: &Env, amount: i128) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::CreationFee, &amount);
    Ok(())
}

/// Issue #507: Get protocol treasury address
pub fn get_protocol_treasury(e: &Env) -> Address {
    e.storage()
        .persistent()
        .get(&ConfigKey::ProtocolTreasury)
        .unwrap_or_else(|| e.current_contract_address())
}

/// Issue #507: Set protocol treasury address (admin only)
pub fn set_protocol_treasury(e: &Env, treasury: Address) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::ProtocolTreasury, &treasury);
    Ok(())
}

pub fn release_creation_deposit(
    e: &Env,
    market_id: u64,
    native_token: Address,
) -> Result<(), ErrorCode> {
    let market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotActive);
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

/// Bump TTL for market data to prevent state expiration
pub fn bump_market_ttl(e: &Env, market_id: u64) {
    e.storage().persistent().extend_ttl(
        &DataKey::Market(market_id),
        TTL_LOW_THRESHOLD,
        TTL_HIGH_THRESHOLD,
    );
}

/// Maximum number of markets returned per paginated query
pub const MAX_QUERY_LIMIT: u32 = 50;

/// Clamp a caller-supplied limit to [1, MAX_QUERY_LIMIT]
pub fn clamp_limit(limit: u32) -> u32 {
    limit.max(1).min(MAX_QUERY_LIMIT)
}

/// Prune (archive) a market that has been resolved and all prizes claimed
/// Can only be called 30 days after resolution
/// This is permissionless - anyone can prune expired markets
pub fn prune_market(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    // Market must be resolved
    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotActive);
    }

    // Check if 30 days have passed since resolution
    let resolved_at = market.resolved_at.ok_or(ErrorCode::MarketNotActive)?;
    let current_time = e.ledger().timestamp();

    if current_time < resolved_at + PRUNE_GRACE_PERIOD {
        return Err(ErrorCode::MarketNotActive);
    }

    // Archive the market ID for off-chain indexers
    crate::modules::event_archive::archive_market(e, market_id);

    // Remove status index entry before dropping the market record.
    e.storage()
        .persistent()
        .remove(&DataKey::StatusIndex(market_id, MarketStatus::Resolved));

    // Remove market from persistent storage
    e.storage().persistent().remove(&DataKey::Market(market_id));
    e.storage()
        .persistent()
        .remove(&DataKey::MarketDisputeWindow(market_id));

    // Emit pruning event
    crate::modules::events::emit_market_pruned(e, market_id, current_time);

    Ok(())
}
