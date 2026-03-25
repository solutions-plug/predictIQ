use crate::errors::ErrorCode;
use crate::types::OracleConfig;
use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

/// Issue #9: Key now includes oracle_id to support multi-oracle aggregation.
#[contracttype]
pub enum OracleData {
    Result(u64, u32),     // (market_id, oracle_id) -> outcome
    LastUpdate(u64, u32), // (market_id, oracle_id) -> timestamp
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PythPrice {
    pub price: i64,
    pub conf: u64,
    pub expo: i32,
    /// Issue #49: stored as u64 to match ledger timestamp type.
    pub publish_time: u64,
}

/// Issue #25: In production replace this stub with a real Pyth cross-contract call.
/// The function signature is kept so callers compile; the implementation
/// returns OracleFailure until a real integration is wired in.
pub fn fetch_pyth_price(_e: &Env, _config: &OracleConfig) -> Result<PythPrice, ErrorCode> {
    Err(ErrorCode::OracleFailure)
}

/// Issue #41: Use saturating_abs to avoid overflow on i64::MIN.
/// Issue #49: publish_time is now u64 — no signed/unsigned mixing.
pub fn validate_price(e: &Env, price: &PythPrice, config: &OracleConfig) -> Result<(), ErrorCode> {
    let current_time = e.ledger().timestamp(); // u64
    let age = current_time.saturating_sub(price.publish_time);

    let max_staleness = config.max_staleness_seconds.unwrap_or(3600);
    if age > max_staleness {
        return Err(ErrorCode::StalePrice);
    }

    // Issue #41: saturating_abs prevents overflow on i64::MIN
    let price_abs = price.price.saturating_abs() as u64;
    let max_conf_bps = config.max_confidence_bps.unwrap_or(200);
    let max_conf = (price_abs * max_conf_bps) / 10000;

    if price.conf > max_conf {
        return Err(ErrorCode::ConfidenceTooLow);
    }

    Ok(())
}

pub fn resolve_with_pyth(
    e: &Env,
    market_id: u64,
    oracle_id: u32,
    config: &OracleConfig,
) -> Result<u32, ErrorCode> {
    let price = fetch_pyth_price(e, config)?;
    validate_price(e, &price, config)?;

    let outcome = determine_outcome(&price);

    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, oracle_id), &outcome);
    e.storage().persistent().set(
        &OracleData::LastUpdate(market_id, oracle_id),
        &price.publish_time,
    );

    e.events().publish(
        (symbol_short!("oracle_ok"), market_id),
        (outcome, price.price, price.conf),
    );

    Ok(outcome)
}

fn determine_outcome(price: &PythPrice) -> u32 {
    if price.price > 0 { 0 } else { 1 }
}

/// Issue #9: oracle_id parameter added; callers use 0 for the primary oracle.
pub fn get_oracle_result(e: &Env, market_id: u64, oracle_id: u32) -> Option<u32> {
    e.storage()
        .persistent()
        .get(&OracleData::Result(market_id, oracle_id))
}

pub fn set_oracle_result(e: &Env, market_id: u64, outcome: u32) -> Result<(), ErrorCode> {
    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, 0), &outcome);
    e.storage().persistent().set(
        &OracleData::LastUpdate(market_id, 0),
        &e.ledger().timestamp(),
    );

    let oracle_addr = e.current_contract_address();
    crate::modules::events::emit_oracle_result_set(e, market_id, oracle_addr, outcome);

    Ok(())
}

pub fn verify_oracle_health(_e: &Env, config: &OracleConfig) -> bool {
    !config.feed_id.is_empty()
}
