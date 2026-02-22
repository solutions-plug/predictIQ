use soroban_sdk::{Env, Address, String, Vec, contracttype, token};
use crate::types::{Market, MarketStatus, OracleConfig, MarketTier, CreatorReputation, ConfigKey};
use crate::errors::ErrorCode;

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
) -> Result<u64, ErrorCode> {
    creator.require_auth();

    // Gas optimization: Limit number of outcomes to prevent excessive iteration
    if options.len() > crate::types::MAX_OUTCOMES_PER_MARKET {
        return Err(ErrorCode::TooManyOutcomes);
    }

    let reputation = get_creator_reputation(e, &creator);
    let creation_deposit = get_creation_deposit(e);
    
    // Check if deposit is required based on reputation
    let deposit_required = !matches!(reputation, CreatorReputation::Pro | CreatorReputation::Institutional);
    
    if deposit_required && creation_deposit > 0 {
        let token_client = token::Client::new(e, &native_token);
        let balance = token_client.balance(&creator);
        
        if balance < creation_deposit {
            return Err(ErrorCode::InsufficientDeposit);
        }
        
        // Lock deposit
        token_client.transfer(&creator, &e.current_contract_address(), &creation_deposit);
    }

    let mut count: u64 = e.storage().instance().get(&DataKey::MarketCount).unwrap_or(0);
    count += 1;

    let num_outcomes = options.len() as u32;

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
        creation_deposit: if deposit_required { creation_deposit } else { 0 },
    };

    e.storage().persistent().set(&DataKey::Market(count), &market);
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

pub fn get_market(e: &Env, id: u64) -> Option<Market> {
    e.storage().persistent().get(&DataKey::Market(id))
}

pub fn update_market(e: &Env, market: Market) {
    e.storage().persistent().set(&DataKey::Market(market.id), &market);
}

pub fn set_payout_mode(e: &Env, market_id: u64, mode: crate::types::PayoutMode) -> Result<(), ErrorCode> {
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
    e.storage().persistent().get(&DataKey::CreatorReputation(creator.clone())).unwrap_or(CreatorReputation::None)
}

pub fn set_creator_reputation(e: &Env, creator: Address, reputation: CreatorReputation) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    e.storage().persistent().set(&DataKey::CreatorReputation(creator), &reputation);
    Ok(())
}

pub fn get_creation_deposit(e: &Env) -> i128 {
    e.storage().persistent().get(&ConfigKey::CreationDeposit).unwrap_or(0)
}

pub fn set_creation_deposit(e: &Env, amount: i128) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    e.storage().persistent().set(&ConfigKey::CreationDeposit, &amount);
    Ok(())
}

pub fn release_creation_deposit(e: &Env, market_id: u64, native_token: Address) -> Result<(), ErrorCode> {
    let market = get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    
    if market.status != MarketStatus::Resolved {
        return Err(ErrorCode::MarketNotActive);
    }
    
    if market.creation_deposit > 0 {
        let token_client = token::Client::new(e, &native_token);
        token_client.transfer(&e.current_contract_address(), &market.creator, &market.creation_deposit);
    }
    
    Ok(())
}
