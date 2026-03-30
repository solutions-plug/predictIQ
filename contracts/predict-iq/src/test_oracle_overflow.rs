//! Overflow-safety and boundary tests for oracle absolute-price conversion.
//!
//! Issue: Manual abs conversion on i64 can overflow/panic on i64::MIN because
//!        i64::MIN.abs() is undefined behaviour in two's-complement arithmetic
//!        (there is no positive counterpart within i64).
//!
//! Fix already applied (Issue #41): `abs_price_to_u64` saturates i64::MIN to
//! i64::MAX as u64 instead of calling `.abs()`.
//!
//! These tests prove:
//!   1. `abs_price_to_u64` never panics for any i64 value.
//!   2. The saturation sentinel (i64::MIN → i64::MAX as u64) is correct.
//!   3. All other boundary values (i64::MAX, -1, 0, 1, i64::MIN+1) round-trip correctly.
//!   4. `validate_price` never panics for extreme price values.
//!   5. `validate_price` never panics for extreme confidence values.
//!   6. `cast_external_timestamp` correctly rejects negative timestamps.
//!   7. `cast_external_timestamp` accepts the full valid u64 range expressible as i64.
//!   8. `is_stale` never panics on saturating subtraction with extreme timestamps.
//!   9. The confidence check in `validate_price` cannot overflow for any
//!      combination of extreme price and max_confidence_bps values.
//!  10. Boundary sweep: every power-of-two boundary in i64 is handled without panic.

#![cfg(test)]

use crate::modules::oracles::{abs_price_to_u64, cast_external_timestamp, is_stale, validate_price, PythPrice};
use crate::types::OracleConfig;
use soroban_sdk::{testutils::Address as _, Address, Env, String};

// ── helpers ──────────────────────────────────────────────────────────────────

/// Minimal OracleConfig that passes validation for a given confidence bps.
fn oracle_config(env: &Env, max_confidence_bps: u64) -> OracleConfig {
    OracleConfig {
        oracle_address: Address::generate(env),
        // feed_id length doesn't matter for validate_price; use a short placeholder.
        feed_id: String::from_str(env, "test"),
        min_responses: 1,
        max_staleness_seconds: u64::MAX, // never stale in these tests
        max_confidence_bps,
    }
}

/// Build a PythPrice with the given price, conf=0 (always passes confidence check),
/// and publish_time=0 (always fresh when max_staleness_seconds=u64::MAX).
fn price_with(price: i64) -> PythPrice {
    PythPrice { price, conf: 0, expo: 0, publish_time: 0 }
}

// ── abs_price_to_u64 unit tests ───────────────────────────────────────────────

/// 1. i64::MIN must not panic and must return the saturation sentinel.
#[test]
fn test_abs_i64_min_no_panic_returns_sentinel() {
    let result = abs_price_to_u64(i64::MIN);
    assert_eq!(result, i64::MAX as u64,
        "i64::MIN must saturate to i64::MAX as u64, not panic");
}

/// 2. i64::MAX round-trips correctly.
#[test]
fn test_abs_i64_max() {
    assert_eq!(abs_price_to_u64(i64::MAX), i64::MAX as u64);
}

/// 3. i64::MIN + 1 is the most-negative value with a valid positive counterpart.
#[test]
fn test_abs_i64_min_plus_one() {
    let val = i64::MIN + 1; // -9_223_372_036_854_775_807
    assert_eq!(abs_price_to_u64(val), 9_223_372_036_854_775_807u64);
}

/// 4. Zero stays zero.
#[test]
fn test_abs_zero() {
    assert_eq!(abs_price_to_u64(0), 0);
}

/// 5. Positive and negative small values are symmetric.
#[test]
fn test_abs_small_values_symmetric() {
    for v in [1i64, 42, 1_000, 1_000_000, 1_000_000_000] {
        assert_eq!(abs_price_to_u64(v), abs_price_to_u64(-v),
            "abs({v}) and abs(-{v}) must be equal");
    }
}

/// 6. Boundary sweep: every power-of-two boundary in i64 — no panics.
#[test]
fn test_abs_power_of_two_boundaries_no_panic() {
    let boundaries: &[i64] = &[
        i64::MIN,
        i64::MIN + 1,
        -(1 << 62),
        -(1 << 32),
        -(1 << 16),
        -(1 << 8),
        -1,
        0,
        1,
        1 << 8,
        1 << 16,
        1 << 32,
        1 << 62,
        i64::MAX,
    ];
    for &b in boundaries {
        // Must not panic — result value is checked separately above.
        let _ = abs_price_to_u64(b);
    }
}

// ── validate_price boundary tests ────────────────────────────────────────────

/// 7. validate_price with price=i64::MIN must not panic.
#[test]
fn test_validate_price_i64_min_no_panic() {
    let env = Env::default();
    let config = oracle_config(&env, 10_000); // 100% confidence allowed
    let p = price_with(i64::MIN);
    // Must not panic — result may be Ok or Err but never a panic.
    let _ = validate_price(&env, &p, &config);
}

/// 8. validate_price with price=i64::MAX must not panic.
#[test]
fn test_validate_price_i64_max_no_panic() {
    let env = Env::default();
    let config = oracle_config(&env, 10_000);
    let p = price_with(i64::MAX);
    let _ = validate_price(&env, &p, &config);
}

/// 9. validate_price with conf=u64::MAX and price=i64::MIN must not panic
///    (confidence check path: price_abs * max_confidence_bps / BPS_DENOMINATOR).
#[test]
fn test_validate_price_max_conf_i64_min_no_panic() {
    let env = Env::default();
    let config = oracle_config(&env, 10_000);
    let p = PythPrice { price: i64::MIN, conf: u64::MAX, expo: 0, publish_time: 0 };
    let _ = validate_price(&env, &p, &config);
}

/// 10. validate_price with conf=u64::MAX and price=i64::MAX must not panic.
#[test]
fn test_validate_price_max_conf_i64_max_no_panic() {
    let env = Env::default();
    let config = oracle_config(&env, 10_000);
    let p = PythPrice { price: i64::MAX, conf: u64::MAX, expo: 0, publish_time: 0 };
    let _ = validate_price(&env, &p, &config);
}

/// 11. validate_price with max_confidence_bps=u64::MAX must not panic
///     (overflow guard: price_abs * u64::MAX could overflow u64).
#[test]
fn test_validate_price_max_confidence_bps_no_panic() {
    let env = Env::default();
    let config = oracle_config(&env, u64::MAX);
    let p = PythPrice { price: i64::MAX, conf: 0, expo: 0, publish_time: 0 };
    let _ = validate_price(&env, &p, &config);
}

/// 12. Confidence check: when conf=0 and price=i64::MIN, validation passes
///     (zero confidence is always within any threshold).
#[test]
fn test_validate_price_zero_conf_always_passes_confidence_check() {
    let env = Env::default();
    let config = oracle_config(&env, 0); // 0 bps threshold — only conf=0 passes
    let p = price_with(i64::MIN);
    // staleness is u64::MAX so only confidence check matters
    let result = validate_price(&env, &p, &config);
    assert!(result.is_ok(),
        "conf=0 must always pass the confidence check regardless of price");
}

/// 13. Confidence check: when conf=0 and price=i64::MAX, validation passes.
#[test]
fn test_validate_price_zero_conf_i64_max_passes() {
    let env = Env::default();
    let config = oracle_config(&env, 0);
    let p = price_with(i64::MAX);
    assert!(validate_price(&env, &p, &config).is_ok());
}

// ── cast_external_timestamp boundary tests ───────────────────────────────────

/// 14. Negative timestamps must be rejected with InvalidTimestamp.
#[test]
fn test_cast_timestamp_negative_rejected() {
    use crate::errors::ErrorCode;
    assert_eq!(
        cast_external_timestamp(-1),
        Err(ErrorCode::InvalidTimestamp)
    );
    assert_eq!(
        cast_external_timestamp(i64::MIN),
        Err(ErrorCode::InvalidTimestamp)
    );
}

/// 15. Zero and positive timestamps are accepted.
#[test]
fn test_cast_timestamp_valid_range_accepted() {
    assert_eq!(cast_external_timestamp(0), Ok(0u64));
    assert_eq!(cast_external_timestamp(1), Ok(1u64));
    assert_eq!(cast_external_timestamp(i64::MAX), Ok(i64::MAX as u64));
}

// ── is_stale boundary tests ───────────────────────────────────────────────────

/// 16. is_stale must not panic when current_time < result_time (saturating_sub → 0).
#[test]
fn test_is_stale_no_panic_when_result_time_in_future() {
    // current_time=0, result_time=u64::MAX → saturating_sub gives 0, never stale
    assert!(!is_stale(0, u64::MAX, 100));
}

/// 17. is_stale must not panic with all-max inputs.
#[test]
fn test_is_stale_no_panic_all_max() {
    // u64::MAX - u64::MAX = 0, which is not > u64::MAX
    assert!(!is_stale(u64::MAX, u64::MAX, u64::MAX));
}

/// 18. is_stale correctly identifies a stale price.
#[test]
fn test_is_stale_detects_stale_price() {
    // current=1000, result=0, max_age=999 → age=1000 > 999 → stale
    assert!(is_stale(1000, 0, 999));
}

/// 19. is_stale correctly identifies a fresh price at the exact boundary.
#[test]
fn test_is_stale_boundary_exact_age_not_stale() {
    // current=1000, result=0, max_age=1000 → age=1000, not > 1000 → fresh
    assert!(!is_stale(1000, 0, 1000));
}
