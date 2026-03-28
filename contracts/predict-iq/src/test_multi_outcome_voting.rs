#![cfg(test)]
use crate::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, Vec, String, token};

fn setup_test_env() -> (Env, Address, Address, PredictIQClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    e.budget().reset_unlimited();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &100);

    (e, admin, contract_id, client)
}

fn create_multi_outcome_market(
    client: &PredictIQClient,
    e: &Env,
    num_outcomes: u32,
    resolution_deadline: u64,
) -> u64 {
    let creator = Address::generate(e);
    let description = String::from_str(e, "Multi-Outcome Test Market");
    let mut options = soroban_sdk::Vec::new(&e);
    
    for i in 0..num_outcomes {
        options.push_back(String::from_str(e, &format!("Outcome{}", i)));
    }

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

    client.create_market(&creator, &description, &options, &100, &resolution_deadline, &oracle_config, &token_address)
}

/// Test 3-outcome market where outcome 0 wins with clear majority (70%)
#[test]
fn test_three_outcomes_clear_majority_outcome_0_wins() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Cast votes: outcome 0 gets 70%, outcome 1 gets 20%, outcome 2 gets 10%
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    let voter3 = Address::generate(&e);
    
    token_client.mint(&voter1, &7000);
    token_client.mint(&voter2, &2000);
    token_client.mint(&voter3, &1000);
    
    client.cast_vote(&voter1, &market_id, &0, &7000);
    client.cast_vote(&voter2, &market_id, &1, &2000);
    client.cast_vote(&voter3, &market_id, &2, &1000);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Finalize with voting outcome
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(0)); // Outcome 0 won with 70%
}

/// Test 3-outcome market where outcome 2 wins with clear majority (65%)
#[test]
fn test_three_outcomes_clear_majority_outcome_2_wins() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Cast votes: outcome 0 gets 20%, outcome 1 gets 15%, outcome 2 gets 65%
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    let voter3 = Address::generate(&e);
    
    token_client.mint(&voter1, &2000);
    token_client.mint(&voter2, &1500);
    token_client.mint(&voter3, &6500);
    
    client.cast_vote(&voter1, &market_id, &0, &2000);
    client.cast_vote(&voter2, &market_id, &1, &1500);
    client.cast_vote(&voter3, &market_id, &2, &6500);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Finalize with voting outcome
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(2)); // Outcome 2 won with 65%
}

/// Test 3-outcome market with no majority (55% vs 30% vs 15%)
#[test]
#[should_panic(expected = "#128")]
fn test_three_outcomes_no_majority_requires_admin() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Cast votes: outcome 0 gets 55%, outcome 1 gets 30%, outcome 2 gets 15%
    // No outcome reaches 60% threshold
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    let voter3 = Address::generate(&e);
    
    token_client.mint(&voter1, &5500);
    token_client.mint(&voter2, &3000);
    token_client.mint(&voter3, &1500);
    
    client.cast_vote(&voter1, &market_id, &0, &5500);
    client.cast_vote(&voter2, &market_id, &1, &3000);
    client.cast_vote(&voter3, &market_id, &2, &1500);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Should fail - no 60% majority
    client.finalize_resolution(&market_id);
}

/// Test 5-outcome market with clear majority (62%)
#[test]
fn test_five_outcomes_clear_majority() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 5, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Cast votes: outcome 3 gets 62%, others split remaining 38%
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    let voter3 = Address::generate(&e);
    let voter4 = Address::generate(&e);
    let voter5 = Address::generate(&e);
    
    token_client.mint(&voter1, &6200);
    token_client.mint(&voter2, &1500);
    token_client.mint(&voter3, &1000);
    token_client.mint(&voter4, &800);
    token_client.mint(&voter5, &500);
    
    client.cast_vote(&voter1, &market_id, &3, &6200);
    client.cast_vote(&voter2, &market_id, &0, &1500);
    client.cast_vote(&voter3, &market_id, &1, &1000);
    client.cast_vote(&voter4, &market_id, &2, &800);
    client.cast_vote(&voter5, &market_id, &4, &500);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Finalize with voting outcome
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(3)); // Outcome 3 won with 62%
}

/// Test 5-outcome market with no majority (all outcomes below 60%)
#[test]
#[should_panic(expected = "#128")]
fn test_five_outcomes_no_majority() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 5, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Cast votes: outcome 2 gets 45% (highest), but no outcome reaches 60%
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    let voter3 = Address::generate(&e);
    let voter4 = Address::generate(&e);
    
    token_client.mint(&voter1, &4500);
    token_client.mint(&voter2, &2500);
    token_client.mint(&voter3, &2000);
    token_client.mint(&voter4, &1000);
    
    client.cast_vote(&voter1, &market_id, &2, &4500);
    client.cast_vote(&voter2, &market_id, &0, &2500);
    client.cast_vote(&voter3, &market_id, &1, &2000);
    client.cast_vote(&voter4, &market_id, &3, &1000);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Should fail - no 60% majority
    client.finalize_resolution(&market_id);
}

/// Test edge case: exactly 60% threshold (boundary condition)
#[test]
fn test_three_outcomes_exactly_60_percent_threshold() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Cast votes: outcome 1 gets exactly 60%, outcome 0 gets 40%
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    
    token_client.mint(&voter1, &6000);
    token_client.mint(&voter2, &4000);
    
    client.cast_vote(&voter1, &market_id, &1, &6000);
    client.cast_vote(&voter2, &market_id, &0, &4000);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Should succeed - exactly 60% meets the threshold
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(1)); // Outcome 1 won with exactly 60%
}

/// Test edge case: just below 60% threshold (59.99%)
#[test]
#[should_panic(expected = "#128")]
fn test_three_outcomes_just_below_60_percent_threshold() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Cast votes: outcome 1 gets 59.99% (5999 basis points), outcome 0 gets 40.01%
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    
    token_client.mint(&voter1, &5999);
    token_client.mint(&voter2, &4001);
    
    client.cast_vote(&voter1, &market_id, &1, &5999);
    client.cast_vote(&voter2, &market_id, &0, &4001);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Should fail - 59.99% is below 60% threshold
    client.finalize_resolution(&market_id);
}

/// Table-driven tests for exact majority threshold boundary behavior
/// Validates the 6000 bps (60%) threshold with precise boundary assertions
#[test]
fn test_majority_threshold_boundary_behavior() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    // Test cases: (outcome1_votes, outcome0_votes, expected_success, description)
    let test_cases = vec![
        // Just below threshold: 59.99% vs 40.01%
        (5999, 4001, false, "59.99% - should fail (below 60% threshold)"),
        
        // Exactly at threshold: 60.00% vs 40.00%
        (6000, 4000, true, "60.00% - should succeed (exactly at threshold)"),
        
        // Just above threshold: 60.01% vs 39.99%
        (6001, 3999, true, "60.01% - should succeed (above 60% threshold)"),
        
        // Clear majority: 75.00% vs 25.00%
        (7500, 2500, true, "75.00% - should succeed (clear majority)"),
        
        // Far below threshold: 50.00% vs 50.00%
        (5000, 5000, false, "50.00% - should fail (tie, below threshold)"),
        
        // Edge case: 99.99% vs 0.01%
        (9999, 1, true, "99.99% - should succeed (overwhelming majority)"),
    ];
    
    for (outcome1_votes, outcome0_votes, expected_success, description) in test_cases {
        // Create fresh market for each test case
        let resolution_deadline = 2000;
        let market_id = create_multi_outcome_market(&client, &e, 2, resolution_deadline);
        
        client.set_oracle_result(&market_id, &0);
        
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
        
        // Cast votes according to test case
        let voter1 = Address::generate(&e);
        let voter2 = Address::generate(&e);
        
        token_client.mint(&voter1, &outcome1_votes);
        token_client.mint(&voter2, &outcome0_votes);
        
        client.cast_vote(&voter1, &market_id, &1, &outcome1_votes);
        client.cast_vote(&voter2, &market_id, &0, &outcome0_votes);
        
        // Advance time by 72 hours
        e.ledger().with_mut(|li| {
            li.timestamp = resolution_deadline + 10000 + 259200;
        });
        
        // Test the finalization result
        let result = client.try_finalize_resolution(&market_id);
        
        if expected_success {
            // Should succeed
            assert!(result.is_ok(), "Test case failed: {} - expected success but got error {:?}", description, result);
            
            let market = client.get_market(&market_id).unwrap();
            assert_eq!(market.status, types::MarketStatus::Resolved);
            assert_eq!(market.winning_outcome, Some(1));
        } else {
            // Should fail with NoMajorityReached error
            assert_eq!(result, Err(Ok(ErrorCode::NoMajorityReached)), 
                      "Test case failed: {} - expected NoMajorityReached error but got {:?}", description, result);
        }
    }
}

/// Test precise threshold calculation with larger numbers to validate division precision
#[test]
fn test_majority_threshold_precision_with_large_numbers() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    // Test with larger numbers to ensure precision is maintained
    let large_test_cases = vec![
        // Scale up by 1000x: 59.99% vs 40.01%
        (5_999_000, 4_001_000, false, "59.99% with large numbers - should fail"),
        
        // Scale up by 1000x: 60.00% vs 40.00%
        (6_000_000, 4_000_000, true, "60.00% with large numbers - should succeed"),
        
        // Scale up by 1000x: 60.01% vs 39.99%
        (6_001_000, 3_999_000, true, "60.01% with large numbers - should succeed"),
    ];
    
    for (outcome1_votes, outcome0_votes, expected_success, description) in large_test_cases {
        let resolution_deadline = 2000;
        let market_id = create_multi_outcome_market(&client, &e, 2, resolution_deadline);
        
        client.set_oracle_result(&market_id, &0);
        
        e.ledger().with_mut(|li| {
            li.timestamp = resolution_deadline;
        });
        
        client.attempt_oracle_resolution(&market_id);
        
        let disputer = Address::generate(&e);
        e.ledger().with_mut(|li| {
            li.timestamp = resolution_deadline + 10000;
        });
        
        client.file_dispute(&disputer, &market_id);
        
        let voter1 = Address::generate(&e);
        let voter2 = Address::generate(&e);
        
        token_client.mint(&voter1, &outcome1_votes);
        token_client.mint(&voter2, &outcome0_votes);
        
        client.cast_vote(&voter1, &market_id, &1, &outcome1_votes);
        client.cast_vote(&voter2, &market_id, &0, &outcome0_votes);
        
        e.ledger().with_mut(|li| {
            li.timestamp = resolution_deadline + 10000 + 259200;
        });
        
        let result = client.try_finalize_resolution(&market_id);
        
        if expected_success {
            assert!(result.is_ok(), "Large number test failed: {} - expected success", description);
        } else {
            assert_eq!(result, Err(Ok(ErrorCode::NoMajorityReached)), 
                      "Large number test failed: {} - expected NoMajorityReached", description);
        }
    }
}

/// Test single voter with 100% of votes
#[test]
fn test_three_outcomes_single_voter_100_percent() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Single voter votes for outcome 2 with 100% of votes
    let voter = Address::generate(&e);
    token_client.mint(&voter, &10000);
    client.cast_vote(&voter, &market_id, &2, &10000);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Should succeed - 100% is well above 60% threshold
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(2)); // Outcome 2 won with 100%
}

/// Test 4-outcome market with outcome 1 winning
#[test]
fn test_four_outcomes_outcome_1_wins() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 4, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Cast votes: outcome 1 gets 75%, others split remaining 25%
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    let voter3 = Address::generate(&e);
    let voter4 = Address::generate(&e);
    
    token_client.mint(&voter1, &7500);
    token_client.mint(&voter2, &1000);
    token_client.mint(&voter3, &1000);
    token_client.mint(&voter4, &500);
    
    client.cast_vote(&voter1, &market_id, &1, &7500);
    client.cast_vote(&voter2, &market_id, &0, &1000);
    client.cast_vote(&voter3, &market_id, &2, &1000);
    client.cast_vote(&voter4, &market_id, &3, &500);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Finalize with voting outcome
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(1)); // Outcome 1 won with 75%
}

/// Test vote revision changes outcome winner
#[test]
fn test_three_outcomes_vote_revision_changes_winner() {
    let (e, admin, contract_id, client) = setup_test_env();
    
    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    
    client.set_governance_token(&token_address);
    
    let resolution_deadline = 2000;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
    
    // Initial votes: outcome 0 gets 50%, outcome 1 gets 50%
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    
    token_client.mint(&voter1, &5000);
    token_client.mint(&voter2, &5000);
    
    client.cast_vote(&voter1, &market_id, &0, &5000);
    client.cast_vote(&voter2, &market_id, &1, &5000);
    
    // Voter2 revises vote to outcome 0, making it 100% for outcome 0
    client.cast_vote(&voter2, &market_id, &0, &5000);
    
    // Advance time by 72 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000 + 259200;
    });
    
    // Should succeed - outcome 0 now has 100%
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(0)); // Outcome 0 won after vote revision
}

// ── Structural regression tests ──────────────────────────────────────────────
// These tests guard against the `max_outcome = 0u32` initialisation bug where
// outcome 0 was silently treated as the winner even when it received zero votes.
// Each test is deterministic: vote weights are chosen so the expected winner and
// the majority-threshold branch are unambiguous.

/// Outcome 0 receives ZERO votes; outcome 1 wins with 80%.
/// Before the fix, `max_outcome` defaulted to 0 and the loop only updated it
/// when `tally > max_votes` (starting at 0).  Because outcome 0's tally is 0,
/// the condition `0 > 0` is false, so `max_outcome` stayed 0 — wrong winner.
#[test]
fn test_winner_is_not_outcome_0_when_outcome_0_has_zero_votes() {
    let (e, _admin, _contract_id, client) = setup_test_env();

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);

    client.set_governance_token(&token_address);

    let resolution_deadline = 2000u64;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);

    client.set_oracle_result(&market_id, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 10_000);
    client.file_dispute(&disputer, &market_id);

    // outcome 0 → 0 votes (no voter), outcome 1 → 8000, outcome 2 → 2000
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    token_client.mint(&voter1, &8000);
    token_client.mint(&voter2, &2000);
    client.cast_vote(&voter1, &market_id, &1, &8000);
    client.cast_vote(&voter2, &market_id, &2, &2000);

    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10_000 + 259_200;
    });
    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    // Must be outcome 1 (80%), NOT outcome 0 (0 votes).
    assert_eq!(market.winning_outcome, Some(1));
}

/// Outcome 0 receives ZERO votes; outcome 2 wins with exactly 60%.
/// Validates the majority-threshold branch when the winner is the last outcome
/// and outcome 0 is empty — the tightest regression case.
#[test]
fn test_winner_is_last_outcome_outcome_0_has_zero_votes_exactly_60_pct() {
    let (e, _admin, _contract_id, client) = setup_test_env();

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);

    client.set_governance_token(&token_address);

    let resolution_deadline = 2000u64;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);

    client.set_oracle_result(&market_id, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 10_000);
    client.file_dispute(&disputer, &market_id);

    // outcome 0 → 0 votes, outcome 1 → 4000, outcome 2 → 6000 (exactly 60%)
    let voter1 = Address::generate(&e);
    let voter2 = Address::generate(&e);
    token_client.mint(&voter1, &4000);
    token_client.mint(&voter2, &6000);
    client.cast_vote(&voter1, &market_id, &1, &4000);
    client.cast_vote(&voter2, &market_id, &2, &6000);

    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10_000 + 259_200;
    });
    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    // outcome 2 has exactly 60% — must pass the threshold.
    assert_eq!(market.winning_outcome, Some(2));
}

/// All outcomes receive zero votes → NoMajorityReached (#128).
/// Ensures the `total_votes == 0` guard fires before the majority check and
/// that `max_outcome` (now `Option`) never produces a spurious winner.
#[test]
#[should_panic(expected = "#128")]
fn test_no_votes_cast_returns_no_majority() {
    let (e, _admin, _contract_id, client) = setup_test_env();

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    client.set_governance_token(&token_address);

    let resolution_deadline = 2000u64;
    let market_id = create_multi_outcome_market(&client, &e, 3, resolution_deadline);

    client.set_oracle_result(&market_id, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 10_000);
    client.file_dispute(&disputer, &market_id);

    // No votes cast at all.
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10_000 + 259_200;
    });
    // Must panic with NoMajorityReached.
    client.finalize_resolution(&market_id);
}

/// Outcome 0 has votes but loses; outcome 4 wins with 70% in a 5-outcome market.
/// Validates that the loop correctly tracks the maximum across all outcomes and
/// does not short-circuit on the first outcome that beats the initial 0 baseline.
#[test]
fn test_five_outcomes_winner_is_outcome_4_not_outcome_0() {
    let (e, _admin, _contract_id, client) = setup_test_env();

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);

    client.set_governance_token(&token_address);

    let resolution_deadline = 2000u64;
    let market_id = create_multi_outcome_market(&client, &e, 5, resolution_deadline);

    client.set_oracle_result(&market_id, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 10_000);
    client.file_dispute(&disputer, &market_id);

    // outcome 0→500, 1→500, 2→500, 3→1500, 4→7000  (total 10000; outcome 4 = 70%)
    let v0 = Address::generate(&e);
    let v1 = Address::generate(&e);
    let v2 = Address::generate(&e);
    let v3 = Address::generate(&e);
    let v4 = Address::generate(&e);
    token_client.mint(&v0, &500);
    token_client.mint(&v1, &500);
    token_client.mint(&v2, &500);
    token_client.mint(&v3, &1500);
    token_client.mint(&v4, &7000);
    client.cast_vote(&v0, &market_id, &0, &500);
    client.cast_vote(&v1, &market_id, &1, &500);
    client.cast_vote(&v2, &market_id, &2, &500);
    client.cast_vote(&v3, &market_id, &3, &1500);
    client.cast_vote(&v4, &market_id, &4, &7000);

    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10_000 + 259_200;
    });
    client.finalize_resolution(&market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(4));
}

/// cast_vote with an out-of-range outcome index must return InvalidOutcome.
#[test]
fn test_cast_vote_invalid_outcome() {
    let (e, _admin, _, client) = setup_test_env();

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    client.set_governance_token(&token_address);

    let resolution_deadline = 2000;
    // Market has 2 outcomes (indices 0 and 1).
    let market_id = create_multi_outcome_market(&client, &e, 2, resolution_deadline);

    client.set_oracle_result(&market_id, &0);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline);
    client.attempt_oracle_resolution(&market_id);

    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| li.timestamp = resolution_deadline + 1000);
    client.file_dispute(&disputer, &market_id);

    let voter = Address::generate(&e);
    token_client.mint(&voter, &1000);

    // Outcome index 99 is out of range — must return InvalidOutcome.
    let result = client.try_cast_vote(&voter, &market_id, &99, &1000);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}
