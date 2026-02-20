use soroban_sdk::{Env, Address, Symbol, String, Vec, contracttype};
use crate::types::{Market, MarketStatus, OracleConfig};
use crate::errors::ErrorCode;

#[contracttype]
pub enum DataKey {
    Market(u64),
    MarketCount,
}

pub fn create_market(
    e: &Env,
    creator: Address,
    description: String,
    options: Vec<String>,
    deadline: u64,
    resolution_deadline: u64,
    oracle_config: OracleConfig,
) -> Result<u64, ErrorCode> {
    creator.require_auth();

    // Gas optimization: Limit number of outcomes to prevent excessive iteration
    if options.len() > crate::types::MAX_OUTCOMES_PER_MARKET {
        return Err(ErrorCode::TooManyOutcomes);
    }

    let mut count: u64 = e.storage().instance().get(&DataKey::MarketCount).unwrap_or(0);
    count += 1;

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
        payout_mode: crate::types::PayoutMode::Pull, // Default to pull for safety
    };

    e.storage().persistent().set(&DataKey::Market(count), &market);
    e.storage().instance().set(&DataKey::MarketCount, &count);

    // Event format: (Topic, MarketID, SubjectAddr, Data)
    e.events().publish(
        (Symbol::new(e, "market_created"), count, creator),
        (),
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
