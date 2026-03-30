use crate::errors::ErrorCode;
use crate::types::{
    ConfigKey, CreatorReputation, Market, MarketStatus, MarketTier, OracleConfig,
    PayoutMode, PRUNE_GRACE_PERIOD, TTL_HIGH_THRESHOLD, TTL_LOW_THRESHOLD,
    status_tag,
};
use soroban_sdk::{contracttype, token, Address, Env, String, Vec};

#[contracttype]
pub enum DataKey {
    Market(u64),
    MarketCount,
    CreatorReputation(Address),
    OutcomeStake(u64, u32), // market_id, outcome
    OutcomeBetCount(u64, u32),
    Config(ConfigKey),
}

fn creator_reputation_rank(r: &CreatorReputation) -> u32 {
    match r {
        CreatorReputation::None => 0,
        CreatorReputation::Basic => 1,
        CreatorReputation::Pro => 2,
        CreatorReputation::Institutional => 3,
    }
}

// ---------------------------------------------------------------------------
// Issue #406: Status index helpers
// Maintains a set of market IDs per status using individual storage keys
// ConfigKey::StatusIndex(status_tag, market_id) -> bool.
// Writing/removing one key per status transition is O(1) and avoids full scans.
// ---------------------------------------------------------------------------

/// Record market_id under its current status in the status index.
pub fn index_market_status(e: &Env, market_id: u64, status: &MarketStatus) {
    let key = ConfigKey::StatusIndex(status_tag(status), market_id);
    e.storage().persistent().set(&key, &true);
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_LOW_THRESHOLD, TTL_HIGH_THRESHOLD);
}

/// Remove market_id from the old status bucket when status changes.
pub fn deindex_market_status(e: &Env, market_id: u64, old_status: &MarketStatus) {
    let key = ConfigKey::StatusIndex(status_tag(old_status), market_id);
    if e.storage().persistent().has(&key) {
        e.storage().persistent().remove(&key);
    }
}

/// Check whether market_id is indexed under the given status.
pub fn has_status_index(e: &Env, market_id: u64, status: &MarketStatus) -> bool {
    e.storage()
        .persistent()
        .has(&ConfigKey::StatusIndex(status_tag(status), market_id))
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

    if options.len() < 2 {
        return Err(ErrorCode::InvalidOutcome);
    }
    if options.len() > crate::types::MAX_OUTCOMES_PER_MARKET {
        return Err(ErrorCode::TooManyOutcomes);
    }

    if deadline <= e.ledger().timestamp() || resolution_deadline <= deadline {
        return Err(ErrorCode::InvalidDeadline);
    }

    if parent_id > 0 {
        validate_parent_market(e, parent_id, parent_outcome_idx)?;

        let parent_market = get_market(e, parent_id).ok_or(ErrorCode::MarketNotFound)?;

        if parent_outcome_idx >= parent_market.options.len() {
            return Err(ErrorCode::InvalidOutcome);
        }

        match parent_market.status {
            MarketStatus::Active => {}
            MarketStatus::Resolved => {
                let parent_winning_outcome = parent_market
                    .winning_outcome
                    .ok_or(ErrorCode::ParentMarketNotResolved)?;
                if parent_winning_outcome != parent_outcome_idx {
                    return Err(ErrorCode::ParentMarketInvalidOutcome);
                }
            }
            _ => {
                return Err(ErrorCode::ParentMarketNotResolved);
            }
        }
    }

    let reputation = get_creator_reputation(e, &creator);
    let creation_deposit = get_creation_deposit(e);
    let deposit_required = !matches!(
        reputation,
        CreatorReputation::Pro | CreatorReputation::Institutional
    );
    let adjusted_deposit = if deposit_required { creation_deposit } else { 0 };

    if creation_deposit > 0 {
        let token_client = token::Client::new(e, &native_token);
        let balance = token_client.balance(&creator);

        if balance < creation_deposit {
            return Err(ErrorCode::InsufficientDeposit);
        }

        token_client.transfer(&creator, &e.current_contract_address(), &adjusted_deposit);
    }

    let count = allocate_market_id(e)?;

    let num_outcomes = options.len() as u32;

    for i in 0..num_outcomes {
        set_outcome_stake(e, count, i, 0);
        set_outcome_bet_count(e, count, i, 0u32);
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
        payout_mode: PayoutMode::Pull,
        tier,
        creation_deposit,
        parent_id,
        parent_outcome_idx,
        resolved_at: None,
        token_address: native_token,
        outcome_stakes: soroban_sdk::Map::new(e),
        pending_resolution_timestamp: None,
        dispute_snapshot_ledger: None,
        dispute_timestamp: None,
        winner_counts: soroban_sdk::Map::new(e),
        total_claimed: 0,
    };

    e.storage()
        .persistent()
        .set(&DataKey::Market(count), &market);

    e.storage().persistent().extend_ttl(
        &DataKey::Market(count),
        TTL_LOW_THRESHOLD,
        TTL_HIGH_THRESHOLD,
    );

    e.storage().instance().set(&DataKey::MarketCount, &count);

    // Issue #406: Index the new market under Active status.
    index_market_status(e, count, &MarketStatus::Active);

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
}

/// Validates that a parent market exists, is resolved, and resolved to the required outcome.
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

/// Update a market in storage, maintaining the status index when status changes.
pub fn update_market(e: &Env, market: Market) {
    // Issue #406: Keep status index in sync on every market write.
    if let Some(old) = get_market(e, market.id) {
        if old.status != market.status {
            deindex_market_status(e, market.id, &old.status);
            index_market_status(e, market.id, &market.status);
        }
    }
    e.storage()
        .persistent()
        .set(&DataKey::Market(market.id), &market);
}

pub fn get_outcome_stake(e: &Env, market_id: u64, outcome: u32) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::OutcomeStake(market_id, outcome))
        .unwrap_or(0)
}

pub fn set_outcome_stake(e: &Env, market_id: u64, outcome: u32, amount: i128) {
    let key = DataKey::OutcomeStake(market_id, outcome);
    e.storage().persistent().set(&key, &amount);
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_LOW_THRESHOLD, TTL_HIGH_THRESHOLD);
}

pub fn set_outcome_bet_count(e: &Env, market_id: u64, outcome: u32, count: u32) {
    let key = DataKey::OutcomeBetCount(market_id, outcome);
    e.storage().persistent().set(&key, &count);
    e.storage()
        .persistent()
        .extend_ttl(&key, TTL_LOW_THRESHOLD, TTL_HIGH_THRESHOLD);
}

pub fn increment_outcome_bet_count(e: &Env, market_id: u64, outcome: u32) {
    let key = DataKey::OutcomeBetCount(market_id, outcome);
    let n: u32 = e.storage().persistent().get(&key).unwrap_or(0);
    set_outcome_bet_count(e, market_id, outcome, n + 1);
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
    let old = get_creator_reputation(e, &creator);
    e.storage()
        .persistent()
        .set(&DataKey::CreatorReputation(creator.clone()), &reputation);
    crate::modules::events::emit_creator_reputation_set(
        e,
        creator,
        creator_reputation_rank(&old),
        creator_reputation_rank(&reputation),
    );
    Ok(())
}

pub fn get_creation_deposit(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::Config(ConfigKey::CreationDeposit))
        .unwrap_or(0)
}

pub fn set_creation_deposit(e: &Env, amount: i128) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    let old = get_creation_deposit(e);
    e.storage()
        .persistent()
        .set(&DataKey::Config(ConfigKey::CreationDeposit), &amount);
    e.storage().persistent().extend_ttl(
        &DataKey::Config(ConfigKey::CreationDeposit),
        crate::types::GOV_TTL_LOW_THRESHOLD,
        crate::types::GOV_TTL_HIGH_THRESHOLD,
    );
    crate::modules::events::emit_creation_deposit_set(e, old, amount);
    Ok(())
}

/// Issue #7: Creator must explicitly claim their deposit after finality.
pub fn claim_creation_deposit(
    e: &Env,
    market_id: u64,
    caller: soroban_sdk::Address,
) -> Result<(), ErrorCode> {
    let mut market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if caller != market.creator {
        return Err(ErrorCode::NotAuthorized);
    }
    caller.require_auth();

    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotActive);
    }

    let pending_ts = market
        .pending_resolution_timestamp
        .ok_or(ErrorCode::ResolutionNotReady)?;
    let dispute_window = crate::modules::resolution::get_dispute_window(e);
    let dispute_window_end = pending_ts + dispute_window;

    if e.ledger().timestamp() < dispute_window_end {
        return Err(ErrorCode::DisputeWindowStillOpen);
    }

    let amount = market.creation_deposit;
    let creator = market.creator.clone();
    let token_address = market.token_address.clone();
    market.creation_deposit = 0;
    update_market(e, market);

    crate::modules::sac::safe_transfer(
        e,
        &token_address,
        &e.current_contract_address(),
        &creator,
        &amount,
    )?;

    e.events().publish(
        (soroban_sdk::symbol_short!("dep_claim"), market_id, creator),
        amount,
    );

    Ok(())
}

/// Issue #182: Set the payout mode for a market.
///
/// The mode may only be changed while the market is still `Active`.
/// Once the resolution process begins (`PendingResolution`, `Disputed`, or
/// `Resolved`) the payout mode is locked to guarantee stable gas and
/// distribution-path calculations throughout finalization.
pub fn set_payout_mode(
    e: &Env,
    caller: Address,
    market_id: u64,
    mode: PayoutMode,
) -> Result<(), ErrorCode> {
    caller.require_auth();

    let mut market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if caller != market.creator {
        return Err(ErrorCode::NotAuthorized);
    }

    if market.status != MarketStatus::Active {
        return Err(ErrorCode::PayoutModeLocked);
    }

    market.payout_mode = mode;
    update_market(e, market);

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
    let market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotResolved);
    }

    let resolved_at = market.resolved_at.ok_or(ErrorCode::MarketNotResolved)?;
    let current_time = e.ledger().timestamp();

    if current_time < resolved_at + PRUNE_GRACE_PERIOD {
        return Err(ErrorCode::GracePeriodActive);
    }

    if market.total_claimed < market.total_staked {
        return Err(ErrorCode::InsufficientBalance);
    }

    crate::modules::voting::prune_market_voting_state(e, market_id, market.options.len() as u32);

    // Issue #406: Remove status index entry before removing the market record.
    deindex_market_status(e, market_id, &market.status);

    e.storage().persistent().remove(&DataKey::Market(market_id));

    for i in 0..market.options.len() as u32 {
        e.storage()
            .persistent()
            .remove(&DataKey::OutcomeStake(market_id, i));
        e.storage()
            .persistent()
            .remove(&DataKey::OutcomeBetCount(market_id, i));
    }

    crate::modules::event_archive::archive_market(e, market_id);

    Ok(())
}
