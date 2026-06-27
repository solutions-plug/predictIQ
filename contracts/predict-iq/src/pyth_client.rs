//! Pyth Network oracle client for Soroban smart contracts.
//!
//! This module defines the cross-contract interface to the on-chain Pyth price-feed
//! contract deployed on Stellar/Soroban.  It mirrors the official Pyth Soroban SDK
//! interface so that the generated [`PythOracleClient`] can be used to query live
//! price data during market resolution.
//!
//! # Usage
//!
//! ```rust,ignore
//! use crate::pyth_client::{PythOracleClient, Price};
//!
//! let client = PythOracleClient::new(&env, &oracle_address);
//!
//! // Query the latest price for a feed, enforcing a maximum age.
//! let price: Price = client.get_price_no_older_than(&feed_id, &max_age_seconds);
//! ```
//!
//! # Feed IDs
//!
//! Each price feed is identified by a 32-byte [`BytesN<32>`] value.  Feed IDs are
//! stored in [`crate::types::OracleConfig::feed_id`] as a 64-character lowercase hex
//! string and decoded at call time by [`crate::modules::oracles::decode_feed_id`].
//!
//! # Staleness
//!
//! Two staleness-enforcement strategies are available:
//!
//! * **On-chain enforcement** – call [`PythOracleInterface::get_price_no_older_than`].
//!   The Pyth contract itself reverts if the price is older than `age_seconds`.
//!   This is the preferred path for production resolution.
//!
//! * **Off-chain enforcement** – call [`PythOracleInterface::get_price`] and then
//!   validate the returned [`Price::publish_time`] against
//!   [`crate::modules::oracles::validate_price`].  Used in tests and as a fallback.
//!
//! # Reference
//!
//! * Pyth Soroban SDK: <https://github.com/pyth-network/pyth-crosschain/tree/main/target_chains/stellar/sdk/soroban>
//! * Price feed IDs: <https://pyth.network/developers/price-feed-ids>

use soroban_sdk::{contractclient, contracttype, BytesN, Env};

/// A Pyth price with confidence interval, exponent, and publication timestamp.
///
/// The actual price is `price * 10^expo`.  For example, if `price = 5_000_000`,
/// `expo = -2`, the real-world price is `50_000.00`.
///
/// This struct mirrors the `Price` type in the official Pyth Soroban SDK.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Price {
    /// The price value scaled by `10^expo`.
    pub price: i64,
    /// The confidence interval (±) around `price`, in the same units.
    pub conf: u64,
    /// The power-of-ten exponent applied to `price` and `conf`.
    /// Typically negative (e.g. `-8` for most crypto feeds).
    pub expo: i32,
    /// Unix timestamp (seconds) when this price was published by the Pyth network.
    pub publish_time: i64,
}

/// Cross-contract interface to the on-chain Pyth price-feed contract.
///
/// The `#[contractclient]` macro generates a `PythOracleClient` struct that
/// issues cross-contract calls to the Pyth contract deployed at a given address.
///
/// Both methods accept a `feed_id: BytesN<32>` that uniquely identifies the
/// price feed (e.g. BTC/USD, ETH/USD).  Feed IDs are configurable per market
/// via [`crate::types::OracleConfig::feed_id`].
#[contractclient(name = "PythOracleClient")]
pub trait PythOracleInterface {
    /// Return the most recent price for `feed_id` regardless of age.
    ///
    /// Callers **must** validate [`Price::publish_time`] themselves using
    /// [`crate::modules::oracles::validate_price`] before trusting the result.
    /// Prefer [`get_price_no_older_than`] for production resolution paths.
    fn get_price(env: Env, feed_id: BytesN<32>) -> Price;

    /// Return the most recent price for `feed_id`, reverting if it is older
    /// than `age_seconds` seconds relative to the current ledger timestamp.
    ///
    /// This is the **preferred** method for market resolution because staleness
    /// is enforced atomically by the Pyth contract itself, eliminating any
    /// time-of-check / time-of-use window.
    ///
    /// # Errors
    ///
    /// The Pyth contract panics (contract error) if:
    /// * The feed ID is unknown.
    /// * The latest price is older than `age_seconds`.
    fn get_price_no_older_than(env: Env, feed_id: BytesN<32>, age_seconds: u64) -> Price;
}
