//! Issue #246: Tie-handling tests for dispute voting resolution.
//!
//! Covers the case where two or more outcomes share the maximum vote tally,
//! meaning no single outcome reaches the 60% supermajority threshold.
//! `calculate_voting_outcome` (via `finalize_resolution`) must return
//! `NoMajorityReached` in all tie scenarios.

#![cfg(test)]

use crate::errors::ErrorCode;
use crate::modules::markets::DataKey as MarketDataKey;
use crate::modules::voting::DataKey as VotingDataKey;
use crate::types::{Market, MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Vec,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn setup(e: &Env) -> (PredictIQClient, Address, Address, Address) {
    e.mock_all_auths();

    let admin = Address::generate(e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(e, &contract_id);
    client.initialize(&admin, &1000);

    let token_admin = Address::generate(e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    client.set_governance_token(&token_address);

    (client, contract_id, admin, token_address)
}

fn create_market(
    e: &Env,
    client: &PredictIQClient,
    token_address: &Address,
    num_outcomes: u32,
) -> u64 {
    let creator = Address::generate(e);
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "BTC/USD"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    let mut options = soroban_sdk::Vec::new(e);
    for i in 0..num_outcomes {
        let label = match i {
            0 => "Option 0",
            1 => "Option 1",
            2 => "Option 2",
            3 => "Option 3",
            _ => "Option X",
        };
        options.push_back(String::from_str(e, label));
    }

    client.create_market(
        &creator,
        &String::from_str(e, "Test market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        token_address,
        &0u64,
        &0u32,
    )
}

/// Force a market into `Disputed` status and inject vote tallies directly into
/// storage, bypassing the token-locking flow so we can set exact tally values.
fn set_disputed_with_tallies(
    e: &Env,
    contract_id: &Address,
    market_id: u64,
    tallies: &[(u32, i128)], // (outcome_index, weight)
) {
    e.as_contract(contract_id, || {
        // Set market to Disputed
        let mut market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.status = MarketStatus::Disputed;
        market.dispute_timestamp = Some(e.ledger().timestamp());
        e.storage()
            .persistent()
            .set(&MarketDataKey::Market(market_id), &market);

        // Inject tallies
        for (outcome, weight) in tallies {
            e.storage()
                .persistent()
                .set(&VotingDataKey::VoteTally(market_id, *outcome), weight);
        }
    });
}

/// Advance ledger time past the 72-hour voting period so `finalize_resolution`
/// can be called.
fn advance_past_voting_period(e: &Env) {
    e.ledger().with_mut(|li| {
        li.timestamp += 259_200 + 1; // 72 h + 1 s
    });
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Two outcomes with identical maximum tallies → NoMajorityReached.
#[test]
fn test_two_way_tie_returns_no_majority() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);

    let market_id = create_market(&e, &client, &token_address, 2);

    // outcome 0: 5000, outcome 1: 5000 → each 50 %, below 60 % threshold
    set_disputed_with_tallies(&e, &contract_id, market_id, &[(0, 5000), (1, 5000)]);
    advance_past_voting_period(&e);

    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::NoMajorityReached)));
}

/// Three outcomes where two share the top tally → NoMajorityReached.
#[test]
fn test_three_way_tie_at_top_returns_no_majority() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);

    let market_id = create_market(&e, &client, &token_address, 3);

    // outcome 0: 4000, outcome 1: 4000, outcome 2: 2000
    // top share = 4000/10000 = 40 % < 60 %
    set_disputed_with_tallies(
        &e,
        &contract_id,
        market_id,
        &[(0, 4000), (1, 4000), (2, 2000)],
    );
    advance_past_voting_period(&e);

    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::NoMajorityReached)));
}

/// All outcomes tied with equal votes → NoMajorityReached.
#[test]
fn test_all_outcomes_tied_returns_no_majority() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);

    let market_id = create_market(&e, &client, &token_address, 4);

    // Each outcome gets 2500 → 25 % each, well below 60 %
    set_disputed_with_tallies(
        &e,
        &contract_id,
        market_id,
        &[(0, 2500), (1, 2500), (2, 2500), (3, 2500)],
    );
    advance_past_voting_period(&e);

    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::NoMajorityReached)));
}

/// Exact 60 % supermajority (boundary) → resolves successfully (not a tie).
/// This guards against off-by-one regressions introduced while fixing ties.
#[test]
fn test_exact_60_percent_is_not_a_tie() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);

    let market_id = create_market(&e, &client, &token_address, 2);

    // outcome 0: 6000, outcome 1: 4000 → outcome 0 = 60 % exactly
    set_disputed_with_tallies(&e, &contract_id, market_id, &[(0, 6000), (1, 4000)]);
    advance_past_voting_period(&e);

    let result = client.try_finalize_resolution(&market_id);
    assert!(result.is_ok(), "60% supermajority should resolve successfully");

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(0));
}

/// After a tie, admin_fallback_resolution must succeed (tie confirms deadlock).
#[test]
fn test_admin_fallback_allowed_after_tie() {
    let e = Env::default();
    let (client, contract_id, admin, token_address) = setup(&e);

    let market_id = create_market(&e, &client, &token_address, 2);

    set_disputed_with_tallies(&e, &contract_id, market_id, &[(0, 5000), (1, 5000)]);
    advance_past_voting_period(&e);

    // Admin picks outcome 1 as the winner
    let result = client.try_admin_fallback_resolution(&market_id, &1u32);
    assert!(result.is_ok(), "admin fallback must succeed after a tie");

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(1));
}
