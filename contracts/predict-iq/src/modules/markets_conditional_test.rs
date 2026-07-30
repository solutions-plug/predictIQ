#![cfg(test)]
/// Issue #069: Tests for conditional market parent validation.
/// Issue #070: Tests for market tier access control.
use crate::errors::ErrorCode;
use crate::modules::markets;
use crate::types::{CreatorReputation, MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env, String, Vec};

fn setup() -> (Env, PredictIQClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &0);

    (env, client, admin, contract_id)
}

fn oracle_config(env: &Env) -> OracleConfig {
    OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
        strike_price: None,
    }
}

fn two_options(env: &Env) -> Vec<String> {
    Vec::from_array(
        env,
        [String::from_str(env, "Yes"), String::from_str(env, "No")],
    )
}

/// Create a market, advance time past deadline, set oracle result, and resolve it.
/// Returns the resolved market_id.
fn create_resolved_market(
    env: &Env,
    client: &PredictIQClient,
    contract_id: &Address,
    admin: &Address,
    deadline: u64,
    resolution_deadline: u64,
    winning_outcome: u32,
) -> u64 {
    let token = Address::generate(env);
    let market_id = client.create_market(
        admin,
        &String::from_str(env, "Parent"),
        &two_options(env),
        &deadline,
        &resolution_deadline,
        &oracle_config(env),
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Move market directly to Resolved state via internal storage
    env.as_contract(contract_id, || {
        let mut market = markets::get_market(env, market_id).unwrap();
        market.status = MarketStatus::Resolved;
        market.winning_outcome = Some(winning_outcome);
        market.resolved_at = Some(resolution_deadline + 1);
        markets::update_market(env, market);
    });

    market_id
}

// ── Issue #069: Conditional market parent validation ──────────────────────────

/// Happy path: conditional market created when parent is resolved with correct outcome.
#[test]
fn test_conditional_market_valid_parent() {
    let (env, client, admin, cid) = setup();

    let parent_id = create_resolved_market(&env, &client, &cid, &admin, 1000, 2000, 0);

    let token = Address::generate(&env);
    // Conditional market deadline must be <= parent resolution_deadline (2000)
    let child_id = client.create_market(
        &admin,
        &String::from_str(&env, "Child"),
        &two_options(&env),
        &1500,
        &1800,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &parent_id,
        &0, // parent resolved to outcome 0
    );

    let child = client.get_market(&child_id).unwrap();
    assert_eq!(child.parent_id, parent_id);
    assert_eq!(child.parent_outcome_idx, 0);
}

/// Conditional market fails when parent is still Active.
#[test]
fn test_conditional_market_parent_not_resolved() {
    let (env, client, admin, _cid) = setup();

    let token = Address::generate(&env);
    let parent_id = client.create_market(
        &admin,
        &String::from_str(&env, "Parent"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Child"),
        &two_options(&env),
        &500,
        &800,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &parent_id,
        &0,
    );
    assert_eq!(result, Err(Ok(ErrorCode::ParentMarketNotResolved)));
}

/// Conditional market fails when parent resolved to a different outcome.
#[test]
fn test_conditional_market_wrong_parent_outcome() {
    let (env, client, admin, cid) = setup();

    // Parent resolves to outcome 1
    let parent_id = create_resolved_market(&env, &client, &cid, &admin, 1000, 2000, 1);

    let token = Address::generate(&env);
    // Child requires parent outcome 0 — mismatch
    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Child"),
        &two_options(&env),
        &1500,
        &1800,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &parent_id,
        &0,
    );
    assert_eq!(result, Err(Ok(ErrorCode::ParentMarketInvalidOutcome)));
}

/// Conditional market fails when parent_outcome_idx is out of range.
#[test]
fn test_conditional_market_invalid_outcome_index() {
    let (env, client, admin, cid) = setup();

    let parent_id = create_resolved_market(&env, &client, &cid, &admin, 1000, 2000, 0);

    let token = Address::generate(&env);
    // Parent only has 2 options (0, 1); index 5 is invalid
    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Child"),
        &two_options(&env),
        &1500,
        &1800,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &parent_id,
        &5,
    );
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}

/// Conditional market fails when its deadline exceeds parent's resolution_deadline.
#[test]
fn test_conditional_market_deadline_exceeds_parent() {
    let (env, client, admin, cid) = setup();

    // Parent resolution_deadline = 2000
    let parent_id = create_resolved_market(&env, &client, &cid, &admin, 1000, 2000, 0);

    let token = Address::generate(&env);
    // Child deadline 3000 > parent resolution_deadline 2000
    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Child"),
        &two_options(&env),
        &3000,
        &4000,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &parent_id,
        &0,
    );
    assert_eq!(result, Err(Ok(ErrorCode::DeadlinePassed)));
}

/// Non-conditional market (parent_id = 0) is unaffected by parent validation.
#[test]
fn test_independent_market_no_parent_validation() {
    let (env, client, admin, _cid) = setup();

    let token = Address::generate(&env);
    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Independent"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &0, // no parent
        &0,
    );

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, MarketStatus::Active);
    assert_eq!(market.parent_id, 0);
}

// ── Issue #070: Market tier access control ────────────────────────────────────

/// Creator with None reputation can only create Basic markets.
#[test]
fn test_tier_none_reputation_basic_only() {
    let (env, client, _admin, _cid) = setup();

    let creator = Address::generate(&env);
    // reputation defaults to None
    let token = Address::generate(&env);

    // Basic — allowed
    client.create_market(
        &creator,
        &String::from_str(&env, "Basic"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Pro — rejected
    let result = client.try_create_market(
        &creator,
        &String::from_str(&env, "Pro"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Pro,
        &token,
        &0,
        &0,
    );
    assert_eq!(result, Err(Ok(ErrorCode::InsufficientReputation)));

    // Institutional — rejected
    let result = client.try_create_market(
        &creator,
        &String::from_str(&env, "Inst"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Institutional,
        &token,
        &0,
        &0,
    );
    assert_eq!(result, Err(Ok(ErrorCode::InsufficientReputation)));
}

/// Creator with Pro reputation can create Basic and Pro markets but not Institutional.
#[test]
fn test_tier_pro_reputation() {
    let (env, client, _admin, _cid) = setup();

    let creator = Address::generate(&env);
    client.set_creator_reputation(&creator, &CreatorReputation::Pro);
    let token = Address::generate(&env);

    // Basic — allowed
    client.create_market(
        &creator,
        &String::from_str(&env, "Basic"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Pro — allowed
    client.create_market(
        &creator,
        &String::from_str(&env, "Pro"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Pro,
        &token,
        &0,
        &0,
    );

    // Institutional — rejected
    let result = client.try_create_market(
        &creator,
        &String::from_str(&env, "Inst"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Institutional,
        &token,
        &0,
        &0,
    );
    assert_eq!(result, Err(Ok(ErrorCode::InsufficientReputation)));
}

/// Creator with Institutional reputation can create all tiers.
#[test]
fn test_tier_institutional_reputation_all_tiers() {
    let (env, client, _admin, _cid) = setup();

    let creator = Address::generate(&env);
    client.set_creator_reputation(&creator, &CreatorReputation::Institutional);
    let token = Address::generate(&env);

    for tier in [
        MarketTier::Basic,
        MarketTier::Pro,
        MarketTier::Institutional,
    ] {
        client.create_market(
            &creator,
            &String::from_str(&env, "Market"),
            &two_options(&env),
            &1000,
            &2000,
            &oracle_config(&env),
            &tier,
            &token,
            &0,
            &0,
        );
    }
}

/// Admin can upgrade a creator's reputation, enabling higher-tier market creation.
#[test]
fn test_tier_upgrade_path() {
    let (env, client, _admin, _cid) = setup();

    let creator = Address::generate(&env);
    let token = Address::generate(&env);

    // Initially None — Pro rejected
    let result = client.try_create_market(
        &creator,
        &String::from_str(&env, "Pro"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Pro,
        &token,
        &0,
        &0,
    );
    assert_eq!(result, Err(Ok(ErrorCode::InsufficientReputation)));

    // Admin upgrades to Pro
    client.set_creator_reputation(&creator, &CreatorReputation::Pro);

    // Now Pro is allowed
    client.create_market(
        &creator,
        &String::from_str(&env, "Pro"),
        &two_options(&env),
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Pro,
        &token,
        &0,
        &0,
    );
}

// ── place_bet accessor functions (markets::get_outcome_stake / set_outcome_stake /
// increment_outcome_bet_count / validate_parent_market) ────────────────────────

/// place_bet must correctly accumulate per-outcome stakes and unique-bettor counts
/// via the real markets accessor functions, including on a conditional (child) market.
#[test]
fn test_place_bet_tracks_outcome_stakes_and_winner_counts() {
    let (env, client, admin, cid) = setup();

    let parent_id = create_resolved_market(&env, &client, &cid, &admin, 1000, 2000, 0);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_id.address();

    let child_id = client.create_market(
        &admin,
        &String::from_str(&env, "Child"),
        &two_options(&env),
        &1500,
        &1800,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &parent_id,
        &0, // parent resolved to outcome 0
    );

    let alice = Address::generate(&env);
    let bob = Address::generate(&env);
    let carol = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&alice, &10_000);
    token_client.mint(&bob, &10_000);
    token_client.mint(&carol, &10_000);

    client.place_bet(&alice, &child_id, &0, &4_000, &token, &None);
    client.place_bet(&bob, &child_id, &0, &1_500, &token, &None); // same outcome, different bettor
    client.place_bet(&carol, &child_id, &1, &2_500, &token, &None);

    let market = client.get_market(&child_id).unwrap();
    assert_eq!(
        market.outcome_stakes.get(0).unwrap_or(0),
        5_500,
        "outcome 0 stake should be the sum of alice's and bob's bets"
    );
    assert_eq!(
        market.outcome_stakes.get(1).unwrap_or(0),
        2_500,
        "outcome 1 stake should equal carol's bet"
    );
    assert_eq!(
        market.winner_counts.get(0).unwrap_or(0),
        2,
        "two unique bettors placed bets on outcome 0"
    );
    assert_eq!(
        market.winner_counts.get(1).unwrap_or(0),
        1,
        "one unique bettor placed a bet on outcome 1"
    );
}

/// place_bet must re-validate the parent market at bet time via
/// markets::validate_parent_market, not just trust state captured when the
/// conditional market was created.
#[test]
fn test_place_bet_on_conditional_market_rejected_when_parent_reverts() {
    let (env, client, admin, cid) = setup();

    let parent_id = create_resolved_market(&env, &client, &cid, &admin, 1000, 2000, 0);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin);
    let token = token_id.address();

    let child_id = client.create_market(
        &admin,
        &String::from_str(&env, "Child"),
        &two_options(&env),
        &1500,
        &1800,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &parent_id,
        &0,
    );

    // Simulate the parent's resolution being reverted (e.g. by a dispute) after
    // the child market was already created.
    env.as_contract(&cid, || {
        let mut parent = markets::get_market(&env, parent_id).unwrap();
        parent.status = MarketStatus::Active;
        markets::update_market(&env, parent);
    });

    let alice = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&alice, &10_000);

    let result = client.try_place_bet(&alice, &child_id, &0, &1_000, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::ParentMarketNotResolved)));
}
