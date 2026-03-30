#![cfg(test)]
use crate::*;
use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env, Vec, String, token};

fn setup_test_env() -> (Env, Address, Address, PredictIQClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    e.budget().reset_unlimited();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    let init_guardians = {
        let mut g = soroban_sdk::Vec::new(&e);
        g.push_back(types::Guardian {
            address: Address::generate(&e),
            voting_power: 1,
        });
        g
    };
    client.initialize(&admin, &100, &init_guardians);

    (e, admin, contract_id, client)
}

fn create_test_market(
    client: &PredictIQClient,
    e: &Env,
    resolution_deadline: u64,
) -> u64 {
    let creator = Address::generate(e);
    let description = String::from_str(e, "Test Market");
    let mut options = Vec::new(e);
    options.push_back(String::from_str(e, "Yes"));
    options.push_back(String::from_str(e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    let token_admin = Address::generate(e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    client.create_market(
        &creator,
        &description,
        &options,
        &100,
        &resolution_deadline,
        &oracle_config,
        &types::MarketTier::Basic,
        &token_address,
        &0,
        &0,
    )
}

// ── Illegal transition matrix ────────────────────────────────────────────────

/// Active → Resolved is illegal: must go through PendingResolution first.
#[test]
fn test_illegal_active_to_resolved_via_finalize() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = create_test_market(&client, &e, 2000);

    // Market is Active; finalize_resolution must reject it.
    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::ResolutionNotReady)));
}

/// Active → Disputed is illegal: dispute requires PendingResolution.
#[test]
fn test_illegal_active_to_disputed() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = create_test_market(&client, &e, 2000);

    let disputer = Address::generate(&e);
    let result = client.try_file_dispute(&disputer, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotPendingResolution)));
}

/// Active → oracle resolution before deadline is illegal.
#[test]
fn test_illegal_oracle_resolution_before_deadline() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = create_test_market(&client, &e, 2000);

    client.set_oracle_result(&market_id, &0, &0);
    // Timestamp is 0 (before deadline 2000).
    let result = client.try_attempt_oracle_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::ResolutionNotReady)));
}

/// PendingResolution → oracle resolution again is illegal (already past Active).
#[test]
fn test_illegal_oracle_resolution_from_pending() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    // Market is now PendingResolution — calling again must fail.
    let result = client.try_attempt_oracle_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotActive)));
}

/// PendingResolution → finalize before dispute window closes is illegal.
#[test]
fn test_illegal_finalize_before_dispute_window() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    // 1 second inside the 72-hour window.
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1);
    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::DisputeWindowStillOpen)));
}

/// PendingResolution → dispute after window closes is illegal.
#[test]
fn test_illegal_dispute_after_window_closed() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    // 1 second past the 72-hour window.
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 259_200 + 1);
    let result = client.try_file_dispute(&disputer, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::DisputeWindowClosed)));
}

/// Disputed → finalize before voting period ends is illegal.
#[test]
fn test_illegal_finalize_disputed_before_voting_period() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    // 1 second before the 72-hour voting period ends.
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000 + 259_200 - 1);
    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::TimelockActive)));
}

/// Disputed → dispute again is illegal.
#[test]
fn test_illegal_double_dispute() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    // Second dispute on an already-Disputed market must fail.
    let result = client.try_file_dispute(&disputer, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotPendingResolution)));
}

/// Resolved → finalize again is illegal.
#[test]
fn test_illegal_finalize_already_resolved() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 259_200);
    client.finalize_resolution(&market_id);

    // Market is Resolved — calling finalize again must fail.
    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::CannotChangeOutcome)));
}

/// Resolved → dispute is illegal.
#[test]
fn test_illegal_dispute_resolved_market() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 259_200);
    client.finalize_resolution(&market_id);

    let disputer = Address::generate(&e);
    let result = client.try_file_dispute(&disputer, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotPendingResolution)));
}

/// Resolved → oracle resolution again is illegal.
#[test]
fn test_illegal_oracle_resolution_from_resolved() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 259_200);
    client.finalize_resolution(&market_id);

    let result = client.try_attempt_oracle_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotActive)));
}

/// admin_fallback_resolution on a PendingResolution market is illegal (must be Disputed).
#[test]
fn test_illegal_admin_fallback_on_pending_resolution() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let result = client.try_admin_fallback_resolution(&market_id, &0);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotDisputed)));
}

/// admin_fallback_resolution before voting period ends is illegal.
#[test]
fn test_illegal_admin_fallback_before_voting_period() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    // 1 second before voting period ends.
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000 + 259_200 - 1);
    let result = client.try_admin_fallback_resolution(&market_id, &0);
    assert_eq!(result, Err(Ok(ErrorCode::VotingPeriodNotElapsed)));
}

/// admin_fallback_resolution when community reached majority is illegal.
#[test]
fn test_illegal_admin_fallback_when_majority_exists() {
    let (e, _admin, _, client) = setup_test_env();

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    client.set_governance_token(&token_address);

    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    // 70% majority for outcome 1.
    let voter = Address::generate(&e);
    token_client.mint(&voter, &7000);
    client.cast_vote(&voter, &market_id, &1, &7000);
    let voter2 = Address::generate(&e);
    token_client.mint(&voter2, &3000);
    client.cast_vote(&voter2, &market_id, &0, &3000);

    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000 + 259_200);

    // Community has a clear majority — admin override must be rejected.
    let result = client.try_admin_fallback_resolution(&market_id, &0);
    assert_eq!(result, Err(Ok(ErrorCode::CannotChangeOutcome)));
}

/// admin_fallback_resolution with an out-of-range outcome index is illegal.
#[test]
fn test_illegal_admin_fallback_invalid_outcome() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    // No votes cast → NoMajorityReached, but outcome index 99 is out of range.
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000 + 259_200);
    let result = client.try_admin_fallback_resolution(&market_id, &99);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}

/// Cancelled → oracle resolution is illegal.
#[test]
fn test_illegal_oracle_resolution_from_cancelled() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.cancel_market_admin(&market_id);

    let result = client.try_attempt_oracle_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotActive)));
}

/// Cancelled → finalize is illegal.
#[test]
fn test_illegal_finalize_cancelled_market() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = create_test_market(&client, &e, 2000);

    client.cancel_market_admin(&market_id);

    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::ResolutionNotReady)));
}

/// Cancelled → dispute is illegal.
#[test]
fn test_illegal_dispute_cancelled_market() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = create_test_market(&client, &e, 2000);

    client.cancel_market_admin(&market_id);

    let disputer = Address::generate(&e);
    let result = client.try_file_dispute(&disputer, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotPendingResolution)));
}

// ── Legal transition matrix (happy paths) ────────────────────────────────────

#[test]
fn test_stage1_oracle_resolution_success() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    // Set oracle result
    client.set_oracle_result(&market_id, &0, &0);
    
    // Advance time to resolution deadline
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    // Attempt oracle resolution
    client.attempt_oracle_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::PendingResolution);
    assert_eq!(market.winning_outcome, Some(0));
    assert_eq!(market.pending_resolution_timestamp, Some(resolution_deadline));
}

// Boundary: finalize at exactly T+72h (dispute window boundary — legal).
#[test]
fn test_legal_finalize_at_exact_dispute_window_boundary() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    // Exactly at the boundary (>= pending_ts + window).
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 259_200);
    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
}

// Boundary: dispute at exactly T+72h-1 (last valid second — legal).
#[test]
fn test_legal_dispute_at_last_valid_second() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    // One second before the window closes (strictly less than pending_ts + window).
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 259_200 - 1);
    client.file_dispute(&disputer, &market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Disputed);
}

#[test]
fn test_stage2_finalize_after_72h_no_dispute() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    // Set oracle result and resolve
    client.set_oracle_result(&market_id, &0, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // Advance time by 72 hours (new default dispute window)
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 259_200;
    });
    
    // Finalize resolution
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(0));
}

#[test]
#[should_panic(expected = "#126")]
fn test_stage2_cannot_finalize_before_72h() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // Try to finalize before 24h
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000; // Less than 24h
    });
    
    client.finalize_resolution(&market_id);
}

#[test]
fn test_stage3_dispute_filed_within_72h() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // File dispute within 72h window
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000;
    });
    
    client.file_dispute(&disputer, &market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Disputed);
    // dispute window is tracked via pending_resolution_timestamp (set at oracle resolution)
    assert!(market.pending_resolution_timestamp.is_some());
}

#[test]
#[should_panic(expected = "#110")]
fn test_stage3_cannot_dispute_after_72h() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // Try to dispute after 72h
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 259_200 + 1;
    });
    
    client.file_dispute(&disputer, &market_id);
}

#[test]
fn test_stage4_voting_resolution_with_majority() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // File dispute
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000;
    });
    
    client.file_dispute(&disputer, &market_id);
    
    // Cast votes (70% for outcome 1, 30% for outcome 0)
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    let voter3 = Address::generate(&e);
    
    token_client.mint(&voter1, &7000);
    token_client.mint(&voter2, &2000);
    token_client.mint(&voter3, &1000);
    
    client.cast_vote(&voter1, &market_id, &1, &7000);
    client.cast_vote(&voter2, &market_id, &0, &2000);
    client.cast_vote(&voter3, &market_id, &0, &1000);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Finalize with voting outcome
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(1)); // Outcome 1 won with 70%
}

#[test]
#[should_panic(expected = "#128")]
fn test_stage4_no_majority_requires_admin() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // File dispute
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000;
    });
    
    client.file_dispute(&disputer, &market_id);
    
    // Cast votes with no clear majority (55% vs 45%)
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    
    token_client.mint(&voter1, &5500);
    token_client.mint(&voter2, &4500);
    
    client.cast_vote(&voter1, &market_id, &1, &5500);
    client.cast_vote(&voter2, &market_id, &0, &4500);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Should fail - no 60% majority
    client.finalize_resolution(&market_id);
}

#[test]
fn test_payouts_blocked_until_resolved() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    // Setup token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    let resolution_deadline = 2000;
    
    // Create market with the same token we'll use for betting
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Test Market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test"),
        min_responses: 1,
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    let market_id = client.create_market(&creator, &description, &options, &100, &resolution_deadline, &oracle_config, &token_address);
    
    // Place bet
    let bettor = Address::generate(&e);
    token_client.mint(&bettor, &1000);
    client.place_bet(&bettor, &market_id, &0, &1000, &token_address, &None);
    
    // Set oracle result
    client.set_oracle_result(&market_id, &0, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // Try to claim while PendingResolution - should fail
    let result = client.try_claim_winnings(&bettor, &market_id);
    assert!(result.is_err());
    
    // Finalize
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 259_200;
    });
    
    client.finalize_resolution(&market_id);
    
    // Now claim should work
    let payout = client.claim_winnings(&bettor, &market_id);
    assert!(payout > 0);
}

// ── Admin fallback legal path ────────────────────────────────────────────────

/// Disputed → Resolved via admin_fallback when no majority and voting period elapsed.
#[test]
fn test_legal_admin_fallback_after_deadlock() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    // No votes cast → NoMajorityReached after voting period.
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000 + 259_200);
    client.admin_fallback_resolution(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(0));
}

#[test]
fn test_resolved_at_populated_after_oracle_finalization() {
    let (e, _admin, _, client) = setup_test_env();

    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);

    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });

    client.attempt_oracle_resolution(&market_id);

    let finalize_time = resolution_deadline + 259_200; // T+72h (dispute window)
    e.ledger().with_mut(|li| {
        li.timestamp = finalize_time;
    });

    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    // resolved_at must be set so prune_market can enforce the 30-day grace period
    assert_eq!(market.resolved_at, Some(finalize_time));
}

#[test]
fn test_prune_market_succeeds_after_30_days() {
    let (e, _admin, _, client) = setup_test_env();

    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);

    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });

    client.attempt_oracle_resolution(&market_id);

    let finalize_time = resolution_deadline + 259_200;
    e.ledger().with_mut(|li| {
        li.timestamp = finalize_time;
    });

    client.finalize_resolution(&market_id);

    // Advance to exactly 30 days after resolution
    let prune_time = finalize_time + 2_592_000; // PRUNE_GRACE_PERIOD
    e.ledger().with_mut(|li| {
        li.timestamp = prune_time;
    });

    // Should succeed — 30-day grace period has elapsed
    client.prune_market(&market_id);

    // Market must no longer exist in storage
    assert!(client.get_market(&market_id).is_none());
}

#[test]
#[should_panic]
fn test_prune_market_fails_before_30_days() {
    let (e, _admin, _, client) = setup_test_env();

    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);

    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });

    client.attempt_oracle_resolution(&market_id);

    let finalize_time = resolution_deadline + 259_200;
    e.ledger().with_mut(|li| {
        li.timestamp = finalize_time;
    });

    client.finalize_resolution(&market_id);

    // Only 15 days after resolution — should fail
    e.ledger().with_mut(|li| {
        li.timestamp = finalize_time + 1_296_000; // 15 days
    });

    client.prune_market(&market_id);
}

#[test]
fn test_resolved_at_populated_after_dispute_resolution() {
    let (e, _admin, contract_id, client) = setup_test_env();

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);

    client.set_governance_token(&token_address);

    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);

    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });

    client.attempt_oracle_resolution(&market_id);

    // File dispute within 72h window
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000;
    });
    client.file_dispute(&disputer, &market_id);

    // Cast votes with clear majority
    let voter = Address::generate(&e);
    token_client.mint(&voter, &7000);
    client.cast_vote(&voter, &market_id, &1, &7000);

    // Advance past 24h dispute window + 72h voting period
    let finalize_time = resolution_deadline + 259_200 + 259_200;
    e.ledger().with_mut(|li| {
        li.timestamp = finalize_time;
    });

    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    // resolved_at must be set on the dispute path too
    assert_eq!(market.resolved_at, Some(finalize_time));
}


// ── Dispute window exact-boundary tests ──────────────────────────────────────
//
// The dispute window condition in file_dispute is:
//   CLOSED  when  now >= pending_ts + window   (>= semantics)
//   OPEN    when  now <  pending_ts + window
//
// The finalize_resolution condition is the mirror:
//   ALLOWED when  now >= pending_ts + window
//   BLOCKED when  now <  pending_ts + window
//
// All six tests below use deterministic timestamps:
//   pending_ts = 10_000
//   window     = 259_200  (default 72 h)
//   boundary   = 269_200  (= pending_ts + window)

const PENDING_TS: u64 = 10_000;
const WINDOW: u64 = 259_200;
const BOUNDARY: u64 = PENDING_TS + WINDOW; // 269_200

fn setup_pending_market(client: &PredictIQClient, e: &Env) -> u64 {
    let market_id = create_test_market(client, e, PENDING_TS);
    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = PENDING_TS);
    client.attempt_oracle_resolution(&market_id);
    market_id
}

// ── file_dispute boundary ────────────────────────────────────────────────────

/// T = boundary - 1: window still open → dispute ALLOWED.
#[test]
fn test_dispute_window_boundary_minus_one_dispute_allowed() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = setup_pending_market(&client, &e);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = BOUNDARY - 1);
    client.file_dispute(&disputer, &market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Disputed);
}

/// T = boundary (exact): window is closed → dispute REJECTED.
#[test]
fn test_dispute_window_boundary_exact_dispute_rejected() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = setup_pending_market(&client, &e);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = BOUNDARY);
    let result = client.try_file_dispute(&disputer, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::DisputeWindowClosed)));
}

/// T = boundary + 1: window is closed → dispute REJECTED.
#[test]
fn test_dispute_window_boundary_plus_one_dispute_rejected() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = setup_pending_market(&client, &e);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = BOUNDARY + 1);
    let result = client.try_file_dispute(&disputer, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::DisputeWindowClosed)));
}

// ── finalize_resolution boundary ─────────────────────────────────────────────

/// T = boundary - 1: window still open → finalize BLOCKED.
#[test]
fn test_dispute_window_boundary_minus_one_finalize_blocked() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = setup_pending_market(&client, &e);

    e.ledger().with_mut(|li| li.timestamp = BOUNDARY - 1);
    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::DisputeWindowStillOpen)));
}

/// T = boundary (exact): window just closed → finalize ALLOWED.
#[test]
fn test_dispute_window_boundary_exact_finalize_allowed() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = setup_pending_market(&client, &e);

    e.ledger().with_mut(|li| li.timestamp = BOUNDARY);
    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
}

/// T = boundary + 1: window closed → finalize ALLOWED.
#[test]
fn test_dispute_window_boundary_plus_one_finalize_allowed() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = setup_pending_market(&client, &e);

    e.ledger().with_mut(|li| li.timestamp = BOUNDARY + 1);
    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
}

// ── Issue #244: Disputed markets must use dispute_timestamp for the 72h voting window ──
//
// Before the fix, finalize_resolution in the Disputed arm could have been
// reading pending_resolution_timestamp instead of dispute_timestamp, meaning
// a market disputed long after oracle resolution could be finalized too early.
//
// These tests prove that only dispute_timestamp + 72h gates finalization,
// regardless of when pending_resolution_timestamp was set.

/// Disputed market cannot finalize before dispute_timestamp + 72h,
/// even when pending_resolution_timestamp + 72h has already elapsed.
#[test]
fn test_disputed_market_uses_dispute_timestamp_not_pending_ts() {
    let (e, _admin, _, client) = setup_test_env();

    // Oracle resolves at T=10_000 → pending_resolution_timestamp = 10_000
    let market_id = setup_pending_market(&client, &e);

    // Dispute filed at T=10_001 (well within the 72h window)
    // → dispute_timestamp = 10_001
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = 10_001);
    client.file_dispute(&disputer, &market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Disputed);
    assert_eq!(market.dispute_timestamp, Some(10_001));

    // pending_resolution_timestamp + 72h = 10_000 + 259_200 = 269_200
    // dispute_timestamp       + 72h = 10_001 + 259_200 = 269_201
    //
    // At T=269_200: pending window has elapsed but dispute window has NOT.
    // Finalize must be REJECTED.
    e.ledger().with_mut(|li| li.timestamp = 269_200);
    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::TimelockActive)));

    // At T=269_201: dispute_timestamp + 72h has elapsed → finalize ALLOWED.
    e.ledger().with_mut(|li| li.timestamp = 269_201);
    // No votes cast → NoMajorityReached, so finalize will still error,
    // but the error must be NoMajorityReached — NOT TimelockActive.
    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::NoMajorityReached)));
}

/// Disputed market with a majority vote finalizes correctly after
/// dispute_timestamp + 72h (not pending_resolution_timestamp + 72h).
#[test]
fn test_disputed_market_finalizes_after_dispute_timestamp_plus_72h() {
    let (e, _admin, _, client) = setup_test_env();

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    client.set_governance_token(&token_address);

    // Oracle resolves at T=10_000
    let market_id = setup_pending_market(&client, &e);

    // Dispute filed at T=50_000 → dispute_timestamp = 50_000
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = 50_000);
    client.file_dispute(&disputer, &market_id);

    // Cast a clear 70% majority for outcome 1
    let voter = Address::generate(&e);
    token_client.mint(&voter, &7000);
    client.cast_vote(&voter, &market_id, &1, &7000);
    let voter2 = Address::generate(&e);
    token_client.mint(&voter2, &3000);
    client.cast_vote(&voter2, &market_id, &0, &3000);

    // dispute_timestamp + 72h = 50_000 + 259_200 = 309_200
    // Attempt one second early → must be REJECTED
    e.ledger().with_mut(|li| li.timestamp = 309_199);
    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::TimelockActive)));

    // At exact boundary → must SUCCEED
    e.ledger().with_mut(|li| li.timestamp = 309_200);
    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(1));
}

// ── Issue: Zero votes on disputed finalization must return NoMajorityReached ──

/// Disputed market with no votes cast must fail finalization with NoMajorityReached,
/// not succeed or panic with an unrelated error.
#[test]
fn test_disputed_finalization_with_zero_votes_returns_no_majority() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    // No votes cast. Advance past the 72h voting period.
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000 + 259_200);

    let result = client.try_finalize_resolution(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::NoMajorityReached)));
}

// ── Cross-module InvalidOutcome assertions ────────────────────────────────────

/// resolve_market (disputes module) with an out-of-range outcome must return InvalidOutcome.
#[test]
fn test_resolve_market_invalid_outcome() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = create_test_market(&client, &e, 2000);

    let result = client.try_resolve_market(&market_id, &99);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}

/// set_oracle_result with an out-of-range outcome must return InvalidOutcome.
#[test]
fn test_set_oracle_result_invalid_outcome() {
    let (e, _admin, _, client) = setup_test_env();
    let market_id = create_test_market(&client, &e, 2000);

    // Market has 2 outcomes (0, 1); outcome 99 is out of range.
    let result = client.try_set_oracle_result(&market_id, &0, &99);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}

/// cast_vote (voting module) with an out-of-range outcome must return InvalidOutcome.
#[test]
fn test_cast_vote_invalid_outcome() {
    let (e, _admin, _, client) = setup_test_env();
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);

    client.set_oracle_result(&market_id, &0, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    let voter = Address::generate(&e);
    let result = client.try_cast_vote(&voter, &market_id, &99, &1);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}
