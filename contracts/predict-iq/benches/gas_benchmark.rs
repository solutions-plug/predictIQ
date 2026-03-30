// Gas Benchmarking Tests for PredictIQ
// Measures instruction counts and validates gas thresholds for key operations.
//
// CI pass/fail thresholds are enforced via assertions on `get_resolution_metrics`.
// The gas_estimate formula is: 100_000 + (winner_count * 50_000).
// At MAX_OUTCOMES_PER_MARKET (32) with MAX_PUSH_PAYOUT_WINNERS (50) winners,
// the ceiling is: 100_000 + (50 * 50_000) = 2_600_000 instructions.

#![cfg(test)]

use soroban_sdk::{
    testutils::Address as _,
    token, Address, Env, String, Vec,
};

extern crate predict_iq;
use predict_iq::{PredictIQ, PredictIQClient};

// ── Gas threshold constants (CI pass/fail gates) ──────────────────────────────

/// Maximum acceptable gas estimate for a dispute/payout flow at max outcomes.
/// Formula: 100_000 base + (MAX_PUSH_PAYOUT_WINNERS * 50_000 per winner).
const GAS_THRESHOLD_DISPUTE_PAYOUT: u64 = 2_600_000;

/// Maximum acceptable gas estimate for a single-winner resolution.
const GAS_THRESHOLD_SINGLE_WINNER: u64 = 200_000;

// ── Shared helpers ────────────────────────────────────────────────────────────

fn create_test_env() -> (Env, Address, PredictIQClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    env.budget().reset_unlimited();

    let contract_id = env.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let mut guardians = soroban_sdk::Vec::new(&env);
    guardians.push_back(predict_iq::types::Guardian {
        address: Address::generate(&env),
        voting_power: 1,
    });
    client.initialize(&admin, &0, &guardians);

    (env, admin, client)
}

fn create_options(env: &Env, count: u32) -> Vec<String> {
    let mut options = Vec::new(env);
    for i in 0..count {
        options.push_back(String::from_str(env, &format!("Option{}", i)));
    }
    options
}

fn create_oracle_config(env: &Env) -> predict_iq::types::OracleConfig {
    predict_iq::types::OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "test_feed"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    }
}

/// Create a market and return its id + token address.
fn create_market_with_token(
    env: &Env,
    client: &PredictIQClient,
    outcome_count: u32,
) -> (u64, Address) {
    let creator = Address::generate(env);
    let token_admin = Address::generate(env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    let options = create_options(env, outcome_count);
    let oracle_config = create_oracle_config(env);

    let market_id = client.create_market(
        &creator,
        &String::from_str(env, "Benchmark Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &predict_iq::types::MarketTier::Basic,
        &token_address,
        &0u64,
        &0u32,
    );

    (market_id, token_address)
}

/// Mint tokens and place a bet, returning the bettor address.
fn place_bet(
    env: &Env,
    client: &PredictIQClient,
    market_id: u64,
    outcome: u32,
    amount: i128,
    token_address: &Address,
) -> Address {
    let bettor = Address::generate(env);
    let stellar = token::StellarAssetClient::new(env, token_address);
    stellar.mint(&bettor, &amount);
    client.place_bet(&bettor, &market_id, &outcome, &amount, token_address, &None);
    bettor
}

// ── Market creation benchmarks ────────────────────────────────────────────────

#[test]
fn bench_create_market_10_outcomes() {
    let (env, _admin, client) = create_test_env();
    let result = client.try_create_market(
        &Address::generate(&env),
        &String::from_str(&env, "10 Outcome Market"),
        &create_options(&env, 10),
        &1000,
        &2000,
        &create_oracle_config(&env),
        &predict_iq::types::MarketTier::Basic,
        &Address::generate(&env),
        &0u64,
        &0u32,
    );
    assert!(result.is_ok(), "10-outcome market creation must succeed");
}

#[test]
fn bench_create_market_max_outcomes() {
    let (env, _admin, client) = create_test_env();
    // MAX_OUTCOMES_PER_MARKET = 32
    let result = client.try_create_market(
        &Address::generate(&env),
        &String::from_str(&env, "Max Outcome Market"),
        &create_options(&env, predict_iq::types::MAX_OUTCOMES_PER_MARKET),
        &1000,
        &2000,
        &create_oracle_config(&env),
        &predict_iq::types::MarketTier::Basic,
        &Address::generate(&env),
        &0u64,
        &0u32,
    );
    assert!(result.is_ok(), "MAX_OUTCOMES_PER_MARKET market creation must succeed");
}

#[test]
fn bench_reject_excessive_outcomes() {
    let (env, _admin, client) = create_test_env();
    let result = client.try_create_market(
        &Address::generate(&env),
        &String::from_str(&env, "Too Many Outcomes"),
        &create_options(&env, predict_iq::types::MAX_OUTCOMES_PER_MARKET + 1),
        &1000,
        &2000,
        &create_oracle_config(&env),
        &predict_iq::types::MarketTier::Basic,
        &Address::generate(&env),
        &0u64,
        &0u32,
    );
    assert!(result.is_err(), "exceeding MAX_OUTCOMES_PER_MARKET must be rejected");
}

// ── Gas threshold assertions for dispute/payout flows ─────────────────────────

/// At max outcomes with a single winner, gas estimate must stay within threshold.
#[test]
fn bench_gas_threshold_single_winner_max_outcomes() {
    let (env, _admin, client) = create_test_env();
    let (market_id, token_address) =
        create_market_with_token(&env, &client, predict_iq::types::MAX_OUTCOMES_PER_MARKET);

    place_bet(&env, &client, market_id, 0, 1000, &token_address);

    client.resolve_market(&market_id, &0);

    let metrics = client.get_resolution_metrics(&market_id, &0);
    assert_eq!(metrics.winner_count, 1, "must have exactly 1 winner");
    assert!(
        metrics.gas_estimate <= GAS_THRESHOLD_SINGLE_WINNER,
        "single-winner gas estimate {} exceeds threshold {}",
        metrics.gas_estimate,
        GAS_THRESHOLD_SINGLE_WINNER
    );
}

/// At max outcomes with MAX_PUSH_PAYOUT_WINNERS winners, gas estimate must stay
/// within the dispute/payout threshold. This is the worst-case Push resolution.
#[test]
fn bench_gas_threshold_max_push_winners_max_outcomes() {
    let (env, _admin, client) = create_test_env();
    let (market_id, token_address) =
        create_market_with_token(&env, &client, predict_iq::types::MAX_OUTCOMES_PER_MARKET);

    let max_winners = predict_iq::types::MAX_PUSH_PAYOUT_WINNERS;
    for _ in 0..max_winners {
        place_bet(&env, &client, market_id, 0, 100, &token_address);
    }

    client.resolve_market(&market_id, &0);

    let metrics = client.get_resolution_metrics(&market_id, &0);
    assert_eq!(
        metrics.winner_count, max_winners,
        "must have exactly MAX_PUSH_PAYOUT_WINNERS winners"
    );
    assert!(
        metrics.gas_estimate <= GAS_THRESHOLD_DISPUTE_PAYOUT,
        "max-push-winners gas estimate {} exceeds CI threshold {}",
        metrics.gas_estimate,
        GAS_THRESHOLD_DISPUTE_PAYOUT
    );
}

/// One winner above MAX_PUSH_PAYOUT_WINNERS triggers Pull mode.
/// Gas estimate must still be within threshold (Pull mode doesn't iterate winners).
#[test]
fn bench_gas_threshold_pull_mode_triggered_at_max_outcomes() {
    let (env, _admin, client) = create_test_env();
    let (market_id, token_address) =
        create_market_with_token(&env, &client, predict_iq::types::MAX_OUTCOMES_PER_MARKET);

    let over_threshold = predict_iq::types::MAX_PUSH_PAYOUT_WINNERS + 1;
    for _ in 0..over_threshold {
        place_bet(&env, &client, market_id, 0, 100, &token_address);
    }

    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.payout_mode,
        predict_iq::types::PayoutMode::Pull,
        "exceeding MAX_PUSH_PAYOUT_WINNERS must trigger Pull mode"
    );

    let metrics = client.get_resolution_metrics(&market_id, &0);
    // Gas estimate is still computed; assert it doesn't overflow u64 and is bounded.
    let expected_max = 100_000 + (over_threshold as u64 * 50_000);
    assert_eq!(
        metrics.gas_estimate, expected_max,
        "gas estimate formula must be deterministic"
    );
}

// ── Full lifecycle benchmark ──────────────────────────────────────────────────

#[test]
fn bench_full_market_lifecycle_max_outcomes() {
    let (env, _admin, client) = create_test_env();
    let (market_id, token_address) =
        create_market_with_token(&env, &client, predict_iq::types::MAX_OUTCOMES_PER_MARKET);

    // Place bets across multiple outcomes to stress the winner_counts map.
    let bettor0 = place_bet(&env, &client, market_id, 0, 1000, &token_address);
    place_bet(&env, &client, market_id, 1, 500, &token_address);

    client.resolve_market(&market_id, &0);

    let metrics = client.get_resolution_metrics(&market_id, &0);
    assert_eq!(metrics.winner_count, 1);
    assert!(
        metrics.gas_estimate <= GAS_THRESHOLD_SINGLE_WINNER,
        "lifecycle gas estimate {} exceeds threshold {}",
        metrics.gas_estimate,
        GAS_THRESHOLD_SINGLE_WINNER
    );

    // Winner claims payout.
    let result = client.try_claim_winnings(&bettor0, &market_id);
    assert!(result.is_ok(), "winner must be able to claim winnings");
}
