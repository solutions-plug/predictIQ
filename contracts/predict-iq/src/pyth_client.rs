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

/// Compute `10^exp` using checked arithmetic.
///
/// Returns `None` if `exp` is negative or the result overflows `i128`.
fn checked_pow10(exp: u32) -> Option<i128> {
    let mut result: i128 = 1;
    let mut i: u32 = 0;
    while i < exp {
        result = result.checked_mul(10)?;
        i += 1;
    }
    Some(result)
}

/// Compare a Pyth [`Price`] against a `strike_price`, applying the Pyth exponent.
///
/// The raw `price.price` is scaled by `10^price.expo` before comparison.  Both
/// sides are normalised to a common scale (the smaller of the two exponents) so
/// that the comparison is exact and avoids overflow.  All arithmetic is checked;
/// the function returns `None` if any intermediate value overflows `i128`.
///
/// Returns `Some(true)` when the scaled Pyth price is strictly greater than
/// `strike_price`, `Some(false)` otherwise (including equality).
///
/// `strike_price` is interpreted with the same exponent as the Pyth price
/// (`price.expo`), i.e. it is expressed in the feed's native units.
pub fn price_above_strike(price: &Price, strike_price: i64) -> Option<bool> {
    // Normalise both sides to a common scale.  We pick the smaller exponent so
    // that neither side needs a fractional (negative-power) multiplier.
    let pyth_exp = price.expo;
    let strike_exp: i32 = 0;
    let common_exp = if pyth_exp < strike_exp { pyth_exp } else { strike_exp };

    // Scale factor applied to the Pyth raw price: 10^(pyth_exp - common_exp).
    let pyth_shift = (pyth_exp as i64).checked_sub(common_exp as i64)?;
    // Scale factor applied to the strike: 10^(strike_exp - common_exp).
    let strike_shift = (strike_exp as i64).checked_sub(common_exp as i64)?;

    if pyth_shift < 0 || strike_shift < 0 {
        return None;
    }

    let pyth_factor = checked_pow10(pyth_shift as u32)?;
    let strike_factor = checked_pow10(strike_shift as u32)?;

    let scaled_pyth = (price.price as i128).checked_mul(pyth_factor)?;
    let scaled_strike = (strike_price as i128).checked_mul(strike_factor)?;

    Some(scaled_pyth > scaled_strike)
}
