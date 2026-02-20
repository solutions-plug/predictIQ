#![cfg(test)]
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{Address, Env, Vec, String, token, testutils::{Address as _, Ledger}};
use crate::types::{OracleConfig, MarketStatus, Market};
use crate::errors::ErrorCode;

// Helper function to manually set market status for testing
fn set_market_to_pending_resolution(e: &Env, contract_id: &Address, market_id: u64) {
    use crate::modules::markets::DataKey;
    e.as_contract(contract_id, || {
        let mut market: Market = e.storage().persistent().get(&DataKey::Market(market_id)).unwrap();
        market.status = MarketStatus::PendingResolution;
        market.pending_resolution_timestamp = Some(e.ledger().timestamp());
        e.storage().persistent().set(&DataKey::Market(market_id), &market);
    });
}

#[test]
fn test_snapshot_voting_with_balance_change() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    // Initialize contract
    client.initialize(&admin, &1000);

    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let stellar_token = token::StellarAssetClient::new(&e, &token_address);
    let token_client = token::Client::new(&e, &token_address);

    client.set_governance_token(&token_address);

    // Create market
    let creator = Address::generate(&e);
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "BTC/USD"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(&e, "Will BTC reach $100k?"),
        &Vec::from_array(&e, [String::from_str(&e, "Yes"), String::from_str(&e, "No")]),
        &1000,
        &2000,
        &oracle_config,
        &token_address,
    );

    // Setup voter with initial balance
    let voter = Address::generate(&e);
    stellar_token.mint(&voter, &10000);

    // Move market to PendingResolution status
    set_market_to_pending_resolution(&e, &contract_id, market_id);

    // File dispute to capture snapshot
    let disciplinarian = Address::generate(&e);
    client.file_dispute(&disciplinarian, &market_id);

    // Verify market is disputed
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, MarketStatus::Disputed);
    assert!(market.dispute_snapshot_ledger.is_some());

    // Change voter balance after snapshot (simulate flash loan attempt)
    stellar_token.mint(&voter, &90000); // Now has 100000 total

    // Try to vote - should succeed with fallback since standard token doesn't support snapshots
    // In fallback mode, it will lock the requested weight
    let result = client.try_cast_vote(&voter, &market_id, &0, &5000);
    
    // Should succeed with fallback locking mechanism
    assert!(result.is_ok());
}

#[test]
fn test_zero_balance_at_snapshot() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &1000);

    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    client.set_governance_token(&token_address);

    // Create market
    let creator = Address::generate(&e);
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "BTC/USD"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(&e, "Will BTC reach $100k?"),
        &Vec::from_array(&e, [String::from_str(&e, "Yes"), String::from_str(&e, "No")]),
        &1000,
        &2000,
        &oracle_config,
        &token_address,
    );

    // File dispute first
    let disciplinarian = Address::generate(&e);
    set_market_to_pending_resolution(&e, &contract_id, market_id);
    client.file_dispute(&disciplinarian, &market_id);

    // Voter has no balance at snapshot time
    let voter = Address::generate(&e);
    
    // Try to vote with 0 balance - should fail
    let result = client.try_cast_vote(&voter, &market_id, &0, &1000);
    
    assert_eq!(result, Err(Ok(ErrorCode::InsufficientVotingWeight)));
}

#[test]
fn test_token_locking_fallback() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &1000);

    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let stellar_token = token::StellarAssetClient::new(&e, &token_address);
    let token_client = token::Client::new(&e, &token_address);

    client.set_governance_token(&token_address);

    // Create market
    let creator = Address::generate(&e);
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "BTC/USD"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(&e, "Will BTC reach $100k?"),
        &Vec::from_array(&e, [String::from_str(&e, "Yes"), String::from_str(&e, "No")]),
        &1000,
        &2000,
        &oracle_config,
        &token_address,
    );

    // Setup voter with balance
    let voter = Address::generate(&e);
    stellar_token.mint(&voter, &10000);

    // File dispute
    let disciplinarian = Address::generate(&e);
    set_market_to_pending_resolution(&e, &contract_id, market_id);
    client.file_dispute(&disciplinarian, &market_id);

    let initial_balance = token_client.balance(&voter);
    let vote_weight = 5000_i128;

    // Cast vote - tokens should be locked
    client.cast_vote(&voter, &market_id, &0, &vote_weight);

    // Verify tokens were transferred to contract
    let voter_balance_after = token_client.balance(&voter);
    let contract_balance = token_client.balance(&contract_id);

    assert_eq!(voter_balance_after, initial_balance - vote_weight);
    assert_eq!(contract_balance, vote_weight);

    // Try to unlock before resolution deadline - should fail
    let result = client.try_unlock_tokens(&voter, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::VotingNotStarted)));

    // Advance time past resolution deadline
    e.ledger().with_mut(|li| {
        li.timestamp = 2001 + (86400 * 3); // Past resolution deadline + 3 days
    });

    // Now unlock should succeed
    client.unlock_tokens(&voter, &market_id);

    // Verify tokens returned
    let final_balance = token_client.balance(&voter);
    assert_eq!(final_balance, initial_balance);
}

#[test]
fn test_insufficient_balance_for_vote_weight() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &1000);

    // Setup governance token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let stellar_token = token::StellarAssetClient::new(&e, &token_address);

    client.set_governance_token(&token_address);

    // Create market
    let creator = Address::generate(&e);
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "BTC/USD"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(&e, "Will BTC reach $100k?"),
        &Vec::from_array(&e, [String::from_str(&e, "Yes"), String::from_str(&e, "No")]),
        &1000,
        &2000,
        &oracle_config,
        &token_address,
    );

    // Setup voter with insufficient balance
    let voter = Address::generate(&e);
    stellar_token.mint(&voter, &1000);

    // File dispute
    let disciplinarian = Address::generate(&e);
    set_market_to_pending_resolution(&e, &contract_id, market_id);
    client.file_dispute(&disciplinarian, &market_id);

    // Try to vote with more weight than balance
    let result = client.try_cast_vote(&voter, &market_id, &0, &5000);
    
    assert_eq!(result, Err(Ok(ErrorCode::InsufficientVotingWeight)));
}

#[test]
fn test_dispute_captures_ledger_sequence() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &1000);

    // Setup token
    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    // Create market
    let creator = Address::generate(&e);
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "BTC/USD"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(&e, "Will BTC reach $100k?"),
        &Vec::from_array(&e, [String::from_str(&e, "Yes"), String::from_str(&e, "No")]),
        &1000,
        &2000,
        &oracle_config,
        &token_address,
    );

    // Get current ledger sequence
    let current_sequence = e.ledger().sequence();

    // File dispute
    let disciplinarian = Address::generate(&e);
    set_market_to_pending_resolution(&e, &contract_id, market_id);
    client.file_dispute(&disciplinarian, &market_id);

    // Verify snapshot ledger was captured
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.dispute_snapshot_ledger, Some(current_sequence));
}
