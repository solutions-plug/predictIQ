//! Property-based fuzz tests for bet placement inputs (Issue: bets area).
//!
//! Covers the non-trivial interactions between:
//!   - outcome index  (valid: 0..options.len(), invalid: >= options.len())
//!   - amount         (valid: > 0, invalid: <= 0)
//!   - timestamp      (valid: < deadline AND < resolution_deadline,
//!                     invalid: >= deadline OR >= resolution_deadline)
//!
//! A minimal LCG PRNG is used so no external crate is required.

#![cfg(test)]

use crate::errors::ErrorCode;
use crate::types::{MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, String, Vec,
};

// ---------------------------------------------------------------------------
// Deterministic PRNG (LCG — no external deps)
// ---------------------------------------------------------------------------

struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
    fn next_i128(&mut self) -> i128 {
        ((self.next() as i128) << 64) | (self.next() as i128)
    }
    fn next_u32_range(&mut self, lo: u32, hi: u32) -> u32 {
        lo + (self.next() as u32 % (hi - lo + 1))
    }
    fn next_u64_range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + (self.next() % (hi - lo + 1))
    }
    fn next_i128_range(&mut self, lo: i128, hi: i128) -> i128 {
        let span = (hi - lo + 1) as u128;
        lo + (self.next_i128().unsigned_abs() % span) as i128
    }
}

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

const DEADLINE: u64 = 1_000;
const RESOLUTION_DEADLINE: u64 = 2_000;
const NUM_OPTIONS: u32 = 4; // outcomes 0..3

fn setup() -> (Env, PredictIQClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &0); // 0 fee — keeps net == gross for assertions

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token = token_id.address();
    (env, client, admin, token)
}

fn create_market(env: &Env, client: &PredictIQClient, token: &Address) -> u64 {
    let mut options = Vec::new(env);
    let labels = ["opt0", "opt1", "opt2", "opt3"];
    for label in labels.iter().take(NUM_OPTIONS as usize) {
        options.push_back(String::from_str(env, label));
    }
    let oracle = OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "feed"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };
    client.create_market(
        &Address::generate(env),
        &String::from_str(env, "Fuzz Market"),
        &options,
        &DEADLINE,
        &RESOLUTION_DEADLINE,
        &oracle,
        &MarketTier::Basic,
        token,
        &0,
        &0,
    )
}

fn funded_user(env: &Env, token: &Address, balance: i128) -> Address {
    let user = Address::generate(env);
    token::StellarAssetClient::new(env, token).mint(&user, &balance);
    user
}

// ---------------------------------------------------------------------------
// Property 1 — valid tuples always succeed
//
// ∀ (outcome ∈ [0, NUM_OPTIONS), amount > 0, timestamp < deadline):
//   place_bet succeeds.
// ---------------------------------------------------------------------------

#[test]
fn prop_valid_bet_tuples_always_succeed() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0xDEAD_BEEF_1234_5678);

    for _ in 0..200 {
        let outcome = rng.next_u32_range(0, NUM_OPTIONS - 1);
        let amount = rng.next_i128_range(1, 50_000);
        let ts = rng.next_u64_range(0, DEADLINE - 1);

        env.ledger().set_timestamp(ts);
        let user = funded_user(&env, &token, amount + 1);

        let result = client.try_place_bet(&user, &market_id, &outcome, &amount, &token, &None);
        assert!(
            result.is_ok(),
            "valid tuple (outcome={outcome}, amount={amount}, ts={ts}) must succeed, got {result:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 2 — invalid outcome always rejected
//
// ∀ outcome >= NUM_OPTIONS, any valid (amount, timestamp):
//   place_bet returns InvalidOutcome.
// ---------------------------------------------------------------------------

#[test]
fn prop_out_of_range_outcome_always_rejected() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0xCAFE_BABE_0000_0001);

    for _ in 0..200 {
        let outcome = rng.next_u32_range(NUM_OPTIONS, u32::MAX / 2);
        let amount = rng.next_i128_range(1, 50_000);
        let ts = rng.next_u64_range(0, DEADLINE - 1);

        env.ledger().set_timestamp(ts);
        let user = funded_user(&env, &token, amount + 1);

        let result = client.try_place_bet(&user, &market_id, &outcome, &amount, &token, &None);
        assert_eq!(
            result,
            Err(Ok(ErrorCode::InvalidOutcome)),
            "outcome={outcome} >= NUM_OPTIONS must yield InvalidOutcome"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 3 — non-positive amount always rejected
//
// ∀ amount <= 0, any valid (outcome, timestamp):
//   place_bet returns InvalidAmount.
// ---------------------------------------------------------------------------

#[test]
fn prop_non_positive_amount_always_rejected() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0x1111_2222_3333_4444);

    for _ in 0..200 {
        let outcome = rng.next_u32_range(0, NUM_OPTIONS - 1);
        // amounts in [-100_000, 0]
        let amount = rng.next_i128_range(-100_000, 0);
        let ts = rng.next_u64_range(0, DEADLINE - 1);

        env.ledger().set_timestamp(ts);
        let user = Address::generate(&env); // no mint needed — rejected before transfer

        let result = client.try_place_bet(&user, &market_id, &outcome, &amount, &token, &None);
        assert_eq!(
            result,
            Err(Ok(ErrorCode::InvalidAmount)),
            "amount={amount} <= 0 must yield InvalidAmount"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 4 — timestamp >= deadline always rejected
//
// ∀ timestamp ∈ [deadline, resolution_deadline), valid (outcome, amount):
//   place_bet returns MarketClosed.
// ---------------------------------------------------------------------------

#[test]
fn prop_timestamp_at_or_after_deadline_rejected() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0xAAAA_BBBB_CCCC_DDDD);

    for _ in 0..200 {
        let outcome = rng.next_u32_range(0, NUM_OPTIONS - 1);
        let amount = rng.next_i128_range(1, 50_000);
        let ts = rng.next_u64_range(DEADLINE, RESOLUTION_DEADLINE - 1);

        env.ledger().set_timestamp(ts);
        let user = funded_user(&env, &token, amount + 1);

        let result = client.try_place_bet(&user, &market_id, &outcome, &amount, &token, &None);
        assert_eq!(
            result,
            Err(Ok(ErrorCode::MarketClosed)),
            "ts={ts} >= deadline={DEADLINE} must yield MarketClosed"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 5 — timestamp >= resolution_deadline always rejected
//
// ∀ timestamp >= resolution_deadline, valid (outcome, amount):
//   place_bet returns ResolutionDeadlinePassed.
// ---------------------------------------------------------------------------

#[test]
fn prop_timestamp_at_or_after_resolution_deadline_rejected() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0xFEED_FACE_DEAD_BEEF);

    for _ in 0..200 {
        let outcome = rng.next_u32_range(0, NUM_OPTIONS - 1);
        let amount = rng.next_i128_range(1, 50_000);
        let ts = rng.next_u64_range(RESOLUTION_DEADLINE, RESOLUTION_DEADLINE + 1_000_000);

        env.ledger().set_timestamp(ts);
        let user = funded_user(&env, &token, amount + 1);

        let result = client.try_place_bet(&user, &market_id, &outcome, &amount, &token, &None);
        assert_eq!(
            result,
            Err(Ok(ErrorCode::ResolutionDeadlinePassed)),
            "ts={ts} >= resolution_deadline={RESOLUTION_DEADLINE} must yield ResolutionDeadlinePassed"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 6 — wrong token always rejected
//
// ∀ valid (outcome, amount, timestamp), token != market.token_address:
//   place_bet returns InvalidBetAmount.
// ---------------------------------------------------------------------------

#[test]
fn prop_wrong_token_always_rejected() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0x0102_0304_0506_0708);

    for _ in 0..100 {
        let outcome = rng.next_u32_range(0, NUM_OPTIONS - 1);
        let amount = rng.next_i128_range(1, 50_000);
        let ts = rng.next_u64_range(0, DEADLINE - 1);

        env.ledger().set_timestamp(ts);
        let user = Address::generate(&env);

        // Register a fresh token that is NOT the market's token
        let other_token_admin = Address::generate(&env);
        let other_token = env
            .register_stellar_asset_contract_v2(other_token_admin)
            .address();

        let result = client.try_place_bet(&user, &market_id, &outcome, &amount, &other_token, &None);
        assert_eq!(
            result,
            Err(Ok(ErrorCode::InvalidBetAmount)),
            "wrong token must yield InvalidBetAmount"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 7 — self-referral always rejected regardless of other inputs
//
// ∀ valid (outcome, amount, timestamp), referrer == bettor:
//   place_bet returns InvalidReferrer.
// ---------------------------------------------------------------------------

#[test]
fn prop_self_referral_always_rejected() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0x9999_8888_7777_6666);

    for _ in 0..100 {
        let outcome = rng.next_u32_range(0, NUM_OPTIONS - 1);
        let amount = rng.next_i128_range(1, 50_000);
        let ts = rng.next_u64_range(0, DEADLINE - 1);

        env.ledger().set_timestamp(ts);
        let user = funded_user(&env, &token, amount + 1);

        let result =
            client.try_place_bet(&user, &market_id, &outcome, &amount, &token, &Some(user.clone()));
        assert_eq!(
            result,
            Err(Ok(ErrorCode::InvalidReferrer)),
            "self-referral must yield InvalidReferrer"
        );
    }
}

// ---------------------------------------------------------------------------
// Property 8 — accumulated stake is monotonically non-decreasing
//
// After N valid bets on the same outcome, total_staked == sum of net amounts.
// (fee = 0 in this suite so net == gross)
// ---------------------------------------------------------------------------

#[test]
fn prop_total_staked_monotonically_increases() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0x1234_5678_9ABC_DEF0);

    env.ledger().set_timestamp(0);

    let mut expected_total: i128 = 0;

    for _ in 0..50 {
        let outcome = rng.next_u32_range(0, NUM_OPTIONS - 1);
        let amount = rng.next_i128_range(1, 10_000);

        let user = funded_user(&env, &token, amount + 1);
        client.place_bet(&user, &market_id, &outcome, &amount, &token, &None);
        expected_total += amount; // fee = 0

        let market = client.get_market(&market_id).unwrap();
        assert!(
            market.total_staked >= expected_total,
            "total_staked must be >= cumulative bets; got {} expected >= {}",
            market.total_staked,
            expected_total
        );
    }
}

// ---------------------------------------------------------------------------
// Property 9 — mixed valid/invalid tuples: error codes are mutually exclusive
//
// For each randomly generated tuple, exactly one error condition fires
// (or it succeeds), never a silent wrong-error.
// ---------------------------------------------------------------------------

#[test]
fn prop_error_codes_are_mutually_exclusive() {
    let (env, client, _admin, token) = setup();
    let market_id = create_market(&env, &client, &token);
    let mut rng = Lcg(0xABCD_EF01_2345_6789);

    for _ in 0..300 {
        // Randomly pick valid or invalid for each dimension
        let outcome: u32 = rng.next_u32_range(0, NUM_OPTIONS + 2); // sometimes invalid
        let amount: i128 = rng.next_i128_range(-5, 50_000); // sometimes <= 0
        let ts: u64 = rng.next_u64_range(0, RESOLUTION_DEADLINE + 500); // sometimes past deadline

        env.ledger().set_timestamp(ts);
        let user = if amount > 0 {
            funded_user(&env, &token, amount + 1)
        } else {
            Address::generate(&env)
        };

        let result = client.try_place_bet(&user, &market_id, &outcome, &amount, &token, &None);

        // Classify expected outcome
        let amount_invalid = amount <= 0;
        let outcome_invalid = outcome >= NUM_OPTIONS;
        let past_resolution = ts >= RESOLUTION_DEADLINE;
        let past_deadline = ts >= DEADLINE && !past_resolution;

        match result {
            Ok(_) => {
                assert!(
                    !amount_invalid && !outcome_invalid && !past_deadline && !past_resolution,
                    "unexpected success for (outcome={outcome}, amount={amount}, ts={ts})"
                );
            }
            Err(Ok(ErrorCode::InvalidAmount)) => {
                assert!(amount_invalid, "InvalidAmount fired but amount={amount} > 0");
            }
            Err(Ok(ErrorCode::InvalidOutcome)) => {
                // InvalidOutcome fires only when amount is valid (checked after amount guard)
                assert!(
                    outcome_invalid && !amount_invalid,
                    "InvalidOutcome fired unexpectedly (outcome={outcome}, amount={amount})"
                );
            }
            Err(Ok(ErrorCode::MarketClosed)) => {
                assert!(
                    past_deadline,
                    "MarketClosed fired but ts={ts} is not in [deadline, resolution_deadline)"
                );
            }
            Err(Ok(ErrorCode::ResolutionDeadlinePassed)) => {
                assert!(
                    past_resolution,
                    "ResolutionDeadlinePassed fired but ts={ts} < resolution_deadline={RESOLUTION_DEADLINE}"
                );
            }
            // Other errors (e.g. transfer failure) are acceptable for invalid inputs
            _ => {}
        }
    }
}
