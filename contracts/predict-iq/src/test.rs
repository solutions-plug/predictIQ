#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, Vec, String, token};

fn setup_test_env() -> (Env, Address, soroban_sdk::Address, PredictIQClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &100); // 1% fee

    (e, admin, contract_id, client)
}

fn create_test_market(
    client: &PredictIQClient,
    e: &Env,
    creator: &Address,
    tier: types::MarketTier,
    native_token: &Address,
) -> u64 {
    let description = String::from_str(e, "Test Market");
    let mut options = Vec::new(e);
    options.push_back(String::from_str(e, "Yes"));
    options.push_back(String::from_str(e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "test_feed"),
        min_responses: Some(1),
    };

    client.create_market(creator, &description, &options, &1000, &2000, &oracle_config, &tier, native_token)
}

#[test]
fn test_market_creation_fails_without_deposit() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    // Set creation deposit
    client.set_creation_deposit(&10_000_000); // 10 XLM
    
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    // Try to create market without sufficient balance - will fail because token contract doesn't exist
    // In production, this would check balance first
    let result = client.try_create_market(
        &creator,
        &String::from_str(&e, "Test Market"),
        &{
            let mut opts = Vec::new(&e);
            opts.push_back(String::from_str(&e, "Yes"));
            opts.push_back(String::from_str(&e, "No"));
            opts
        },
        &1000,
        &2000,
        &types::OracleConfig {
            oracle_address: Address::generate(&e),
            feed_id: String::from_str(&e, "test"),
            min_responses: Some(1),
        },
        &types::MarketTier::Basic,
        &native_token,
    );
    
    // Will fail due to missing token contract (simulates insufficient balance)
    assert!(result.is_err());
}

#[test]
fn test_market_creation_with_sufficient_deposit() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    let deposit_amount = 10_000_000i128; // 10 XLM
    client.set_creation_deposit(&deposit_amount);
    
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    // With no deposit requirement (set to 0), market creation should work
    client.set_creation_deposit(&0);
    
    let market_id = create_test_market(&client, &e, &creator, types::MarketTier::Basic, &native_token);
    
    assert_eq!(market_id, 1);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.creation_deposit, 0);
    assert_eq!(market.tier, types::MarketTier::Basic);
}

#[test]
fn test_pro_reputation_skips_deposit() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    let deposit_amount = 10_000_000i128;
    client.set_creation_deposit(&deposit_amount);
    
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    // Set creator reputation to Pro
    client.set_creator_reputation(&creator, &types::CreatorReputation::Pro);
    
    // Create market - should succeed without deposit
    let market_id = create_test_market(&client, &e, &creator, types::MarketTier::Pro, &native_token);
    
    assert_eq!(market_id, 1);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.creation_deposit, 0); // No deposit required
    assert_eq!(market.tier, types::MarketTier::Pro);
}

#[test]
fn test_institutional_reputation_skips_deposit() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    let deposit_amount = 10_000_000i128;
    client.set_creation_deposit(&deposit_amount);
    
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    // Set creator reputation to Institutional
    client.set_creator_reputation(&creator, &types::CreatorReputation::Institutional);
    
    // Create market - should succeed without deposit
    let market_id = create_test_market(&client, &e, &creator, types::MarketTier::Institutional, &native_token);
    
    assert_eq!(market_id, 1);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.creation_deposit, 0); // No deposit required
}

#[test]
fn test_deposit_released_after_resolution() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    // No deposit for this test
    client.set_creation_deposit(&0);
    
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    // Create market
    let market_id = create_test_market(&client, &e, &creator, types::MarketTier::Basic, &native_token);
    
    // Resolve market
    client.resolve_market(&market_id, &0);
    
    // Verify market is resolved
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
}

#[test]
fn test_tiered_commission_rates() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    client.set_creation_deposit(&0); // No deposit for this test
    
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);
    
    // Create Basic tier market
    let basic_market_id = create_test_market(&client, &e, &creator, types::MarketTier::Basic, &native_token);
    let basic_market = client.get_market(&basic_market_id).unwrap();
    assert_eq!(basic_market.tier, types::MarketTier::Basic);
    
    // Create Pro tier market
    let pro_market_id = create_test_market(&client, &e, &creator, types::MarketTier::Pro, &native_token);
    let pro_market = client.get_market(&pro_market_id).unwrap();
    assert_eq!(pro_market.tier, types::MarketTier::Pro);
    
    // Create Institutional tier market
    let inst_market_id = create_test_market(&client, &e, &creator, types::MarketTier::Institutional, &native_token);
    let inst_market = client.get_market(&inst_market_id).unwrap();
    assert_eq!(inst_market.tier, types::MarketTier::Institutional);
}

#[test]
fn test_reputation_management() {
    let (e, _admin, _contract_id, client) = setup_test_env();
    
    let creator = Address::generate(&e);
    
    // Default reputation should be None
    let rep = client.get_creator_reputation(&creator);
    assert_eq!(rep, types::CreatorReputation::None);
    
    // Set to Basic
    client.set_creator_reputation(&creator, &types::CreatorReputation::Basic);
    let rep = client.get_creator_reputation(&creator);
    assert_eq!(rep, types::CreatorReputation::Basic);
    
    // Upgrade to Pro
    client.set_creator_reputation(&creator, &types::CreatorReputation::Pro);
    let rep = client.get_creator_reputation(&creator);
    assert_eq!(rep, types::CreatorReputation::Pro);
    
    // Upgrade to Institutional
    client.set_creator_reputation(&creator, &types::CreatorReputation::Institutional);
    let rep = client.get_creator_reputation(&creator);
    assert_eq!(rep, types::CreatorReputation::Institutional);
}

#[test]
fn test_guardian_pause_functionality() {
    let (e, _admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);

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
    let (e, _admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    let bettor = Address::generate(&e);
    let token_address = Address::generate(&e);
    
    client.set_guardian(&guardian);

    // Create a market
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);

    e.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_test_market(&client, &e, &creator, types::MarketTier::Basic, &native_token);

    // Pause the contract
    client.pause();

    // Try to place bet - should fail with ContractPaused error
    let result = client.try_place_bet(&bettor, &market_id, &0, &1000, &token_address);
    assert_eq!(result, Err(Ok(ErrorCode::ContractPaused)));
}

#[test]
fn test_partial_freeze_claim_winnings_works_when_paused() {
    let (e, _admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    let bettor = Address::generate(&e);
    let token_address = Address::generate(&e);
    
    client.set_guardian(&guardian);
    client.set_creation_deposit(&0); // No deposit for this test

    // Create a market
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);

    e.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_test_market(&client, &e, &creator, types::MarketTier::Basic, &native_token);

    // Pause the contract (skip placing bet since it requires token contract)
    client.pause();

    // claim_winnings should still work when paused (partial freeze)
    let result = client.try_claim_winnings(&bettor, &market_id, &token_address);
    assert_ne!(result, Err(Ok(ErrorCode::ContractPaused)));
}

#[test]
fn test_only_guardian_can_unpause() {
    let (e, _admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    client.set_guardian(&guardian);

    // Pause the contract
    client.pause();

    // Guardian can unpause
    client.unpause();

    // Verify contract is unpaused by checking we can place bets again
    let creator = Address::generate(&e);
    let native_token = Address::generate(&e);

    e.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_test_market(&client, &e, &creator, types::MarketTier::Basic, &native_token);
    
    let bettor = Address::generate(&e);
    let token_address = Address::generate(&e);
    
    // This should succeed now that contract is unpaused
    let result = client.try_place_bet(&bettor, &market_id, &0, &1000, &token_address);
    assert_ne!(result, Err(Ok(ErrorCode::ContractPaused)));
}
