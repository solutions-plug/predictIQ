use soroban_sdk::{Env, Symbol, contracttype};
use crate::types::OracleConfig;
use crate::errors::ErrorCode;

#[contracttype]
pub enum OracleData {
    Result(u64, u32), // market_id -> outcome
    LastUpdate(u64, u64), // market_id -> timestamp
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PythPrice {
    pub price: i64,
    pub conf: u64,
    pub expo: i32,
    pub publish_time: i64,
}

pub fn fetch_pyth_price(_e: &Env, _config: &OracleConfig) -> Result<PythPrice, ErrorCode> {
    // In production, this would call the Pyth contract
    // For now, return a mock implementation that can be overridden in tests
    Err(ErrorCode::OracleFailure)
}

pub fn validate_price(e: &Env, price: &PythPrice, config: &OracleConfig) -> Result<(), ErrorCode> {
    let current_time = e.ledger().timestamp() as i64;
    let age = current_time - price.publish_time;
    
    // Check freshness
    if age > config.max_staleness_seconds as i64 {
        return Err(ErrorCode::OracleFailure);
    }
    
    // Check confidence: conf should be < max_confidence_bps% of price
    let price_abs = if price.price < 0 { -price.price } else { price.price } as u64;
    let max_conf = (price_abs * config.max_confidence_bps) / 10000;
    
    if price.conf > max_conf {
        return Err(ErrorCode::OracleFailure);
    }
    
    Ok(())
}

pub fn resolve_with_pyth(e: &Env, market_id: u64, config: &OracleConfig) -> Result<u32, ErrorCode> {
    let price = fetch_pyth_price(e, config)?;
    
    // Convert price to outcome (implementation depends on market logic)
    let outcome = determine_outcome(&price);
    
    // Record oracle update for manipulation prevention
    crate::modules::reentrancy::record_oracle_update(e, market_id);
    
    // Store result
    e.storage().persistent().set(&OracleData::Result(market_id, 0), &outcome);
    e.storage().persistent().set(&OracleData::LastUpdate(market_id, 0), &(price.publish_time as u64));
    
    // Publish event
    e.events().publish(
        (Symbol::new(e, "oracle_resolution"), market_id),
        (outcome, price.price, price.conf),
    );
    
    Ok(outcome)
}

fn determine_outcome(price: &PythPrice) -> u32 {
    // Placeholder logic - real implementation would use market-specific threshold
    if price.price > 0 { 0 } else { 1 }
}

pub fn get_oracle_result(e: &Env, market_id: u64, _config: &OracleConfig) -> Option<u32> {
    e.storage().persistent().get(&OracleData::Result(market_id, 0))
}

pub fn set_oracle_result(e: &Env, market_id: u64, outcome: u32) -> Result<(), ErrorCode> {
    // Record oracle update for manipulation prevention
    crate::modules::reentrancy::record_oracle_update(e, market_id);
    
    // Manual override for testing
    e.storage().persistent().set(&OracleData::Result(market_id, 0), &outcome);
    e.storage().persistent().set(&OracleData::LastUpdate(market_id, 0), &e.ledger().timestamp());
    
    e.events().publish(
        (Symbol::new(e, "oracle_update"), market_id),
        outcome,
    );
    
    Ok(())
}

pub fn verify_oracle_health(_e: &Env, config: &OracleConfig) -> bool {
    !config.feed_id.is_empty()
}
