use crate::errors::ErrorCode;
use crate::types::{MarketStatus, OracleConfig};
use soroban_sdk::{contracttype, symbol_short, Bytes, Env, Map};

pub const MAX_STALENESS: u64 = 60;
pub const MAX_STALENESS_SECONDS: u64 = MAX_STALENESS;

#[contracttype]
pub enum OracleData {
    Result(u64, u32),     // market_id -> outcome
    LastUpdate(u64, u64), // market_id -> timestamp
    OracleResponses(u64), // market_id -> Map<oracle_index, outcome>
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PythPrice {
    pub price: i64,
    pub conf: u64,
    pub expo: i32,
    pub publish_time: i64,
}

/// Decode a 64-char hex feed_id string into a 32-byte BytesN<32>.
fn decode_feed_id(
    e: &Env,
    feed_id: &soroban_sdk::String,
) -> Result<soroban_sdk::BytesN<32>, ErrorCode> {
    if feed_id.len() != 64 {
        return Err(ErrorCode::OracleFailure);
    }
    let bytes = Bytes::from(feed_id.clone());
    let mut buf = [0u8; 32];
    for i in 0..32 {
        let hi = hex_char(bytes.get(i * 2).ok_or(ErrorCode::OracleFailure)?)?;
        let lo = hex_char(bytes.get(i * 2 + 1).ok_or(ErrorCode::OracleFailure)?)?;
        buf[i as usize] = (hi << 4) | lo;
    }
    Ok(soroban_sdk::BytesN::from_array(e, &buf))
}

fn hex_char(c: u8) -> Result<u8, ErrorCode> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(ErrorCode::OracleFailure),
    }
}

/// Call the on-chain Pyth price-feed contract using config.oracle_address and config.feed_id.
pub fn fetch_pyth_price(e: &Env, config: &OracleConfig) -> Result<PythPrice, ErrorCode> {
    let feed_id = decode_feed_id(e, &config.feed_id)?;
    let client = crate::pyth_client::PythOracleClient::new(e, &config.oracle_address);
    let (price, conf, expo, publish_time) = client.get_price(&feed_id);
    Ok(PythPrice {
        price,
        conf,
        expo,
        publish_time,
    })
}

pub fn validate_price(e: &Env, price: &PythPrice, config: &OracleConfig) -> Result<(), ErrorCode> {
    let publish_time = cast_external_timestamp(price.publish_time)?;
    let current_time = e.ledger().timestamp();

    if is_stale(current_time, publish_time, effective_max_staleness(config)) {
        return Err(ErrorCode::StalePrice);
    }

    let price_abs = abs_price_to_u64(price.price);
    let max_conf = (price_abs.saturating_mul(config.max_confidence_bps)) / 10000;

    if price.conf > max_conf {
        return Err(ErrorCode::ConfidenceTooLow);
    }

    Ok(())
}

/// Issue #508: Validate oracle staleness before resolution — checks all configured oracle indices.
pub fn validate_oracle_staleness(
    e: &Env,
    market_id: u64,
    config: &OracleConfig,
) -> Result<(), ErrorCode> {
    let num_oracles = config.min_responses.unwrap_or(1);
    let current_time = e.ledger().timestamp();
    let max_staleness = effective_max_staleness(config);
    let mut found = 0u32;

    for idx in 0..num_oracles {
        let last_update = e
            .storage()
            .persistent()
            .get::<_, u64>(&OracleData::LastUpdate(market_id, idx as u64));

        if let Some(update_time) = last_update {
            found += 1;
            let age = current_time.saturating_sub(update_time);
            if age > max_staleness {
                crate::modules::monitoring::track_error(e);
                return Err(ErrorCode::StalePrice);
            }
        }
    }

    // Issue #1707: every one of the min_responses oracles must have reported.
    if found >= num_oracles {
        Ok(())
    } else {
        crate::modules::monitoring::track_error(e);
        Err(ErrorCode::OracleFailure)
    }
}

pub fn resolve_with_pyth(
    e: &Env,
    market_id: u64,
    oracle_id: u32,
    config: &OracleConfig,
) -> Result<u32, ErrorCode> {
    let price = fetch_pyth_price(e, config)?;
    validate_price(e, &price, config)?;

    let outcome = determine_outcome(&price, config)?;

    let publish_time = cast_external_timestamp(price.publish_time)?;

    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, oracle_id), &outcome);
    e.storage().persistent().set(
        &OracleData::LastUpdate(market_id, oracle_id as u64),
        &publish_time,
    );
    record_oracle_response(e, market_id, oracle_id, outcome)?;

    e.events().publish(
        (
            symbol_short!("oracle_ok"),
            market_id,
            config.oracle_address.clone(),
        ),
        (outcome, price.price, price.conf),
    );

    Ok(outcome)
}

/// Issue #1706: `strike_price` is expressed in `STRIKE_EXPO` (1e-8) fixed point.
/// The feed price (`price * 10^expo`) is normalised to that scale before comparing,
/// so feeds with any exponent (e.g. -8 for BTC, -2 for some FX feeds) resolve correctly.
pub const STRIKE_EXPO: i32 = -8;

/// Rescale `price` from `10^expo` to `10^STRIKE_EXPO` using checked arithmetic.
///
/// Scale: `strike_price` is an integer in units of 1e-8 (i.e. `strike_price = 6_500_000_000_000`
/// means 65,000.00000000). Returns `ArithmeticOverflow` if rescaling would overflow.
pub fn normalize_price_checked(price: i64, expo: i32) -> Result<i128, ErrorCode> {
    let diff = expo
        .checked_sub(STRIKE_EXPO)
        .ok_or(ErrorCode::ArithmeticOverflow)?;
    let p = price as i128;
    if diff >= 0 {
        let m = 10i128
            .checked_pow(diff as u32)
            .ok_or(ErrorCode::ArithmeticOverflow)?;
        p.checked_mul(m).ok_or(ErrorCode::ArithmeticOverflow)
    } else {
        // Dividing by a power of ten too large for i128 truncates any i64 price to 0.
        match 10i128.checked_pow(diff.unsigned_abs()) {
            Some(d) => Ok(p / d),
            None => Ok(0),
        }
    }
}

/// Saturating variant of [`normalize_price_checked`] (kept for callers that cannot fail).
pub fn normalize_price(price: i64, expo: i32) -> i128 {
    match normalize_price_checked(price, expo) {
        Ok(v) => v,
        Err(_) if price < 0 => i128::MIN,
        Err(_) => i128::MAX,
    }
}

/// Outcome 0 if the normalised price is >= strike, otherwise 1.
pub fn determine_outcome(price: &PythPrice, config: &OracleConfig) -> Result<u32, ErrorCode> {
    let threshold = config.strike_price.unwrap_or(0) as i128;
    if normalize_price_checked(price.price, price.expo)? >= threshold {
        Ok(0)
    } else {
        Ok(1)
    }
}

pub fn get_oracle_result(e: &Env, market_id: u64, oracle_id: u32) -> Option<u32> {
    e.storage()
        .persistent()
        .get(&OracleData::Result(market_id, oracle_id))
}

pub fn get_last_update(e: &Env, market_id: u64, oracle_id: u32) -> Option<u64> {
    e.storage()
        .persistent()
        .get(&OracleData::LastUpdate(market_id, oracle_id as u64))
}

pub fn set_oracle_result(
    e: &Env,
    market_id: u64,
    oracle_id: u32,
    outcome: u32,
) -> Result<(), ErrorCode> {
    let market =
        crate::modules::markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    if !matches!(
        market.status,
        MarketStatus::Active | MarketStatus::PendingResolution
    ) {
        return Err(ErrorCode::MarketNotActive);
    }
    if outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }
    if oracle_id >= market.oracle_config.min_responses.unwrap_or(1) {
        return Err(ErrorCode::OracleFailure);
    }

    e.storage()
        .persistent()
        .set(&OracleData::Result(market_id, oracle_id), &outcome);
    e.storage().persistent().set(
        &OracleData::LastUpdate(market_id, oracle_id as u64),
        &e.ledger().timestamp(),
    );
    record_oracle_response(e, market_id, oracle_id, outcome)?;

    let oracle_addr = market.oracle_config.oracle_address;
    crate::modules::events::emit_oracle_result_set(e, market_id, oracle_id, oracle_addr, outcome);

    Ok(())
}

/// Convert i64 timestamp to u64, rejecting negative values.
pub fn cast_external_timestamp(ts: i64) -> Result<u64, ErrorCode> {
    if ts < 0 {
        Err(ErrorCode::InvalidTimestamp)
    } else {
        Ok(ts as u64)
    }
}

/// Check if oracle data is stale (age > max_staleness_seconds).
pub fn is_stale(current_time: u64, result_time: u64, max_staleness_seconds: u64) -> bool {
    let age = current_time.saturating_sub(result_time);
    age > max_staleness_seconds
}

fn effective_max_staleness(config: &OracleConfig) -> u64 {
    if config.max_staleness_seconds < MAX_STALENESS_SECONDS {
        config.max_staleness_seconds
    } else {
        MAX_STALENESS_SECONDS
    }
}

/// Convert i64 price to u64 absolute value, saturating i64::MIN to i64::MAX as u64.
pub fn abs_price_to_u64(price: i64) -> u64 {
    if price == i64::MIN {
        i64::MAX as u64
    } else if price < 0 {
        (-price) as u64
    } else {
        price as u64
    }
}

/// Issue #1554: Validate that an oracle config is usable before a market accepts bets.
///
/// Unlike a mere non-emptiness check, this verifies the `feed_id` actually decodes
/// into a valid 32-byte feed identifier (exactly 64 hex characters). This catches
/// malformed feed_ids at market-creation time instead of only failing later at
/// resolution via `decode_feed_id`.
pub fn verify_oracle_health(e: &Env, config: &OracleConfig) -> bool {
    decode_feed_id(e, &config.feed_id).is_ok()
}

/// Issue #1554: Reject market creation when the oracle config is malformed.
pub fn validate_oracle_config(e: &Env, config: &OracleConfig) -> Result<(), ErrorCode> {
    if verify_oracle_health(e, config) {
        Ok(())
    } else {
        Err(ErrorCode::OracleFailure)
    }
}

/// Issue #509: Record an oracle response for consensus validation
pub fn record_oracle_response(
    e: &Env,
    market_id: u64,
    oracle_index: u32,
    outcome: u32,
) -> Result<(), ErrorCode> {
    let key = OracleData::OracleResponses(market_id);
    let mut responses: Map<u32, u32> = e
        .storage()
        .persistent()
        .get(&key)
        .unwrap_or_else(|| Map::new(e));

    responses.set(oracle_index, outcome);
    e.storage().persistent().set(&key, &responses);

    Ok(())
}

/// Issue #509: Validate oracle consensus - requires min_responses confirmations
pub fn validate_consensus(
    e: &Env,
    market_id: u64,
    config: &OracleConfig,
) -> Result<u32, ErrorCode> {
    let min_responses = config.min_responses.unwrap_or(1);

    let key = OracleData::OracleResponses(market_id);
    let responses: Map<u32, u32> = e
        .storage()
        .persistent()
        .get(&key)
        .ok_or(ErrorCode::OracleFailure)?;

    // Check if we have enough responses
    if responses.len() < min_responses {
        return Err(ErrorCode::OracleFailure);
    }

    // Count votes for each outcome.
    // Fix #1539: iterate by actual (oracle_index, outcome) pairs via Map::iter()
    // instead of a synthetic positional index. Map::get(i) is a key lookup, not
    // positional — with non-contiguous oracle indices the old loop silently
    // skipped entries whose key didn't match the loop counter.
    let mut outcome_votes: Map<u32, u32> = Map::new(e);
    for (_oracle_index, outcome) in responses.iter() {
        let votes = outcome_votes.get(outcome).unwrap_or(0);
        outcome_votes.set(outcome, votes + 1);
    }

    // Find outcome with most votes (quorum).
    // Fix #1539: iterate outcome_votes by actual (outcome_id, vote_count) pairs.
    // The old loop used outcome_votes.get(i) which probes key i, not position i,
    // so non-contiguous outcome IDs (e.g. outcomes 0 and 2 with len==2) would
    // never check key 2 — its votes were silently ignored.
    let mut consensus_outcome: Option<u32> = None;
    let mut max_votes = 0u32;
    for (outcome_id, votes) in outcome_votes.iter() {
        if votes > max_votes {
            max_votes = votes;
            consensus_outcome = Some(outcome_id);
        }
    }

    let winning_outcome = consensus_outcome.ok_or(ErrorCode::OracleFailure)?;

    Ok(winning_outcome)
}

/// Permissionless: fetch the Pyth price for an Active market past its
/// resolution deadline and store it as oracle 0's result.
pub fn resolve_via_oracle(e: &Env, market_id: u64) -> Result<u32, ErrorCode> {
    let market =
        crate::modules::markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    if market.status != MarketStatus::Active {
        return Err(ErrorCode::MarketNotActive);
    }
    if e.ledger().timestamp() < market.resolution_deadline {
        return Err(ErrorCode::ResolutionNotReady);
    }
    let outcome = resolve_with_pyth(e, market_id, 0, &market.oracle_config)?;
    if outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }
    Ok(outcome)
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
                "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
            ),
            max_staleness_seconds: MAX_STALENESS_SECONDS,
            max_confidence_bps: 100,
            strike_price: Some(100),
            min_responses: Some(1),
        }
    }

    #[test]
    fn verify_oracle_health_accepts_valid_feed_id() {
        let e = Env::default();
        let config = valid_config(&e);
        assert!(verify_oracle_health(&e, &config));
        assert!(validate_oracle_config(&e, &config).is_ok());
    }

    #[test]
    fn verify_oracle_health_rejects_wrong_length_feed_id() {
        let e = Env::default();
        let mut config = valid_config(&e);
        config.feed_id = String::from_str(&e, "0123456789abcdef");
        assert!(!verify_oracle_health(&e, &config));
        assert_eq!(
            validate_oracle_config(&e, &config),
            Err(ErrorCode::OracleFailure)
        );
    }

    #[test]
    fn verify_oracle_health_rejects_non_hex_feed_id() {
        let e = Env::default();
        let mut config = valid_config(&e);
        config.feed_id = String::from_str(
            &e,
            "zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz",
        );
        assert!(!verify_oracle_health(&e, &config));
        assert_eq!(
            validate_oracle_config(&e, &config),
            Err(ErrorCode::OracleFailure)
        );
    }

    #[test]
    fn verify_oracle_health_rejects_empty_feed_id() {
        let e = Env::default();
        let mut config = valid_config(&e);
        config.feed_id = String::from_str(&e, "");
        assert!(!verify_oracle_health(&e, &config));
        assert_eq!(
            validate_oracle_config(&e, &config),
            Err(ErrorCode::OracleFailure)
        );
    }

    /// Issue #1707: min_responses = 3 with a single fresh report must be rejected.
    #[test]
    fn staleness_requires_min_responses_reports() {
        let e = Env::default();
        let id = e.register(crate::PredictIQ, ());
        let mut config = valid_config(&e);
        config.min_responses = Some(3);
        e.as_contract(&id, || {
            e.storage()
                .persistent()
                .set(&OracleData::LastUpdate(1, 0), &e.ledger().timestamp());
            assert_eq!(
                validate_oracle_staleness(&e, 1, &config),
                Err(ErrorCode::OracleFailure)
            );
            for i in 1..3u64 {
                e.storage()
                    .persistent()
                    .set(&OracleData::LastUpdate(1, i), &e.ledger().timestamp());
            }
            assert_eq!(validate_oracle_staleness(&e, 1, &config), Ok(()));
        });
    }

    /// Issue #1960: zero, partial and full response sets.
    #[test]
    fn staleness_zero_partial_full_responses() {
        let e = Env::default();
        let id = e.register(crate::PredictIQ, ());
        let mut config = valid_config(&e);
        config.min_responses = Some(2);
        e.as_contract(&id, || {
            assert_eq!(validate_oracle_staleness(&e, 1, &config), Err(ErrorCode::OracleFailure));
            e.storage().persistent().set(&OracleData::LastUpdate(1, 0), &e.ledger().timestamp());
            assert_eq!(validate_oracle_staleness(&e, 1, &config), Err(ErrorCode::OracleFailure));
            // Re-reporting from the same oracle must not count twice.
            e.storage().persistent().set(&OracleData::LastUpdate(1, 0), &e.ledger().timestamp());
            assert_eq!(validate_oracle_staleness(&e, 1, &config), Err(ErrorCode::OracleFailure));
            e.storage().persistent().set(&OracleData::LastUpdate(1, 1), &e.ledger().timestamp());
            assert_eq!(validate_oracle_staleness(&e, 1, &config), Ok(()));
        });
    }
}
