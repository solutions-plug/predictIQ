#![cfg(test)]

use soroban_sdk::{
    contract, contractimpl, testutils::Address as _, Address, Env, Map, String, Vec,
};

// Mock Identity Contract
#[contract]
pub struct MockIdentityContract;

#[contractimpl]
impl MockIdentityContract {
    pub fn set_verified(e: Env, user: Address, verified: bool) {
        let mut verifications: Map<Address, bool> = e
            .storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("verified"))
            .unwrap_or(Map::new(&e));

        verifications.set(user, verified);
        e.storage()
            .instance()
            .set(&soroban_sdk::symbol_short!("verified"), &verifications);
    }

    pub fn is_verify(e: Env, user: Address) -> bool {
        let verifications: Map<Address, bool> = e
            .storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("verified"))
            .unwrap_or(Map::new(&e));

        verifications.get(user).unwrap_or(false)
    }
}

mod test_identity_verification {
    use super::*;
    use predict_iq::{PredictIQ, PredictIQClient};
    use soroban_sdk::{token, testutils::Address as _};

    fn setup_test() -> (
        Env,
        PredictIQClient<'static>,
        Address,
        Address,
        token::StellarAssetClient<'static>,
        Address,
    ) {
        let e = Env::default();
        e.mock_all_auths();

        let admin = Address::generate(&e);
        let user = Address::generate(&e);

        // Create a proper token contract
        let token_admin = Address::generate(&e);
        let token_contract = e.register_stellar_asset_contract_v2(token_admin.clone());
        let token_client = token::StellarAssetClient::new(&e, &token_contract.address());

        // Mint tokens to user
        token_client.mint(&user, &1_000_000);

        let contract_id = e.register(PredictIQ, ());
        let client = PredictIQClient::new(&e, &contract_id);

        let identity_contract_id = e.register(MockIdentityContract, ());

        client.initialize(&admin, &1000);
        client.set_identity_contract(&identity_contract_id);

        (e, client, admin, user, token_client, identity_contract_id)
    }

    fn create_test_market(
        e: &Env,
        client: &PredictIQClient,
        creator: &Address,
        token: &Address,
    ) -> u64 {
        let description = String::from_str(e, "Test Market");
        let options = Vec::from_array(
            e,
            [String::from_str(e, "Yes"), String::from_str(e, "No")],
        );
        let deadline = e.ledger().timestamp() + 86400;
        let resolution_deadline = deadline + 86400;

        // Create OracleConfig with correct fields
        let oracle_config = predict_iq::types::OracleConfig {
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
        let (e, client, _admin, user, token_client, _identity_contract) = setup_test();

        let market_id = create_test_market(&e, &client, &user, &token_client.address);

        // User is not verified, should fail
        let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);

        assert_eq!(
            result,
            Err(Ok(predict_iq::errors::ErrorCode::IdentityVerificationRequired))
        );
    }

    #[test]
    fn test_verified_user_can_place_bet() {
        let (e, client, _admin, user, token_client, identity_contract_id) = setup_test();

        let market_id = create_test_market(&e, &client, &user, &token_client.address);

        // Verify the user
        let identity_client = super::MockIdentityContractClient::new(&e, &identity_contract_id);
        identity_client.set_verified(&user, &true);

        // User is verified, should succeed
        let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);

        assert!(result.is_ok());
    }

    #[test]
    fn test_revoked_verification_blocks_betting() {
        let (e, client, _admin, user, token_client, identity_contract_id) = setup_test();

        let market_id = create_test_market(&e, &client, &user, &token_client.address);

        let identity_client = super::MockIdentityContractClient::new(&e, &identity_contract_id);

        // Verify the user
        identity_client.set_verified(&user, &true);

        // User places first bet successfully
        let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);
        assert!(result.is_ok());

        // Revoke verification
        identity_client.set_verified(&user, &false);

        // User tries to place another bet, should fail
        let result = client.try_place_bet(&user, &market_id, &0, &500, &token_client.address, &None);

        assert_eq!(
            result,
            Err(Ok(predict_iq::errors::ErrorCode::IdentityVerificationRequired))
        );
    }

    #[test]
    fn test_no_identity_contract_allows_betting() {
        let e = Env::default();
        e.mock_all_auths();

        let admin = Address::generate(&e);
        let user = Address::generate(&e);

        // Create a proper token contract
        let token_admin = Address::generate(&e);
        let token_contract = e.register_stellar_asset_contract_v2(token_admin.clone());
        let token_client = token::StellarAssetClient::new(&e, &token_contract.address());

        // Mint tokens to user
        token_client.mint(&user, &1_000_000);

        let contract_id = e.register(PredictIQ, ());
        let client = PredictIQClient::new(&e, &contract_id);

        client.initialize(&admin, &1000);
        // Note: NOT setting identity contract

        let market_id = create_test_market(&e, &client, &user, &token_client.address);

        // Without identity contract set, betting should work
        let result = client.try_place_bet(&user, &market_id, &0, &1000, &token_client.address, &None);

        assert!(result.is_ok());
    }
}
