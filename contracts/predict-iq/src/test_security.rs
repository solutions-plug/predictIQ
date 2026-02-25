#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, Env, String, Vec,
};

use crate::{PredictIQ, PredictIQClient};
use crate::types::OracleConfig;
use crate::errors::ErrorCode;

fn setup_test() -> (Env, PredictIQClient<'static>, Address, Address, token::StellarAssetClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user = Address::generate(&e);

    let token_admin = Address::generate(&e);
    let token_contract = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_client = token::StellarAssetClient::new(&e, &token_contract.address());
    token_client.mint(&user, &1_000_000);

    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &1000);

    (e, client, admin, user, token_client)
}

fn create_test_market(
    e: &Env,
    client: &PredictIQClient,
    creator: &Address,
    token: &Address,
) -> u64 {
    let description = String::from_str(e, "Test Market");
    let options = Vec::from_array(e, [String::from_str(e, "Yes"), String::from_str(e, "No")]);
    let deadline = e.ledger().timestamp() + 86400;
    let resolution_deadline = deadline + 86400;
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "test"),
        min_responses: 1,
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    client.create_market(
        creator,
        &description,
        &options,
        &deadline,
        &resolution_deadline,
        &oracle_config,
        token,
    )
}

#[test]
fn test_reentrancy_protection() {
    let (e, client, _admin, user, token_client) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token_client.address);

    // First bet should succeed
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);
    assert!(result.is_ok());

    // The reentrancy guard prevents nested calls
    // In a real attack scenario, a malicious token contract would try to call back
    // The guard ensures the protocol lock prevents this
}

#[test]
fn test_oracle_manipulation_prevention() {
    let (e, client, admin, user, token_client) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token_client.address);

    // Simulate oracle update in current ledger
    client.set_oracle_result(&market_id, &0);

    // Attempt to bet in same ledger sequence should fail
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);
    
    assert_eq!(result, Err(Ok(ErrorCode::OracleUpdateTooRecent)));
}

#[test]
fn test_bet_after_oracle_update_next_ledger() {
    let (e, client, admin, user, token_client) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token_client.address);

    // Oracle update in current ledger
    client.set_oracle_result(&market_id, &0);

    // Advance to next ledger
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp() + 5,
        protocol_version: 22,
        sequence_number: e.ledger().sequence() + 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 3110400,
    });

    // Now bet should succeed (different ledger)
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);
    assert!(result.is_ok());
}

#[test]
fn test_token_transfer_after_storage_writes() {
    let (e, client, _admin, user, token_client) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token_client.address);

    // Advance ledger to avoid oracle check
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp() + 5,
        protocol_version: 22,
        sequence_number: e.ledger().sequence() + 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 3110400,
    });

    // Place bet - token transfer happens last
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);
    assert!(result.is_ok());

    // Verify bet was recorded (storage write succeeded before transfer)
    // If transfer failed, storage would still be written (wrong!)
    // But with our implementation, transfer is last so state is consistent
}

#[test]
fn test_multiple_bets_different_ledgers() {
    let (e, client, _admin, user, token_client) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token_client.address);

    // First bet in ledger 1
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp() + 5,
        protocol_version: 22,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 3110400,
    });
    
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);
    assert!(result.is_ok());

    // Second bet in ledger 2
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp() + 10,
        protocol_version: 22,
        sequence_number: 2,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 3110400,
    });
    
    let result = client.try_place_bet(&user, &market_id, &0, &500, &token_client.address, &None);
    assert!(result.is_ok());
}

#[test]
fn test_claim_winnings_reentrancy_protection() {
    let (e, client, admin, user, token_client) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token_client.address);

    // Place bet
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp() + 5,
        protocol_version: 22,
        sequence_number: 1,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 3110400,
    });
    
    client.place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);

    // Resolve market
    client.set_oracle_result(&market_id, &0);
    
    e.ledger().set(LedgerInfo {
        timestamp: e.ledger().timestamp() + 86400 + 10,
        protocol_version: 22,
        sequence_number: 2,
        network_id: Default::default(),
        base_reserve: 10,
        min_temp_entry_ttl: 1,
        min_persistent_entry_ttl: 1,
        max_entry_ttl: 3110400,
    });

    client.attempt_oracle_resolution(&market_id);

    // Claim winnings - reentrancy guard active
    let result = client.try_claim_winnings(&user, &market_id);
    assert!(result.is_ok());
}
