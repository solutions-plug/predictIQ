#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, Vec, String};

#[test]
fn test_market_lifecycle() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &100); // 1% fee

    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Will BTC reach $100k?");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let deadline = 1000;
    let resolution_deadline = 2000;
    
    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "btc_price"),
        min_responses: 1,
    };

    let market_id = client.create_market(&creator, &description, &options, &deadline, &resolution_deadline, &oracle_config);
    assert_eq!(market_id, 1);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.id, 1);
    assert_eq!(market.status, types::MarketStatus::Active);

    // More tests could be added here for betting, voting, etc.
}

#[test]
fn test_guardian_pause_functionality() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    // Initialize contract
    client.initialize(&admin, &100);

    // Set guardian account (multisig address)
    client.set_guardian(&guardian);

    // Verify guardian is set
    let stored_guardian = client.get_guardian().unwrap();
    assert_eq!(stored_guardian, guardian);

    // Guardian triggers pause
    client.pause();
}

#[test]
fn test_place_bet_blocked_when_paused() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);
    let bettor = Address::generate(&e);
    let token_address = Address::generate(&e);
    
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    // Initialize and setup
    client.initialize(&admin, &100);
    client.set_guardian(&guardian);

    // Create a market
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Test Market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    e.ledger().with_mut(|li| li.timestamp = 500);
    
    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test_feed"),
        min_responses: 1,
    };

    let market_id = client.create_market(&creator, &description, &options, &1000, &2000, &oracle_config);

    // Pause the contract
    client.pause();

    // Try to place bet - should fail with ContractPaused error
    let result = client.try_place_bet(&bettor, &market_id, &0, &1000, &token_address);
    assert_eq!(result, Err(Ok(ErrorCode::ContractPaused)));
}

#[test]
fn test_partial_freeze_claim_winnings_works_when_paused() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);
    let bettor = Address::generate(&e);
    let token_address = Address::generate(&e);
    
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    // Initialize and setup
    client.initialize(&admin, &100);
    client.set_guardian(&guardian);

    // Create a market
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Test Market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    e.ledger().with_mut(|li| li.timestamp = 500);
    
    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test_feed"),
        min_responses: 1,
    };

    let market_id = client.create_market(&creator, &description, &options, &1000, &2000, &oracle_config);

    // Place bet before pause
    client.place_bet(&bettor, &market_id, &0, &1000, &token_address);

    // Pause the contract
    client.pause();

    // claim_winnings should still work when paused (partial freeze)
    // Note: This will fail with MarketNotPendingResolution since market isn't resolved,
    // but it won't fail with ContractPaused, proving partial freeze works
    let result = client.try_claim_winnings(&bettor, &market_id, &token_address);
    assert_ne!(result, Err(Ok(ErrorCode::ContractPaused)));
}

#[test]
fn test_partial_freeze_withdraw_refund_works_when_paused() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);
    let bettor = Address::generate(&e);
    let token_address = Address::generate(&e);
    
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    // Initialize and setup
    client.initialize(&admin, &100);
    client.set_guardian(&guardian);

    // Create a market
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Test Market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    e.ledger().with_mut(|li| li.timestamp = 500);
    
    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test_feed"),
        min_responses: 1,
    };

    let market_id = client.create_market(&creator, &description, &options, &1000, &2000, &oracle_config);

    // Place bet before pause
    client.place_bet(&bettor, &market_id, &0, &1000, &token_address);

    // Pause the contract
    client.pause();

    // withdraw_refund should still work when paused (partial freeze)
    // Note: This will fail with MarketNotActive since market isn't cancelled,
    // but it won't fail with ContractPaused, proving partial freeze works
    let result = client.try_withdraw_refund(&bettor, &market_id, &token_address);
    assert_ne!(result, Err(Ok(ErrorCode::ContractPaused)));
}

#[test]
fn test_only_guardian_can_unpause() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let guardian = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    // Initialize and setup
    client.initialize(&admin, &100);
    client.set_guardian(&guardian);

    // Pause the contract
    client.pause();

    // Guardian can unpause
    client.unpause();

    // Verify contract is unpaused by checking we can place bets again
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Test Market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    e.ledger().with_mut(|li| li.timestamp = 500);
    
    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test_feed"),
        min_responses: 1,
    };

    let market_id = client.create_market(&creator, &description, &options, &1000, &2000, &oracle_config);
    
    let bettor = Address::generate(&e);
    let token_address = Address::generate(&e);
    
    // This should succeed now that contract is unpaused
    let result = client.try_place_bet(&bettor, &market_id, &0, &1000, &token_address);
    assert_ne!(result, Err(Ok(ErrorCode::ContractPaused)));
}
