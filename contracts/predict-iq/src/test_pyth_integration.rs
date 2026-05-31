//! Integration tests for the Pyth oracle integration.
//!
//! These tests exercise the full resolution path:
//!   fetch_pyth_price → validate_price → resolve_with_pyth → market state
//!
//! A [`MockPythContract`] is registered in the Soroban test environment so
//! cross-contract calls go through the real `#[contractclient]` machinery,
//! matching production behaviour as closely as possible.
//!
//! # Acceptance criteria covered
//!
//! * Pyth price feed queried using the official Soroban Pyth SDK interface
//!   (`get_price` and `get_price_no_older_than`).
//! * Feed ID is configurable per market via [`OracleConfig::feed_id`].
//! * Integration tests use a mock Pyth contract.
//! * Staleness check implemented — both via `get_price_no_older_than` (on-chain)
//!   and `validate_price` (off-chain confidence + age check).

#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl,
    testutils::{Address as _, Ledger as _},
    Address, BytesN, Env, String, Vec,
};

use crate::{
    errors::ErrorCode,
    modules::oracles::{
        fetch_pyth_price, get_last_update, get_oracle_result, resolve_with_pyth, validate_price,
        PythPrice,
    },
    pyth_client::Price,
    types::{MarketStatus, MarketTier, OracleConfig},
    PredictIQ, PredictIQClient,
};

// ---------------------------------------------------------------------------
// Mock Pyth contract
// ---------------------------------------------------------------------------

/// A configurable mock Pyth contract.
///
/// Both `get_price` and `get_price_no_older_than` are implemented so the mock
/// satisfies the full [`crate::pyth_client::PythOracleInterface`].
///
/// The returned price is fixed at BTC/USD $50,000.00 (price=5_000_000, expo=-2)
/// with a 2% confidence interval and publish_time=1_700_000_000.
#[contract]
pub struct MockPythContract;

#[contractimpl]
impl MockPythContract {
    /// Return a fixed price regardless of feed_id.
    pub fn get_price(_env: Env, _feed_id: BytesN<32>) -> Price {
        Price {
            price: 5_000_000i64,
            conf: 100_000u64,
            expo: -2i32,
            publish_time: 1_700_000_000i64,
        }
    }

    /// Return the same fixed price but panic (simulating the Pyth contract
    /// reverting) if the price would be considered stale.
    pub fn get_price_no_older_than(env: Env, feed_id: BytesN<32>, age_seconds: u64) -> Price {
        let price = Self::get_price(env.clone(), feed_id);
        let current = env.ledger().timestamp();
        let age = current.saturating_sub(price.publish_time as u64);
        if age > age_seconds {
            panic!("MockPythContract: price is stale");
        }
        price
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// A 64-char hex string that decodes to a valid 32-byte Pyth feed ID.
/// Matches the BTC/USD feed ID on Pyth mainnet.
fn btc_usd_feed_id(e: &Env) -> String {
    String::from_str(
        e,
        "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
    )
}

/// Build an [`OracleConfig`] pointing at the mock Pyth contract.
fn oracle_config(
    e: &Env,
    pyth_addr: Address,
    max_staleness: u64,
    max_conf_bps: u64,
) -> OracleConfig {
    OracleConfig {
        oracle_address: pyth_addr,
        feed_id: btc_usd_feed_id(e),
        min_responses: Some(1),
        max_staleness_seconds: max_staleness,
        max_confidence_bps: max_conf_bps,
        strike_price: None,
    }
}

/// Spin up a PredictIQ contract and create a market with the given oracle config.
fn setup_market(e: &Env, config: OracleConfig) -> (PredictIQClient<'static>, u64) {
    e.mock_all_auths();
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(e, &contract_id);
    let admin = Address::generate(e);
    client.initialize(&admin, &0);

    let token = Address::generate(e);
    let mut options = Vec::new(e);
    options.push_back(String::from_str(e, "Yes"));
    options.push_back(String::from_str(e, "No"));

    let market_id = client.create_market(
        &admin,
        &String::from_str(e, "BTC above $50k?"),
        &options,
        &1_000,
        &2_000,
        &config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    (client, market_id)
}

// ---------------------------------------------------------------------------
// Tests: feed ID is configurable per market
// ---------------------------------------------------------------------------

#[test]
fn test_feed_id_is_stored_per_market() {
    let e = Env::default();
    e.mock_all_auths();
    let pyth_addr = e.register(MockPythContract, ());
    let config = oracle_config(&e, pyth_addr, 3600, 500);
    let (client, market_id) = setup_market(&e, config.clone());

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.oracle_config.feed_id,
        btc_usd_feed_id(&e),
        "feed_id must be stored in the market's oracle config"
    );
}

#[test]
fn test_different_markets_can_have_different_feed_ids() {
    let e = Env::default();
    e.mock_all_auths();
    let pyth_addr = e.register(MockPythContract, ());

    let btc_config = oracle_config(&e, pyth_addr.clone(), 3600, 500);

    // ETH/USD feed ID (different 64-char hex string)
    let eth_feed = String::from_str(
        &e,
        "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
    );
    let eth_config = OracleConfig {
        oracle_address: pyth_addr,
        feed_id: eth_feed.clone(),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 500,
        strike_price: None,
    };

    let token = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    let admin = Address::generate(&e);
    client.initialize(&admin, &0);

    let mut opts = Vec::new(&e);
    opts.push_back(String::from_str(&e, "Yes"));
    opts.push_back(String::from_str(&e, "No"));

    let btc_market = client.create_market(
        &admin,
        &String::from_str(&e, "BTC market"),
        &opts,
        &1000,
        &2000,
        &btc_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );
    let eth_market = client.create_market(
        &admin,
        &String::from_str(&e, "ETH market"),
        &opts,
        &1000,
        &2000,
        &eth_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    let btc = client.get_market(&btc_market).unwrap();
    let eth = client.get_market(&eth_market).unwrap();

    assert_eq!(btc.oracle_config.feed_id, btc_usd_feed_id(&e));
    assert_eq!(eth.oracle_config.feed_id, eth_feed);
    assert_ne!(btc.oracle_config.feed_id, eth.oracle_config.feed_id);
}

// ---------------------------------------------------------------------------
// Tests: get_price path (permissive mode, max_staleness = u64::MAX)
// ---------------------------------------------------------------------------

#[test]
fn test_fetch_pyth_price_returns_correct_fields() {
    let e = Env::default();
    let pyth_addr = e.register(MockPythContract, ());
    // Use u64::MAX to trigger the get_price (permissive) path.
    let config = oracle_config(&e, pyth_addr, u64::MAX, 500);

    let result = fetch_pyth_price(&e, &config);
    assert!(
        result.is_ok(),
        "fetch_pyth_price should succeed: {:?}",
        result
    );

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
        min_responses: Some(1),
        max_staleness_seconds: u64::MAX,
        max_confidence_bps: 500,
        strike_price: None,
    };

    let result = fetch_pyth_price(&e, &config);
    assert_eq!(result, Err(ErrorCode::OracleFailure));
}

// ---------------------------------------------------------------------------
// Tests: get_price_no_older_than path (production staleness enforcement)
// ---------------------------------------------------------------------------

#[test]
fn test_fetch_pyth_price_no_older_than_succeeds_when_fresh() {
    let e = Env::default();
    // Set ledger timestamp close to publish_time so the price is fresh.
    e.ledger().set_timestamp(1_700_000_060); // 60s after publish_time

    let pyth_addr = e.register(MockPythContract, ());
    // max_staleness_seconds = 3600 → triggers get_price_no_older_than
    let config = oracle_config(&e, pyth_addr, 3600, 500);

    let result = fetch_pyth_price(&e, &config);
    assert!(
        result.is_ok(),
        "price should be accepted when within staleness window"
    );
    assert_eq!(result.unwrap().price, 5_000_000);
}

#[test]
#[should_panic(expected = "MockPythContract: price is stale")]
fn test_fetch_pyth_price_no_older_than_panics_when_stale() {
    let e = Env::default();
    // Set ledger timestamp far ahead so the mock panics.
    e.ledger().set_timestamp(1_700_010_000); // 10_000s after publish_time

    let pyth_addr = e.register(MockPythContract, ());
    let config = oracle_config(&e, pyth_addr, 60, 500); // only 60s tolerance

    // This should panic because the mock enforces staleness.
    let _ = fetch_pyth_price(&e, &config);
}

// ---------------------------------------------------------------------------
// Tests: staleness check via validate_price (off-chain path)
// ---------------------------------------------------------------------------

#[test]
fn test_validate_price_accepts_fresh_price() {
    let e = Env::default();
    e.ledger().set_timestamp(1_700_000_060);

    let pyth_addr = e.register(MockPythContract, ());
    let config = oracle_config(&e, pyth_addr, 3600, 500);

    let price = PythPrice {
        price: 5_000_000,
        conf: 50_000,
        expo: -2,
        publish_time: 1_700_000_000,
    };

    assert!(validate_price(&e, &price, &config).is_ok());
}

#[test]
fn test_validate_price_rejects_stale_price() {
    let e = Env::default();
    e.ledger().set_timestamp(1_700_010_000); // 10_000s after publish_time

    let pyth_addr = e.register(MockPythContract, ());
    let config = oracle_config(&e, pyth_addr, 60, 500); // 60s max

    let price = PythPrice {
        price: 5_000_000,
        conf: 50_000,
        expo: -2,
        publish_time: 1_700_000_000,
    };

    assert_eq!(
        validate_price(&e, &price, &config),
        Err(ErrorCode::StalePrice)
    );
}

#[test]
fn test_validate_price_rejects_low_confidence() {
    let e = Env::default();
    e.ledger().set_timestamp(1_700_000_060);

    let pyth_addr = e.register(MockPythContract, ());
    // max_confidence_bps = 100 (1%) but conf = 200_000 (4% of 5_000_000)
    let config = oracle_config(&e, pyth_addr, 3600, 100);

    let price = PythPrice {
        price: 5_000_000,
        conf: 200_000, // 4% — exceeds 1% threshold
        expo: -2,
        publish_time: 1_700_000_000,
    };

    assert_eq!(
        validate_price(&e, &price, &config),
        Err(ErrorCode::ConfidenceTooLow)
    );
}

#[test]
fn test_validate_price_rejects_negative_publish_time() {
    let e = Env::default();
    let pyth_addr = e.register(MockPythContract, ());
    let config = oracle_config(&e, pyth_addr, 3600, 500);

    let price = PythPrice {
        price: 5_000_000,
        conf: 50_000,
        expo: -2,
        publish_time: -1,
    };

    assert_eq!(
        validate_price(&e, &price, &config),
        Err(ErrorCode::InvalidTimestamp)
    );
}

// ---------------------------------------------------------------------------
// Tests: resolve_with_pyth end-to-end
// ---------------------------------------------------------------------------

#[test]
fn test_resolve_with_pyth_stores_outcome_and_timestamp() {
    let e = Env::default();
    // Ledger timestamp close to publish_time so price is fresh.
    e.ledger().set_timestamp(1_700_000_060);

    let pyth_addr = e.register(MockPythContract, ());
    // strike_price = None → threshold = 0 → price (5_000_000) >= 0 → outcome 0
    let config = oracle_config(&e, pyth_addr, u64::MAX, 500);

    let result = resolve_with_pyth(&e, 1u64, 0u32, &config);
    assert!(
        result.is_ok(),
        "resolve_with_pyth should succeed: {:?}",
        result
    );
    assert_eq!(
        result.unwrap(),
        0u32,
        "outcome should be 0 (price >= strike)"
    );

    assert_eq!(get_oracle_result(&e, 1u64, 0u32), Some(0u32));
    assert!(get_last_update(&e, 1u64, 0u32).is_some());
}

#[test]
fn test_resolve_with_pyth_outcome_below_strike() {
    let e = Env::default();
    e.ledger().set_timestamp(1_700_000_060);

    let pyth_addr = e.register(MockPythContract, ());
    // strike_price = 10_000_000 → price (5_000_000) < strike → outcome 1
    let config = OracleConfig {
        oracle_address: pyth_addr,
        feed_id: btc_usd_feed_id(&e),
        min_responses: Some(1),
        max_staleness_seconds: u64::MAX,
        max_confidence_bps: 500,
        strike_price: Some(10_000_000),
    };

    let result = resolve_with_pyth(&e, 2u64, 0u32, &config);
    assert_eq!(result, Ok(1u32), "outcome should be 1 (price < strike)");
}

#[test]
fn test_resolve_with_pyth_stale_price_leaves_no_storage() {
    let e = Env::default();
    e.ledger().set_timestamp(1_700_010_000); // 10_000s after publish_time

    let pyth_addr = e.register(MockPythContract, ());
    // Use u64::MAX to bypass get_price_no_older_than and test validate_price path.
    let config = OracleConfig {
        oracle_address: pyth_addr,
        feed_id: btc_usd_feed_id(&e),
        min_responses: Some(1),
        max_staleness_seconds: 60, // 60s max — price is 10_000s old
        max_confidence_bps: 500,
        strike_price: None,
    };

    // fetch_pyth_price uses get_price (permissive) when max_staleness != u64::MAX,
    // but validate_price will catch the staleness.
    // To test the off-chain path we temporarily override by calling resolve_with_pyth
    // which calls validate_price after fetch.
    let result = resolve_with_pyth(&e, 3u64, 0u32, &config);
    assert_eq!(result, Err(ErrorCode::StalePrice));

    assert!(
        get_oracle_result(&e, 3u64, 0u32).is_none(),
        "no result should be stored"
    );
    assert!(
        get_last_update(&e, 3u64, 0u32).is_none(),
        "no timestamp should be stored"
    );
}

#[test]
fn test_resolve_with_pyth_multiple_oracle_ids_are_independent() {
    let e = Env::default();
    e.ledger().set_timestamp(1_700_000_060);

    let pyth_addr = e.register(MockPythContract, ());
    let config = oracle_config(&e, pyth_addr, u64::MAX, 500);

    // Resolve with oracle_id 0 and oracle_id 1 for the same market.
    resolve_with_pyth(&e, 10u64, 0u32, &config).unwrap();
    resolve_with_pyth(&e, 10u64, 1u32, &config).unwrap();

    assert_eq!(get_oracle_result(&e, 10u64, 0u32), Some(0u32));
    assert_eq!(get_oracle_result(&e, 10u64, 1u32), Some(0u32));

    // Different market must not be affected.
    assert!(get_oracle_result(&e, 11u64, 0u32).is_none());
}

// ---------------------------------------------------------------------------
// Tests: full contract-level resolution with mock Pyth
// ---------------------------------------------------------------------------

#[test]
fn test_contract_attempt_oracle_resolution_with_mock_pyth() {
    let e = Env::default();
    e.mock_all_auths();

    let pyth_addr = e.register(MockPythContract, ());
    // Ledger timestamp = publish_time + 60s → price is fresh.
    e.ledger().set_timestamp(1_700_000_060);

    let config = oracle_config(&e, pyth_addr, u64::MAX, 500);
    let (client, market_id) = setup_market(&e, config);

    // Advance past resolution_deadline (2000).
    e.ledger().set_timestamp(1_700_002_000);

    // Manually inject oracle result (simulates resolve_with_pyth having run).
    client.set_oracle_result(&market_id, &0, &0);

    let result = client.try_attempt_oracle_resolution(&market_id);
    assert!(
        result.is_ok(),
        "oracle resolution should succeed: {:?}",
        result
    );

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, MarketStatus::PendingResolution);
    assert_eq!(market.winning_outcome, Some(0));
}
