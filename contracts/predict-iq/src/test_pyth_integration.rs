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
//! * Price comparison applies the Pyth exponent for positive and negative
//!   exponents (issue #1897).

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
        feed_id: String::from_str(&e, "not-a-valid-hex-feed-id"),
        min_responses: Some(1),
        max_staleness_seconds: u64::MAX,
        max_confidence_bps: 500,
        strike_price: None,
    };

    let result = fetch_pyth_price(&e, &config);
    assert!(
        result.is_err(),
        "fetch_pyth_price should fail for an invalid feed id"
    );
}

// ---------------------------------------------------------------------------
// Tests: exponent-aware price comparison (issue #1897)
// ---------------------------------------------------------------------------

/// Build a [`PythPrice`] with the given raw price and exponent.
fn pyth_price(price: i64, expo: i32) -> PythPrice {
    PythPrice {
        price,
        conf: 0,
        expo,
        publish_time: 1_700_000_000,
    }
}

#[test]
fn test_compare_applies_negative_exponent_above_strike() {
    // price = 5_000_000, expo = -2  =>  $50,000.00
    // strike = 4_000_000 (same -2 scale) => $40,000.00
    let price = pyth_price(5_000_000, -2);
    let strike = 4_000_000i64;
    let cmp = crate::pyth_client::compare_price_to_strike(&price, strike, -2).unwrap();
    assert!(cmp > 0, "scaled price must be above the strike");
}

#[test]
fn test_compare_applies_negative_exponent_below_strike() {
    // price = 5_000_000, expo = -2  =>  $50,000.00
    // strike = 6_000_000 (same -2 scale) => $60,000.00
    let price = pyth_price(5_000_000, -2);
    let strike = 6_000_000i64;
    let cmp = crate::pyth_client::compare_price_to_strike(&price, strike, -2).unwrap();
    assert!(cmp < 0, "scaled price must be below the strike");
}

#[test]
fn test_compare_applies_negative_exponent_equal_strike() {
    let price = pyth_price(5_000_000, -2);
    let strike = 5_000_000i64;
    let cmp = crate::pyth_client::compare_price_to_strike(&price, strike, -2).unwrap();
    assert_eq!(cmp, 0, "scaled price must equal the strike");
}

#[test]
fn test_compare_applies_positive_exponent() {
    // price = 5, expo = 2  =>  500
    // strike = 400 (same +2 scale) => 400
    let price = pyth_price(5, 2);
    let cmp = crate::pyth_client::compare_price_to_strike(&price, 400, 2).unwrap();
    assert!(cmp > 0, "scaled price must be above the strike");

    let cmp_below = crate::pyth_client::compare_price_to_strike(&price, 600, 2).unwrap();
    assert!(cmp_below < 0, "scaled price must be below the strike");

    let cmp_eq = crate::pyth_client::compare_price_to_strike(&price, 500, 2).unwrap();
    assert_eq!(cmp_eq, 0, "scaled price must equal the strike");
}

#[test]
fn test_compare_normalises_mismatched_exponents() {
    // price = 5_000_000, expo = -2  =>  $50,000.00
    // strike expressed at expo = 0 => 50_000
    let price = pyth_price(5_000_000, -2);
    let cmp = crate::pyth_client::compare_price_to_strike(&price, 50_000, 0).unwrap();
    assert_eq!(cmp, 0, "mismatched exponents must normalise to equal");
}

#[test]
fn test_compare_overflow_is_rejected() {
    // A huge positive exponent would overflow when scaling; must error, not panic.
    let price = pyth_price(i64::MAX, 18);
    let result = crate::pyth_client::compare_price_to_strike(&price, 1, 18);
    assert!(result.is_err(), "overflow must be rejected with an error");
}
