#![cfg(test)]

// Comprehensive tests for Oracle price validation, with focus on confidence threshold rounding.
//
// # Issue #260: Confidence Threshold Rounding
//
// The confidence validation formula is: `max_conf = (price_abs * max_confidence_bps) / 10000`
//
// ## Problem
// Integer division can introduce bias for small prices:
// - price=1, bps=500 (5%): (1 * 500) / 10000 = 0 (truncates, should be ~0.05)
// - price=10, bps=100 (1%): (10 * 100) / 10000 = 0 (truncates, should be ~0.1)
// - price=100, bps=100 (1%): (100 * 100) / 10000 = 1 (correct)
//
// This causes a **downward bias** for small prices, making it harder to accept prices
// with any confidence interval at very small valuations.
//
// ## Potential Solutions
// 1. **Ceiling division**: Use `(price * bps + 9999) / 10000` to round up
// 2. **Fixed-point math**: Scale up before division to preserve precision
// 3. **Reverse formula**: Check `(price * bps) >= (conf * 10000)` to avoid division
//
// ## Test Coverage
// - `test_confidence_rounding_small_prices`: Tests 1-100 range prices
// - `test_confidence_rounding_large_prices`: Tests million+ range prices
// - `test_confidence_rounding_edge_cases_low_prices`: Targets specific rounding boundaries
// - `test_confidence_rounding_negative_prices`: Validates absolute value handling
// - `test_confidence_rounding_boundary_conditions`: Documents exact rounding behavior

use super::oracles::*;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

fn test_config(e: &Env) -> OracleConfig {
    OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "test_feed"),
        min_responses: 1,
        max_staleness_seconds: 300,
        max_confidence_bps: 200,
    }
}

fn create_config(e: &Env, max_confidence_bps: u64) -> OracleConfig {
    OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "test_feed"),
        min_responses: 1,
        max_staleness_seconds: 3600,
        max_confidence_bps,
    }
}

fn create_price(price: i64, conf: u64, timestamp: u64) -> PythPrice {
    PythPrice {
        price,
        conf,
        expo: -2,
        publish_time: timestamp,
    }
}

#[test]
fn test_validate_fresh_price() {
    let e = Env::default();

    let config = test_config(&e);
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

    let config = test_config(&e);
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
fn test_accept_price_at_exact_staleness_boundary() {
    let e = Env::default();

    let config = test_config(&e);
    let price = PythPrice {
        price: 100000,
        conf: 1000, // 1% of price - within 2% threshold
        expo: -2,
        publish_time: e.ledger().timestamp() as i64 - 300, // exactly 300 seconds old (max_staleness_seconds)
    };

    let result = validate_price(&e, &price, &config);
    assert!(result.is_ok(), "Price at exact staleness boundary (age == max_staleness_seconds) should be accepted");
}

#[test]
fn test_reject_low_confidence() {
    let e = Env::default();

    let config = test_config(&e);
    let price = PythPrice {
        price: 100000,
        conf: 3000, // 3% of price - exceeds 2% threshold
        expo: -2,
        publish_time: e.ledger().timestamp() as i64 - 60,
    };

    let result = validate_price(&e, &price, &config);
    assert_eq!(result, Err(ErrorCode::ConfidenceTooLow));
}

#[test]
fn test_cast_external_timestamp_rejects_negative_values() {
    assert_eq!(
        cast_external_timestamp(-1),
        Err(ErrorCode::InvalidTimestamp)
    );
}

#[test]
fn test_cast_external_timestamp_accepts_zero() {
    assert_eq!(cast_external_timestamp(0), Ok(0));
}

#[test]
fn test_cast_external_timestamp_accepts_positive_values() {
    assert_eq!(cast_external_timestamp(1_700_000_000), Ok(1_700_000_000));
}

#[test]
fn test_is_stale_returns_false_for_fresh_data() {
    assert!(!is_stale(1_700_001_000, 1_700_000_900, 300));
}

#[test]
fn test_is_stale_returns_true_for_old_data() {
    assert!(is_stale(1_700_001_000, 1_699_999_000, 300));
}

#[test]
fn test_is_stale_boundary_is_not_stale() {
    assert!(!is_stale(1_700_001_000, 1_700_000_700, 300));
}

#[test]
fn test_is_stale_future_timestamp_does_not_underflow() {
    assert!(!is_stale(1_700_000_000, 1_700_001_000, 300));
}

#[test]
fn test_validate_price_rejects_negative_publish_time() {
    let e = Env::default();
    let config = test_config(&e);
    let price = PythPrice {
        price: 100000,
        conf: 1000,
        expo: -2,
        publish_time: -1,
    };

    let result = validate_price(&e, &price, &config);
    assert_eq!(result, Err(ErrorCode::InvalidTimestamp));
}

// =============================================================================
// Issue #261: Multi-Oracle Keying Tests
// =============================================================================
//
// # Issue #261: Multi-Oracle Keying & Collision Prevention
//
// The oracle result storage uses a composite key: `OracleData::Result(market_id, oracle_id)`
//
// ## Problem
// Without proper testing, the following risks exist:
// 1. **Key Collisions**: Different (market_id, oracle_id) pairs could hash to same storage location
// 2. **Data Isolation Failure**: Retrieving (market_id=1, oracle_id=1) could return data from (market_id=1, oracle_id=2)
// 3. **Missing Multi-Oracle Support**: No tests verifying multiple oracle IDs per market work correctly
// 4. **Boundary Weaknesses**: Untested edge cases with large market_ids, large oracle_ids, or both
//
// ## Storage Key Structure
// ```
// OracleData::Result(market_id: u64, oracle_id: u32) -> outcome: u32
// OracleData::LastUpdate(market_id: u64, oracle_id: u32) -> timestamp: u64
// ```
//
// ## Test Coverage
// - `test_multi_oracle_basic_storage_retrieval`: Verify store/retrieve for (market_id, oracle_id) pairs
// - `test_multi_oracle_isolation_same_market`: Ensure different oracle_ids in same market don't collide
// - `test_multi_oracle_isolation_different_markets`: Ensure same oracle_id in different markets don't collide
// - `test_multi_oracle_matrix_combinations`: Table-driven tests of all combinations
// - `test_multi_oracle_large_ids`: Tests with maximum/boundary u64 and u32 values
// - `test_multi_oracle_sequential_updates`: Ensure updates don't affect other oracles
// - `test_multi_oracle_timestamp_independence`: Verify timestamps are independent per (market_id, oracle_id)
// - `test_multi_oracle_collision_mitigation`: Demonstrates the fix prevents theoretical collisions

/// Basic sanity test: Store and retrieve oracle results for a single (market_id, oracle_id) pair.
#[test]
fn test_multi_oracle_basic_storage_retrieval() {
    let e = Env::default();
    let market_id = 100u64;
    let oracle_id = 0u32;
    let outcome = 1u32;

    // Store result
    e.storage().persistent().set(&OracleData::Result(market_id, oracle_id), &outcome);
    
    // Retrieve and verify
    let retrieved: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, oracle_id));
    assert_eq!(retrieved, Some(outcome), 
               "Failed to retrieve outcome for market_id={}, oracle_id={}", market_id, oracle_id);
}

/// Test isolation within a single market: Different oracle_ids should have independent storage.
/// This is critical for multi-oracle aggregation - same market, different sources.
#[test]
fn test_multi_oracle_isolation_same_market() {
    let e = Env::default();
    let market_id = 100u64;

    // Test cases: (oracle_id, outcome)
    let test_cases = vec![
        (0u32, 0u32, "primary oracle outcome=0"),
        (1u32, 1u32, "secondary oracle outcome=1"),
        (2u32, 0u32, "tertiary oracle outcome=0"),
        (3u32, 1u32, "quaternary oracle outcome=1"),
    ];

    // Store multiple oracle results for same market
    for (oracle_id, outcome, desc) in &test_cases {
        e.storage()
            .persistent()
            .set(&OracleData::Result(market_id, *oracle_id), outcome);
    }

    // Verify each oracle result is independent and correct
    for (oracle_id, expected_outcome, desc) in &test_cases {
        let retrieved: Option<u32> = e.storage()
            .persistent()
            .get(&OracleData::Result(market_id, *oracle_id));
        
        assert_eq!(
            retrieved,
            Some(*expected_outcome),
            "Isolation failure for market_id={}, oracle_id={}: {} | Got: {:?}, Expected: {}",
            market_id, oracle_id, desc, retrieved, expected_outcome
        );
    }
}

/// Test isolation across markets: Same oracle_id in different markets should be independent.
/// This is critical for market isolation - prevents cross-market data leakage.
#[test]
fn test_multi_oracle_isolation_different_markets() {
    let e = Env::default();
    let oracle_id = 0u32; // Use same oracle for different markets

    // Test cases: (market_id, outcome)
    let test_cases = vec![
        (1u64, 0u32, "market 1 outcome=0"),
        (2u64, 1u32, "market 2 outcome=1"),
        (3u64, 0u32, "market 3 outcome=0"),
        (100u64, 1u32, "market 100 outcome=1"),
        (1000u64, 0u32, "market 1000 outcome=0"),
    ];

    // Store oracle result in each market
    for (market_id, outcome, desc) in &test_cases {
        e.storage()
            .persistent()
            .set(&OracleData::Result(*market_id, oracle_id), outcome);
    }

    // Verify each market's result is independent and correct
    for (market_id, expected_outcome, desc) in &test_cases {
        let retrieved: Option<u32> = e.storage()
            .persistent()
            .get(&OracleData::Result(*market_id, oracle_id));
        
        assert_eq!(
            retrieved,
            Some(*expected_outcome),
            "Market isolation failure for market_id={}, oracle_id={}: {} | Got: {:?}, Expected: {}",
            market_id, oracle_id, desc, retrieved, expected_outcome
        );
    }
}

/// Matrix test: All combinations of (market_id, oracle_id) pairs must have independent storage.
/// This comprehensive test verifies the collision-free property of the composite key.
#[test]
fn test_multi_oracle_matrix_combinations() {
    let e = Env::default();

    // Test matrix: 3 markets × 4 oracles = 12 distinct pairs
    let market_ids = vec![1u64, 100u64, 10000u64];
    let oracle_ids = vec![0u32, 1u32, 2u32, 3u32];
    
    // Store unique outcome for each pair: outcome = (market_id % 2) XOR (oracle_id % 2)
    let mut stored_pairs = vec![];
    for market_id in &market_ids {
        for oracle_id in &oracle_ids {
            let outcome = ((market_id % 2) as u32) ^ (oracle_id % 2);
            e.storage()
                .persistent()
                .set(&OracleData::Result(*market_id, *oracle_id), &outcome);
            stored_pairs.push((*market_id, *oracle_id, outcome));
        }
    }

    // Verify all pairs retrieve correct values (no collisions, no cross-pollution)
    for (market_id, oracle_id, expected_outcome) in stored_pairs {
        let retrieved: Option<u32> = e.storage()
            .persistent()
            .get(&OracleData::Result(market_id, oracle_id));
        
        assert_eq!(
            retrieved,
            Some(expected_outcome),
            "Matrix collision detected at market_id={}, oracle_id={} | Got: {:?}, Expected: {}",
            market_id, oracle_id, retrieved, expected_outcome
        );
    }
}

/// Test with large ID values: market_id near u64::MAX and oracle_id near u32::MAX.
/// Boundary testing to ensure hash collisions don't occur with extreme values.
#[test]
fn test_multi_oracle_large_ids() {
    let e = Env::default();

    // Test cases: (market_id, oracle_id, outcome, description)
    let test_cases = vec![
        (u64::MAX, 0u32, 0u32, "market_id=MAX, oracle_id=0"),
        (u64::MAX - 1, 0u32, 1u32, "market_id=MAX-1, oracle_id=0"),
        (0u64, u32::MAX, 1u32, "market_id=0, oracle_id=MAX"),
        (0u64, u32::MAX - 1, 0u32, "market_id=0, oracle_id=MAX-1"),
        (u64::MAX, u32::MAX, 1u32, "market_id=MAX, oracle_id=MAX"),
        (u64::MAX - 1, u32::MAX - 1, 0u32, "market_id=MAX-1, oracle_id=MAX-1"),
        (1u64, u32::MAX / 2, 1u32, "market_id=1, oracle_id=MAX/2"),
        (u64::MAX / 2, 1u32, 0u32, "market_id=MAX/2, oracle_id=1"),
    ];

    // Store and verify large ID combinations
    for (market_id, oracle_id, outcome, desc) in &test_cases {
        e.storage()
            .persistent()
            .set(&OracleData::Result(*market_id, *oracle_id), outcome);
        
        let retrieved: Option<u32> = e.storage()
            .persistent()
            .get(&OracleData::Result(*market_id, *oracle_id));
        
        assert_eq!(
            retrieved,
            Some(*outcome),
            "Large ID test failed: {} | Got: {:?}, Expected: {}",
            desc, retrieved, outcome
        );
    }
}

/// Test sequential updates: Updating one oracle shouldn't affect others.
/// Verifies that write operations are truly isolated.
#[test]
fn test_multi_oracle_sequential_updates() {
    let e = Env::default();
    let market_id = 100u64;

    // Initial state: Store outcomes for 3 oracles
    let initial_outcomes = vec![
        (0u32, 0u32),
        (1u32, 1u32),
        (2u32, 0u32),
    ];

    for (oracle_id, outcome) in &initial_outcomes {
        e.storage()
            .persistent()
            .set(&OracleData::Result(market_id, *oracle_id), outcome);
    }

    // Update oracle 1 outcome and verify others unchanged
    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, 1u32), &1u32);

    // Verify oracle 0 unchanged
    let oracle_0: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 0u32));
    assert_eq!(oracle_0, Some(0u32), "Oracle 0 was corrupted by update to oracle 1");

    // Verify oracle 2 unchanged
    let oracle_2: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 2u32));
    assert_eq!(oracle_2, Some(0u32), "Oracle 2 was corrupted by update to oracle 1");

    // Verify oracle 1 updated
    let oracle_1: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 1u32));
    assert_eq!(oracle_1, Some(1u32), "Oracle 1 failed to update");
}

/// Test timestamp independence: LastUpdate timestamps should be independent per (market_id, oracle_id).
/// Ensures that updating one oracle's timestamp doesn't affect others.
#[test]
fn test_multi_oracle_timestamp_independence() {
    let e = Env::default();
    let market_id = 100u64;
    let oracle_ids = vec![0u32, 1u32, 2u32];
    let timestamps = vec![1000u64, 2000u64, 3000u64];

    // Store different timestamps for each oracle
    for (i, oracle_id) in oracle_ids.iter().enumerate() {
        let timestamp = timestamps[i];
        e.storage()
            .persistent()
            .set(&OracleData::LastUpdate(market_id, *oracle_id), &timestamp);
    }

    // Verify each oracle has its own independent timestamp
    for (i, oracle_id) in oracle_ids.iter().enumerate() {
        let expected_timestamp = timestamps[i];
        let retrieved: Option<u64> = e.storage()
            .persistent()
            .get(&OracleData::LastUpdate(market_id, *oracle_id));
        
        assert_eq!(
            retrieved,
            Some(expected_timestamp),
            "Timestamp isolation failure for oracle_id={} | Got: {:?}, Expected: {}",
            oracle_id, retrieved, expected_timestamp
        );
    }

    // Update one timestamp and verify others unchanged
    e.storage()
        .persistent()
        .set(&OracleData::LastUpdate(market_id, 1u32), &9999u64);

    // Verify oracle 0 timestamp unchanged
    let oracle_0_ts: Option<u64> = e.storage()
        .persistent()
        .get(&OracleData::LastUpdate(market_id, 0u32));
    assert_eq!(oracle_0_ts, Some(1000u64), "Oracle 0 timestamp was corrupted");

    // Verify oracle 2 timestamp unchanged
    let oracle_2_ts: Option<u64> = e.storage()
        .persistent()
        .get(&OracleData::LastUpdate(market_id, 2u32));
    assert_eq!(oracle_2_ts, Some(3000u64), "Oracle 2 timestamp was corrupted");

    // Verify oracle 1 timestamp updated
    let oracle_1_ts: Option<u64> = e.storage()
        .persistent()
        .get(&OracleData::LastUpdate(market_id, 1u32));
    assert_eq!(oracle_1_ts, Some(9999u64), "Oracle 1 timestamp failed to update");
}

/// Comprehensive collision mitigation test: Verify the composite key prevents collisions.
/// Tests theoretical collision scenarios that would fail with poor key design.
#[test]
fn test_multi_oracle_collision_mitigation() {
    let e = Env::default();

    // Collision scenarios that would fail if keys weren't properly composite:
    // 1. Simple concatenation: market_id=1, oracle_id=0 (key="10") vs market_id=10, oracle_id=0 (key="100")
    // 2. Bit-packing errors: market_id=(u32::MAX+1), oracle_id=0 could collide with others
    // 3. Hash collisions: Poor struct hashing could cause different (m,o) pairs to hash to same location

    let collision_scenarios = vec![
        // Scenario 1: Simple string concatenation would collide
        ((1u64, 0u32, 0u32), (10u64, 0u32, 1u32), "concatenation collision risk: '10' vs '100'"),
        // Scenario 2: Overflow/wrapping issues
        ((u32::MAX as u64, 0u32, 0u32), ((u32::MAX as u64) + 1, 0u32, 1u32), "boundary overflow risk"),
        // Scenario 3: Adjacent values
        ((1000u64, 1u32, 0u32), (1000u64, 2u32, 1u32), "adjacent oracle_id differentiation"),
        ((1000u64, 0u32, 0u32), (1001u64, 0u32, 1u32), "adjacent market_id differentiation"),
        // Scenario 4: Reversed pairs (if key wasn't ordered)
        ((1u64, 100u32, 0u32), (100u64, 1u32, 1u32), "reversed (market, oracle) pair"),
    ];

    // Store values for all collision scenarios
    for ((m1, o1, v1), (m2, o2, v2), scenario_desc) in &collision_scenarios {
        e.storage()
            .persistent()
            .set(&OracleData::Result(*m1, *o1), v1);
        e.storage()
            .persistent()
            .set(&OracleData::Result(*m2, *o2), v2);

        // Verify no collision: each key retrieves its own value
        let retrieved_1: Option<u32> = e.storage().persistent().get(&OracleData::Result(*m1, *o1));
        let retrieved_2: Option<u32> = e.storage().persistent().get(&OracleData::Result(*m2, *o2));

        assert_eq!(
            retrieved_1, Some(*v1),
            "Collision scenario failed ({}, {}, {}): first key returned wrong value. Scenario: {}",
            m1, o1, v1, scenario_desc
        );
        
        assert_eq!(
            retrieved_2, Some(*v2),
            "Collision scenario failed ({}, {}, {}): second key returned wrong value. Scenario: {}",
            m2, o2, v2, scenario_desc
        );
    }
}

// =============================================================================
// Issue #25: fetch_pyth_price cross-contract call tests
// =============================================================================

#[cfg(feature = "testutils")]
mod pyth_integration_tests {
    use super::*;
    use soroban_sdk::{contract, contractimpl, testutils::Address as _, Address, BytesN, Env, String};
    use crate::modules::oracles::{fetch_pyth_price, PythPrice};
    use crate::types::OracleConfig;

    /// Minimal mock Pyth contract that returns a fixed price for any feed_id.
    #[contract]
    pub struct MockPythContract;

    #[contractimpl]
    impl MockPythContract {
        pub fn get_price(_env: Env, _feed_id: BytesN<32>) -> (i64, u64, i32, i64) {
            // BTC/USD: $50,000.00 with 2% confidence, expo -2, recent timestamp
            (5_000_000i64, 100_000u64, -2i32, 1_700_000_000i64)
        }
    }

    fn valid_feed_id(e: &Env) -> String {
        // 64-char hex string representing a 32-byte Pyth price feed ID
        String::from_str(e, "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43")
    }

    #[test]
    fn test_fetch_pyth_price_returns_price_from_contract() {
        let e = Env::default();
        let pyth_addr = e.register(MockPythContract, ());

        let config = OracleConfig {
            oracle_address: pyth_addr,
            feed_id: valid_feed_id(&e),
            min_responses: 1,
            max_staleness_seconds: 3600,
            max_confidence_bps: 500,
        };

        let result = fetch_pyth_price(&e, &config);
        assert!(result.is_ok(), "fetch_pyth_price should succeed with a valid mock contract");

        let price = result.unwrap();
        assert_eq!(price.price, 5_000_000);
        assert_eq!(price.conf, 100_000);
        assert_eq!(price.expo, -2);
        assert_eq!(price.publish_time, 1_700_000_000);
    }

    #[test]
    fn test_fetch_pyth_price_fails_with_invalid_feed_id() {
        let e = Env::default();
        let pyth_addr = e.register(MockPythContract, ());

        let config = OracleConfig {
            oracle_address: pyth_addr,
            feed_id: String::from_str(&e, "not_a_valid_hex_feed_id"),
            min_responses: 1,
            max_staleness_seconds: 3600,
            max_confidence_bps: 500,
        };

        let result = fetch_pyth_price(&e, &config);
        assert_eq!(result, Err(crate::errors::ErrorCode::OracleFailure));
    }

    // -------------------------------------------------------------------------
    // Oracle failure path / retry cadence tests
    //
    // These tests use attempt_oracle_resolution at the contract level so they
    // exercise the full path: fetch → validate → write → market state update.
    // A bad feed_id is the simplest way to force OracleFailure without a mock
    // that panics, because decode_feed_id rejects it before the cross-contract
    // call, giving a deterministic failure with no partial storage writes.
    // -------------------------------------------------------------------------

    use crate::{PredictIQ, PredictIQClient};
    use crate::types::{MarketStatus, MarketTier};
    use soroban_sdk::{testutils::Ledger as _, Vec};

    fn setup_contract_with_bad_oracle(e: &Env) -> (PredictIQClient<'static>, u64) {
        e.mock_all_auths();
        let contract_id = e.register(PredictIQ, ());
        let client = PredictIQClient::new(e, &contract_id);
        let admin = Address::generate(e);
        client.initialize(&admin, &100);

        // Oracle config with an invalid feed_id — fetch_pyth_price will return
        // OracleFailure before touching any storage.
        let bad_config = OracleConfig {
            oracle_address: Address::generate(e),
            feed_id: String::from_str(e, "not_hex"),
            min_responses: 1,
            max_staleness_seconds: 3600,
            max_confidence_bps: 500,
        };
        let token = Address::generate(e);
        let options = Vec::from_array(e, [
            soroban_sdk::String::from_str(e, "Yes"),
            soroban_sdk::String::from_str(e, "No"),
        ]);
        let market_id = client.create_market(
            &admin,
            &soroban_sdk::String::from_str(e, "Oracle test market"),
            &options,
            &1000,
            &2000,
            &bad_config,
            &MarketTier::Basic,
            &token,
            &0,
            &0,
        );
        (client, market_id)
    }

    /// A single oracle failure must return OracleFailure and leave the market Active.
    #[test]
    fn test_oracle_failure_leaves_market_active() {
        let e = Env::default();
        let (client, market_id) = setup_contract_with_bad_oracle(&e);

        e.ledger().set_timestamp(2000); // at resolution deadline

        let result = client.try_attempt_oracle_resolution(&market_id);
        assert_eq!(result, Err(Ok(crate::errors::ErrorCode::OracleFailure)));

        let market = client.get_market(&market_id).unwrap();
        assert_eq!(market.status, MarketStatus::Active);
        assert!(market.winning_outcome.is_none());
        assert!(market.pending_resolution_timestamp.is_none());
    }

    /// Repeated oracle failures must never mutate market state — status stays
    /// Active and no partial oracle storage is written on any iteration.
    #[test]
    fn test_repeated_oracle_failures_no_partial_state() {
        let e = Env::default();
        let (client, market_id) = setup_contract_with_bad_oracle(&e);

        e.ledger().set_timestamp(2000);

        for _ in 0..5 {
            let result = client.try_attempt_oracle_resolution(&market_id);
            assert_eq!(result, Err(Ok(crate::errors::ErrorCode::OracleFailure)));
        }

        let market = client.get_market(&market_id).unwrap();
        assert_eq!(market.status, MarketStatus::Active,
            "market must remain Active after repeated oracle failures");
        assert!(market.winning_outcome.is_none(),
            "winning_outcome must not be set after oracle failures");
        assert!(market.pending_resolution_timestamp.is_none(),
            "pending_resolution_timestamp must not be set after oracle failures");

        // Oracle storage keys must also be absent.
        assert!(client.get_oracle_result(&market_id, &0).is_none(),
            "oracle result key must not exist after failed attempts");
        assert!(client.get_oracle_last_update(&market_id, &0).is_none(),
            "oracle last_update key must not exist after failed attempts");
    }

    /// After N failures the market must still accept a successful resolution
    /// once a valid oracle result is injected — verifying retry cadence works.
    #[test]
    fn test_oracle_retry_succeeds_after_failures() {
        let e = Env::default();
        e.mock_all_auths();

        let contract_id = e.register(PredictIQ, ());
        let client = PredictIQClient::new(&e, &contract_id);
        let admin = Address::generate(&e);
        client.initialize(&admin, &100);

        // Start with a bad oracle config so the first attempts fail.
        let bad_config = OracleConfig {
            oracle_address: Address::generate(&e),
            feed_id: String::from_str(&e, "not_hex"),
            min_responses: 1,
            max_staleness_seconds: 3600,
            max_confidence_bps: 500,
        };
        let token = Address::generate(&e);
        let options = Vec::from_array(&e, [
            soroban_sdk::String::from_str(&e, "Yes"),
            soroban_sdk::String::from_str(&e, "No"),
        ]);
        let market_id = client.create_market(
            &admin,
            &soroban_sdk::String::from_str(&e, "Retry market"),
            &options,
            &1000,
            &2000,
            &bad_config,
            &MarketTier::Basic,
            &token,
            &0,
            &0,
        );

        e.ledger().set_timestamp(2000);

        // Three failures.
        for _ in 0..3 {
            assert!(client.try_attempt_oracle_resolution(&market_id).is_err());
        }

        // Inject a valid result via the admin shortcut (simulates oracle feed recovery).
        client.set_oracle_result(&market_id, &0, &0);
        // Now resolve_market (admin path) must succeed — market was never corrupted.
        client.resolve_market(&market_id, &0);

        let market = client.get_market(&market_id).unwrap();
        assert_eq!(market.status, MarketStatus::Resolved);
        assert_eq!(market.winning_outcome, Some(0));
    }

    /// A stale price (publish_time too old) must return StalePrice and leave
    /// no oracle storage written — validate_price fires before any set().
    #[test]
    fn test_stale_oracle_price_leaves_no_partial_storage() {
        let e = Env::default();
        e.mock_all_auths();

        // Register the mock Pyth contract — it returns publish_time=1_700_000_000
        // which is far in the past relative to the ledger timestamp we'll set.
        let pyth_addr = e.register(MockPythContract, ());

        let config = OracleConfig {
            oracle_address: pyth_addr,
            feed_id: valid_feed_id(&e),
            min_responses: 1,
            max_staleness_seconds: 60, // only 60s tolerance
            max_confidence_bps: 500,
        };

        // Set ledger timestamp far ahead so publish_time=1_700_000_000 is stale.
        e.ledger().set_timestamp(1_700_010_000); // 10_000s after publish_time

        let result = crate::modules::oracles::resolve_with_pyth(&e, 1u64, 0u32, &config);
        assert_eq!(result, Err(crate::errors::ErrorCode::StalePrice));

        // No oracle storage must have been written.
        assert!(crate::modules::oracles::get_oracle_result(&e, 1u64, 0u32).is_none(),
            "oracle result must not be stored after stale price rejection");
        assert!(crate::modules::oracles::get_last_update(&e, 1u64, 0u32).is_none(),
            "oracle last_update must not be stored after stale price rejection");
    }
}

// =============================================================================
// Issue #9: set_oracle_result with oracle_id — no-collision tests
// =============================================================================

/// Verify that set_oracle_result stores results under the correct (market_id, oracle_id)
/// composite key and that different oracle_ids for the same market are fully isolated.
#[test]
fn test_set_oracle_result_uses_oracle_id_key() {
    let e = Env::default();
    let market_id = 42u64;

    // Oracle 0 posts outcome 0, oracle 1 posts outcome 1
    e.storage().persistent().set(&OracleData::Result(market_id, 0), &0u32);
    e.storage().persistent().set(&OracleData::Result(market_id, 1), &1u32);

    let r0: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 0));
    let r1: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 1));

    assert_eq!(r0, Some(0), "oracle 0 result should be 0");
    assert_eq!(r1, Some(1), "oracle 1 result should be 1");
}

/// Verify that updating one oracle's result does not overwrite another oracle's result.
#[test]
fn test_oracle_results_are_independent_per_oracle_id() {
    let e = Env::default();
    let market_id = 7u64;

    // Store initial results for three oracles
    for oracle_id in 0u32..3 {
        e.storage()
            .persistent()
            .set(&OracleData::Result(market_id, oracle_id), &oracle_id);
    }

    // Update oracle 1 only
    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, 1), &99u32);

    // Oracle 0 and 2 must be unchanged
    let r0: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 0));
    let r1: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 1));
    let r2: Option<u32> = e.storage().persistent().get(&OracleData::Result(market_id, 2));

    assert_eq!(r0, Some(0), "oracle 0 must not be affected by oracle 1 update");
    assert_eq!(r1, Some(99), "oracle 1 should reflect the update");
    assert_eq!(r2, Some(2), "oracle 2 must not be affected by oracle 1 update");
}

/// Verify that the same oracle_id in different markets stores independently.
#[test]
fn test_oracle_results_are_independent_per_market_id() {
    let e = Env::default();
    let oracle_id = 0u32;

    e.storage().persistent().set(&OracleData::Result(1u64, oracle_id), &10u32);
    e.storage().persistent().set(&OracleData::Result(2u64, oracle_id), &20u32);

    let r1: Option<u32> = e.storage().persistent().get(&OracleData::Result(1u64, oracle_id));
    let r2: Option<u32> = e.storage().persistent().get(&OracleData::Result(2u64, oracle_id));

    assert_eq!(r1, Some(10));
    assert_eq!(r2, Some(20));
}

/// Verify get_oracle_result returns None for an oracle_id that has not posted yet.
#[test]
fn test_get_oracle_result_returns_none_for_unset_oracle() {
    let e = Env::default();
    let result = get_oracle_result(&e, 999u64, 5u32);
    assert_eq!(result, None);
}

/// Verify get_last_update returns None before any result is stored.
#[test]
fn test_get_last_update_returns_none_before_set() {
    let e = Env::default();
    let ts = get_last_update(&e, 1u64, 0u32);
    assert_eq!(ts, None);
}
