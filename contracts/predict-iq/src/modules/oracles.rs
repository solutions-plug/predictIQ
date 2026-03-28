use crate::errors::ErrorCode;
use crate::types::OracleConfig;
use soroban_sdk::{contracttype, symbol_short, BytesN, Env, Symbol};

const BPS_DENOMINATOR: u64 = 10_000;

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
    /// Issue #49: i64 to match the Pyth contract ABI; validated to u64 via cast_external_timestamp.
    pub publish_time: i64,
}

/// Raw price data returned by the Pyth on-chain contract.
/// Mirrors the `Price` struct in the Pyth Soroban contract ABI.
#[contracttype]
#[derive(Clone, Debug)]
struct RawPythPrice {
    price: i64,
    conf: u64,
    expo: i32,
    publish_time: i64,
}

/// Issue #25: Real Pyth cross-contract call via Soroban's invoke_contract.
///
/// The Pyth contract on Stellar exposes a `get_price` function that accepts a
/// 32-byte price feed ID and returns the latest aggregated price.
///
/// `config.oracle_address` — deployed Pyth contract address on this network.
/// `config.feed_id`        — hex-encoded 32-byte Pyth price feed ID stored as
///                           a `BytesN<32>` in the contract call.
///
/// On success the raw price is mapped into our internal `PythPrice` struct.
/// Any host-level error (contract not found, feed not supported, etc.) is
/// surfaced as `ErrorCode::OracleFailure`.
pub fn fetch_pyth_price(e: &Env, config: &OracleConfig) -> Result<PythPrice, ErrorCode> {
    // Decode the feed_id string into a 32-byte array expected by the Pyth contract.
    let feed_id: BytesN<32> = decode_feed_id(e, config)?;

    // Cross-contract call using try_invoke_contract so errors don't panic.
    // The Pyth Soroban contract's `get_price(feed_id: BytesN<32>)` returns
    // a struct with fields (price: i64, conf: u64, expo: i32, publish_time: i64).
    // We map it to our internal PythPrice.
    let raw: Result<Result<RawPythPrice, _>, _> = e.try_invoke_contract(
        &config.oracle_address,
        &Symbol::new(e, "get_price"),
        soroban_sdk::vec![e, feed_id.into()],
    );

    match raw {
        Ok(Ok(r)) => Ok(PythPrice {
            price: r.price,
            conf: r.conf,
            expo: r.expo,
            publish_time: r.publish_time,
        }),
        _ => Err(ErrorCode::OracleFailure),
    }
}

/// Decode the `feed_id` field of `OracleConfig` into a `BytesN<32>`.
/// (e.g. "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43").
/// We convert it to raw bytes for the Pyth contract call.
fn decode_feed_id(e: &Env, config: &OracleConfig) -> Result<BytesN<32>, ErrorCode> {
    let len = config.feed_id.len() as usize;
    if len != 64 {
        return Err(ErrorCode::OracleFailure);
    }

    // Copy the soroban String bytes into a stack buffer.
    let mut hex_buf = [0u8; 64];
    config.feed_id.copy_into_slice(&mut hex_buf);

    let mut bytes = [0u8; 32];
    for i in 0..32 {
        let hi = hex_nibble(hex_buf[i * 2]).ok_or(ErrorCode::OracleFailure)?;
        let lo = hex_nibble(hex_buf[i * 2 + 1]).ok_or(ErrorCode::OracleFailure)?;
        bytes[i] = (hi << 4) | lo;
    }

    Ok(BytesN::from_array(e, &bytes))
}

#[inline]
fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

pub fn cast_external_timestamp(timestamp: i64) -> Result<u64, ErrorCode> {
    timestamp
        .try_into()
        .map_err(|_| ErrorCode::InvalidTimestamp)
}

pub fn is_stale(current_time: u64, result_time: u64, max_age_seconds: u64) -> bool {
    current_time.saturating_sub(result_time) > max_age_seconds
}

/// Issue #41: Use saturating_abs to avoid overflow on i64::MIN.
/// Issue #49: cast publish_time from i64 (Pyth ABI) to u64 (ledger) safely.
pub fn validate_price(e: &Env, price: &PythPrice, config: &OracleConfig) -> Result<(), ErrorCode> {
    let current_time = e.ledger().timestamp(); // u64
    let publish_time = cast_external_timestamp(price.publish_time)?;
    let age = current_time.saturating_sub(publish_time);

    if age > config.max_staleness_seconds {
        return Err(ErrorCode::StalePrice);
    }

    // Issue #41: use abs_price_to_u64 to safely handle i64::MIN.
    // Use u128 arithmetic to prevent overflow when price_abs * max_confidence_bps
    // exceeds u64::MAX (e.g. price_abs = i64::MAX as u64, bps = 10_000).
    let price_abs = abs_price_to_u64(price.price);
    let max_conf = ((price_abs as u128 * config.max_confidence_bps as u128)
        / BPS_DENOMINATOR as u128)
        .min(u64::MAX as u128) as u64;

    if price.conf > max_conf {
        return Err(ErrorCode::ConfidenceTooLow);
    }

    Ok(())
}

/// Safe absolute value for i64 that avoids overflow on i64::MIN.
/// i64::MIN.abs() would panic in debug mode; saturating to i64::MAX instead.
pub fn abs_price_to_u64(price: i64) -> u64 {
    if price == i64::MIN {
        i64::MAX as u64
    } else {
        price.unsigned_abs()
    }
}

pub fn resolve_with_pyth(
    e: &Env,
    market_id: u64,
    oracle_id: u32,
    config: &OracleConfig,
) -> Result<u32, ErrorCode> {
    let price = fetch_pyth_price(e, config)?;

    // Validate freshness and confidence before storing.
    validate_price(e, &price, config)?;

    let outcome = determine_outcome(&price);
    let publish_time = cast_external_timestamp(price.publish_time)?;

    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, oracle_id), &outcome);
    e.storage().persistent().set(
        &OracleData::LastUpdate(market_id, oracle_id),
        &publish_time,
    );

    e.events().publish(
        (symbol_short!("oracle_ok"), market_id),
        (outcome, price.price, price.conf),
    );

    Ok(outcome)
}

/// Determine the winning outcome index from a validated Pyth price.
/// Outcome 0 = price positive (or zero), outcome 1 = price negative.
/// Markets with threshold-based resolution should override this via
/// market-specific configuration in a future iteration.
fn determine_outcome(price: &PythPrice) -> u32 {
    if price.price >= 0 { 0 } else { 1 }
}

/// Issue #9: oracle_id parameter added; callers use 0 for the primary oracle.
pub fn get_oracle_result(e: &Env, market_id: u64, oracle_id: u32) -> Option<u32> {
    e.storage()
        .persistent()
        .get(&OracleData::Result(market_id, oracle_id))
}

pub fn set_oracle_result(e: &Env, market_id: u64, oracle_id: u32, outcome: u32) -> Result<(), ErrorCode> {
    let market = crate::modules::markets::get_market(e, market_id)
        .ok_or(ErrorCode::MarketNotFound)?;
    if outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }

    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, oracle_id), &outcome);
    e.storage().persistent().set(
        &OracleData::LastUpdate(market_id, oracle_id),
        &e.ledger().timestamp(),
    );

    let oracle_addr = e.current_contract_address();
    crate::modules::events::emit_oracle_result_set(e, market_id, oracle_addr, outcome);

    Ok(())
}

/// Issue #9: Query the timestamp of the last update for a given (market_id, oracle_id) pair.
pub fn get_last_update(e: &Env, market_id: u64, oracle_id: u32) -> Option<u64> {
    e.storage()
        .persistent()
        .get(&OracleData::LastUpdate(market_id, oracle_id))
}

/// Validates that an `OracleConfig` is safe to use before any price fetch.
///
/// Checks beyond the previous "feed_id non-empty" guard:
/// - `feed_id` must be exactly 64 hex characters (32-byte Pyth ID).
/// - `max_confidence_bps` must be > 0 (zero BPS makes every price fail validation).
/// - `max_staleness_seconds` must be > 0 (zero window makes every price stale).
pub fn verify_oracle_health(_e: &Env, config: &OracleConfig) -> bool {
    if config.feed_id.len() != 64 {
        return false;
    }
    if config.max_confidence_bps == 0 {
        return false;
    }
    if config.max_staleness_seconds == 0 {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Env, String};

    fn valid_config(e: &Env) -> OracleConfig {
        OracleConfig {
            oracle_address: Address::generate(e),
            feed_id: String::from_str(
                e,
                "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43",
            ),
            min_responses: Some(1),
            max_staleness_seconds: 300,
            max_confidence_bps: 200,
        }
    }

    #[test]
    fn abs_price_handles_i64_min_without_panic() {
        assert_eq!(abs_price_to_u64(i64::MIN), i64::MAX as u64);
    }

    #[test]
    fn abs_price_preserves_normal_values() {
        assert_eq!(abs_price_to_u64(-123), 123);
        assert_eq!(abs_price_to_u64(456), 456);
        assert_eq!(abs_price_to_u64(0), 0);
    }

    #[test]
    fn hex_nibble_parses_all_valid_chars() {
        assert_eq!(hex_nibble(b'0'), Some(0));
        assert_eq!(hex_nibble(b'9'), Some(9));
        assert_eq!(hex_nibble(b'a'), Some(10));
        assert_eq!(hex_nibble(b'f'), Some(15));
        assert_eq!(hex_nibble(b'A'), Some(10));
        assert_eq!(hex_nibble(b'F'), Some(15));
        assert_eq!(hex_nibble(b'g'), None);
        assert_eq!(hex_nibble(b'z'), None);
    }

    // -------------------------------------------------------------------------
    // verify_oracle_health — parameter validation
    // -------------------------------------------------------------------------

    #[test]
    fn health_check_passes_for_valid_config() {
        let e = Env::default();
        assert!(verify_oracle_health(&e, &valid_config(&e)));
    }

    #[test]
    fn health_check_rejects_empty_feed_id() {
        let e = Env::default();
        let mut cfg = valid_config(&e);
        cfg.feed_id = String::from_str(&e, "");
        assert!(!verify_oracle_health(&e, &cfg));
    }

    #[test]
    fn health_check_rejects_short_feed_id() {
        let e = Env::default();
        let mut cfg = valid_config(&e);
        // 32 chars — half the required 64
        cfg.feed_id = String::from_str(&e, "e62df6c8b4a85fe1a67db44dc12de5db");
        assert!(!verify_oracle_health(&e, &cfg));
    }

    #[test]
    fn health_check_rejects_long_feed_id() {
        let e = Env::default();
        let mut cfg = valid_config(&e);
        // 65 chars — one too many
        cfg.feed_id = String::from_str(
            &e,
            "e62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b430",
        );
        assert!(!verify_oracle_health(&e, &cfg));
    }

    #[test]
    fn health_check_rejects_zero_confidence_bps() {
        let e = Env::default();
        let mut cfg = valid_config(&e);
        cfg.max_confidence_bps = 0;
        assert!(
            !verify_oracle_health(&e, &cfg),
            "zero max_confidence_bps makes every price fail validation — must be rejected"
        );
    }

    #[test]
    fn health_check_rejects_zero_staleness_window() {
        let e = Env::default();
        let mut cfg = valid_config(&e);
        cfg.max_staleness_seconds = 0;
        assert!(
            !verify_oracle_health(&e, &cfg),
            "zero max_staleness_seconds makes every price stale — must be rejected"
        );
    }

    #[test]
    fn health_check_rejects_both_zero_bps_and_zero_staleness() {
        let e = Env::default();
        let mut cfg = valid_config(&e);
        cfg.max_confidence_bps = 0;
        cfg.max_staleness_seconds = 0;
        assert!(!verify_oracle_health(&e, &cfg));
    }
}
