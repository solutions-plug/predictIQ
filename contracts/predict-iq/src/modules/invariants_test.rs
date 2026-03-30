#![cfg(test)]
use crate::types::{MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String, Vec,
};

fn setup() -> (Env, PredictIQClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &100);

    (env, client, admin)
}

/// Asserts stake conservation: sum of all outcome stakes equals total_staked
fn assert_stake_conservation(env: &Env, client: &PredictIQClient, market_id: u64) {
    let market = client.get_market(&market_id).unwrap();

    // Get stakes for all outcomes (assuming max 10 outcomes for testing)
    let mut total_outcome_stakes: i128 = 0;
    for outcome in 0..10 {
        let stake = client.get_outcome_stake(&market_id, &outcome);
        total_outcome_stakes += stake;
    }

    assert_eq!(
        total_outcome_stakes,
        market.total_staked,
        "Stake conservation violated: sum(outcome_stakes)={} != total_staked={}",
        total_outcome_stakes,
        market.total_staked
    );
}

// =============================================================================
// BASIC LIFECYCLE CONSERVATION
// =============================================================================

#[test]
fn test_stake_conservation_after_market_creation() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Conservation Test Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // After creation, conservation should hold (total_staked = 0)
    assert_stake_conservation(&env, &client, market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 0);
}

#[test]
fn test_stake_conservation_single_bet() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Single Bet Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place single bet
    client.place_bet(&bettor, &market_id, &0, &100, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    let market = client.get_market(&market_id).unwrap();
    // Net stored after 1% fee: 100 - 1 = 99
    assert_eq!(market.total_staked, 99);
    assert_eq!(client.get_outcome_stake(&market_id, &0), 99);
}

#[test]
fn test_stake_conservation_multiple_bets_same_outcome() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor1 = Address::generate(&env);
    let bettor2 = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Multiple Bets Same Outcome"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place multiple bets on same outcome
    client.place_bet(&bettor1, &market_id, &0, &100, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    client.place_bet(&bettor2, &market_id, &0, &50, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    let market = client.get_market(&market_id).unwrap();
    // Net stored after 1% fee: 100 - 1 = 99, 50 - 0 = 50 (fee truncates to 0 for small amounts)
    assert_eq!(market.total_staked, 149);
    assert_eq!(client.get_outcome_stake(&market_id, &0), 149);
}

#[test]
fn test_stake_conservation_multiple_bets_different_outcomes() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Multiple Bets Different Outcomes"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place bets on different outcomes
    client.place_bet(&bettor, &market_id, &0, &100, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    client.place_bet(&bettor, &market_id, &1, &75, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    let market = client.get_market(&market_id).unwrap();
    // Net stored after 1% fee: 100 - 1 = 99, 75 - 0 = 75
    assert_eq!(market.total_staked, 174);
    assert_eq!(client.get_outcome_stake(&market_id, &0), 99);
    assert_eq!(client.get_outcome_stake(&market_id, &1), 75);
}

// =============================================================================
// CANCELLATION AND REFUNDS
// =============================================================================

#[test]
fn test_stake_conservation_cancellation_refunds() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Cancellation Refunds Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place bets
    client.place_bet(&bettor, &market_id, &0, &100, &token, &None);
    client.place_bet(&bettor, &market_id, &1, &50, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Cancel market
    client.cancel_market_admin(&market_id);
    assert_stake_conservation(&env, &client, market_id);

    // Withdraw refunds
    client.withdraw_refund(&bettor, &market_id, &0, &token);
    assert_stake_conservation(&env, &client, market_id);

    client.withdraw_refund(&bettor, &market_id, &1, &token);
    assert_stake_conservation(&env, &client, market_id);

    // Final state should have zero stakes
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 0);
    assert_eq!(client.get_outcome_stake(&market_id, &0), 0);
    assert_eq!(client.get_outcome_stake(&market_id, &1), 0);
}

#[test]
fn test_stake_conservation_partial_refunds() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor1 = Address::generate(&env);
    let bettor2 = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Partial Refunds Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place bets from different bettors
    client.place_bet(&bettor1, &market_id, &0, &100, &token, &None);
    client.place_bet(&bettor2, &market_id, &1, &50, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Cancel market
    client.cancel_market_admin(&market_id);
    assert_stake_conservation(&env, &client, market_id);

    // Only bettor1 withdraws refund
    client.withdraw_refund(&bettor1, &market_id, &0, &token);
    assert_stake_conservation(&env, &client, market_id);

    // Verify partial state
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 50); // bettor2's net stake still there (50 - 0 fee)
    assert_eq!(client.get_outcome_stake(&market_id, &0), 0);
    assert_eq!(client.get_outcome_stake(&market_id, &1), 50);
}

#[test]
fn test_stake_conservation_multiple_cancellations() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Multiple Cancellations Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place bets, cancel, place more bets, cancel again
    client.place_bet(&bettor, &market_id, &0, &100, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    client.cancel_market_admin(&market_id);
    assert_stake_conservation(&env, &client, market_id);

    // After cancellation, place new bets (assuming market allows this)
    client.place_bet(&bettor, &market_id, &1, &75, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    client.cancel_market_admin(&market_id);
    assert_stake_conservation(&env, &client, market_id);

    // Withdraw all refunds
    client.withdraw_refund(&bettor, &market_id, &0, &token);
    assert_stake_conservation(&env, &client, market_id);

    client.withdraw_refund(&bettor, &market_id, &1, &token);
    assert_stake_conservation(&env, &client, market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 0);
}

// =============================================================================
// RESOLUTION AND PAYOUTS
// =============================================================================

#[test]
fn test_stake_conservation_resolution_winner_payouts() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Resolution Winner Payouts"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place bets on both outcomes
    client.place_bet(&bettor, &market_id, &0, &100, &token, &None);
    client.place_bet(&bettor, &market_id, &1, &50, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Resolve market with outcome 0 as winner
    client.resolve_market(&market_id, &0);
    assert_stake_conservation(&env, &client, market_id);

    // Claim payout
    client.claim_payout(&bettor, &market_id, &token);
    assert_stake_conservation(&env, &client, market_id);

    // After payout, stakes should be zero
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 0);
}

#[test]
fn test_stake_conservation_resolution_multiple_winners() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor1 = Address::generate(&env);
    let bettor2 = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Resolution Multiple Winners"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Multiple bettors bet on winning outcome
    client.place_bet(&bettor1, &market_id, &0, &100, &token, &None);
    client.place_bet(&bettor2, &market_id, &0, &50, &token, &None);
    client.place_bet(&bettor1, &market_id, &1, &25, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Resolve with outcome 0 as winner
    client.resolve_market(&market_id, &0);
    assert_stake_conservation(&env, &client, market_id);

    // Claim payouts
    client.claim_payout(&bettor1, &market_id, &token);
    assert_stake_conservation(&env, &client, market_id);

    client.claim_payout(&bettor2, &market_id, &token);
    assert_stake_conservation(&env, &client, market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 0);
}

// =============================================================================
// MULTI-OUTCOME SCENARIOS
// =============================================================================

#[test]
fn test_stake_conservation_three_outcome_market() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Option A"),
            String::from_str(&env, "Option B"),
            String::from_str(&env, "Option C"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Three Outcome Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place bets on all outcomes
    client.place_bet(&bettor, &market_id, &0, &100, &token, &None);
    client.place_bet(&bettor, &market_id, &1, &75, &token, &None);
    client.place_bet(&bettor, &market_id, &2, &50, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Cancel and refund
    client.cancel_market_admin(&market_id);
    assert_stake_conservation(&env, &client, market_id);

    client.withdraw_refund(&bettor, &market_id, &0, &token);
    client.withdraw_refund(&bettor, &market_id, &1, &token);
    client.withdraw_refund(&bettor, &market_id, &2, &token);
    assert_stake_conservation(&env, &client, market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 0);
}

#[test]
fn test_stake_conservation_boundary_values() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Boundary Values Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Test with very small stakes
    client.place_bet(&bettor, &market_id, &0, &1, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Test with maximum reasonable stakes
    client.place_bet(&bettor, &market_id, &1, &i128::MAX / 2, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Cancel and verify conservation during refunds
    client.cancel_market_admin(&market_id);
    assert_stake_conservation(&env, &client, market_id);

    client.withdraw_refund(&bettor, &market_id, &0, &token);
    assert_stake_conservation(&env, &client, market_id);

    client.withdraw_refund(&bettor, &market_id, &1, &token);
    assert_stake_conservation(&env, &client, market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 0);
}

// =============================================================================
// EDGE CASES AND CONSERVATION
// =============================================================================

#[test]
fn test_stake_conservation_valid_bet_maintains_conservation() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Valid Bet Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place valid bet
    client.place_bet(&bettor, &market_id, &0, &100, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Verify conservation holds
    let market = client.get_market(&market_id).unwrap();
    // Net stored after 1% fee: 100 - 1 = 99
    assert_eq!(market.total_staked, 99);
}

#[test]
fn test_stake_conservation_multiple_outcome_refunds() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    let token = Address::generate(&env);
    let bettor = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Multiple Refund Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Place bets on both outcomes
    client.place_bet(&bettor, &market_id, &0, &100, &token, &None);
    client.place_bet(&bettor, &market_id, &1, &50, &token, &None);
    assert_stake_conservation(&env, &client, market_id);

    // Cancel market
    client.cancel_market_admin(&market_id);
    assert_stake_conservation(&env, &client, market_id);

    // Withdraw from first outcome
    client.withdraw_refund(&bettor, &market_id, &0, &token);
    assert_stake_conservation(&env, &client, market_id);

    // Withdraw from second outcome
    client.withdraw_refund(&bettor, &market_id, &1, &token);
    assert_stake_conservation(&env, &client, market_id);

    // Verify final state
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 0);
}