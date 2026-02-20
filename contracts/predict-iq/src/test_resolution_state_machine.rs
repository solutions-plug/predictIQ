#![cfg(test)]
use crate::*;
use soroban_sdk::testutils::{Address as _, Ledger, LedgerInfo};
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
        min_responses: 1,
    };

    let token_admin = Address::generate(e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    client.create_market(&creator, &description, &options, &100, &resolution_deadline, &oracle_config, &token_address)
}

#[test]
fn test_stage1_oracle_resolution_success() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    // Set oracle result
    client.set_oracle_result(&market_id, &0);
    
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

#[test]
fn test_stage2_finalize_after_24h_no_dispute() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    // Set oracle result and resolve
    client.set_oracle_result(&market_id, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // Advance time by 24 hours
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 86400;
    });
    
    // Finalize resolution
    client.finalize_resolution(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(0));
}

#[test]
#[should_panic(expected = "#126")]
fn test_stage2_cannot_finalize_before_24h() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
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
fn test_stage3_dispute_filed_within_24h() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // File dispute within 24h
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 10000;
    });
    
    client.file_dispute(&disputer, &market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Disputed);
    assert!(market.dispute_timestamp.is_some());
}

#[test]
#[should_panic(expected = "#110")]
fn test_stage3_cannot_dispute_after_24h() {
    let (e, admin, _, client) = setup_test_env();
    
    let resolution_deadline = 2000;
    let market_id = create_test_market(&client, &e, resolution_deadline);
    
    client.set_oracle_result(&market_id, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // Try to dispute after 24h
    let disputer = Address::generate(&e);
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 86400 + 1;
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
    };

    let market_id = client.create_market(&creator, &description, &options, &100, &resolution_deadline, &oracle_config, &token_address);
    
    // Place bet
    let bettor = Address::generate(&e);
    token_client.mint(&bettor, &1000);
    client.place_bet(&bettor, &market_id, &0, &1000, &token_address);
    
    // Set oracle result
    client.set_oracle_result(&market_id, &0);
    
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline;
    });
    
    client.attempt_oracle_resolution(&market_id);
    
    // Try to claim while PendingResolution - should fail
    let result = client.try_claim_winnings(&bettor, &market_id);
    assert!(result.is_err());
    
    // Finalize
    e.ledger().with_mut(|li| {
        li.timestamp = resolution_deadline + 86400;
    });
    
    client.finalize_resolution(&market_id);
    
    // Now claim should work
    let payout = client.claim_winnings(&bettor, &market_id);
    assert!(payout > 0);
}
