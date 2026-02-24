#![cfg(test)]
use super::oracles::*;
use crate::types::OracleConfig;
use crate::errors::ErrorCode;
use soroban_sdk::{Env, Address, String, testutils::Address as _};

#[test]
fn test_validate_fresh_price() {
    let e = Env::default();
    
    let config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test_feed"),
        min_responses: 1,
        max_staleness_seconds: 300,
        max_confidence_bps: 200, // 2%
    };
    
    let price = PythPrice {
        price: 100000,
        conf: 1000, // 1% of price
        expo: -2,
        publish_time: e.ledger().timestamp() as i64 - 60, // 1 minute old
    };
    
    let result = validate_price(&e, &price, &config);
    assert!(result.is_ok());
}

#[test]
fn test_reject_stale_price() {
    let e = Env::default();
    
    let config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test_feed"),
        min_responses: 1,
        max_staleness_seconds: 300,
        max_confidence_bps: 200,
    };
    
    let price = PythPrice {
        price: 100000,
        conf: 1000,
        expo: -2,
        publish_time: e.ledger().timestamp() as i64 - 400, // 400 seconds old
    };
    
    let result = validate_price(&e, &price, &config);
    assert_eq!(result, Err(ErrorCode::StalePrice));
}

#[test]
fn test_reject_low_confidence() {
    let e = Env::default();
    
    let config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test_feed"),
        min_responses: 1,
        max_staleness_seconds: 300,
        max_confidence_bps: 200, // 2%
    };
    
    let price = PythPrice {
        price: 100000,
        conf: 3000, // 3% of price - exceeds 2% threshold
        expo: -2,
        publish_time: e.ledger().timestamp() as i64 - 60,
    };
    
    let result = validate_price(&e, &price, &config);
    assert_eq!(result, Err(ErrorCode::ConfidenceTooLow));
}
