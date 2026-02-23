#![cfg(test)]
use crate::{PredictIQ, PredictIQClient};
use crate::mock_identity::{MockIdentityContract, MockIdentityContractClient};
use crate::types::OracleConfig;
use crate::errors::ErrorCode;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Env, String, Vec,
};

fn setup_test() -> (Env, PredictIQClient<'static>, MockIdentityContractClient<'static>, Address, Address, Address) {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = Address::generate(&e);

    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    let identity_contract_id = e.register_contract(None, MockIdentityContract);
    let identity_client = MockIdentityContractClient::new(&e, &identity_contract_id);

    client.initialize(&admin, &1000);
    client.set_identity_contract(&identity_contract_id);

    (e, client, identity_client, admin, user, token)
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
fn test_unverified_user_cannot_place_bet() {
    let (e, client, _identity_client, _admin, user, token) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token);
    
    // User is not verified, should fail
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    
    assert_eq!(result, Err(Ok(ErrorCode::IdentityVerificationRequired)));
}

#[test]
fn test_verified_user_can_place_bet() {
    let (e, client, identity_client, _admin, user, token) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token);
    
    // Verify the user
    identity_client.set_verified(&user, &true);
    
    // User is verified, should succeed
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    
    assert!(result.is_ok());
}

#[test]
fn test_revoked_verification_blocks_betting() {
    let (e, client, identity_client, _admin, user, token) = setup_test();
    
    let market_id = create_test_market(&e, &client, &user, &token);
    
    // Verify the user
    identity_client.set_verified(&user, &true);
    
    // User places first bet successfully
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert!(result.is_ok());
    
    // Revoke verification
    identity_client.set_verified(&user, &false);
    
    // User tries to place another bet, should fail
    let result = client.try_place_bet(&user, &market_id, &0, &500, &token, &None);
    
    assert_eq!(result, Err(Ok(ErrorCode::IdentityVerificationRequired)));
}

#[test]
fn test_no_identity_contract_allows_betting() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let user = Address::generate(&e);
    let token = Address::generate(&e);

    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &1000);
    // Note: NOT setting identity contract
    
    let market_id = create_test_market(&e, &client, &user, &token);
    
    // Without identity contract set, betting should work
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    
    assert!(result.is_ok());
}
