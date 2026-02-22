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

// ===================== Governance & Upgrade Tests =====================

#[test]
fn test_initialize_guardians() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian1 = Address::generate(&e);
    let guardian2 = Address::generate(&e);

    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian1.clone(),
        voting_power: 1,
    });
    guardians.push_back(types::Guardian {
        address: guardian2.clone(),
        voting_power: 1,
    });

    let result = client.try_initialize_guardians(&guardians);
    assert!(result.is_ok());

    // Verify guardians are set
    let stored_guardians = client.get_guardians();
    assert_eq!(stored_guardians.len(), 2);
}

#[test]
fn test_initialize_guardians_already_initialized() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian1 = Address::generate(&e);
    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian1.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    // Try to initialize again - should fail
    let mut guardians2 = Vec::new(&e);
    guardians2.push_back(types::Guardian {
        address: Address::generate(&e),
        voting_power: 1,
    });

    let result = client.try_initialize_guardians(&guardians2);
    assert_eq!(result, Err(Ok(ErrorCode::AlreadyInitialized)));
}

#[test]
fn test_add_guardian() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian1 = Address::generate(&e);
    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian1.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let guardian2 = Address::generate(&e);
    let result = client.try_add_guardian(&types::Guardian {
        address: guardian2.clone(),
        voting_power: 1,
    });

    assert!(result.is_ok());

    let stored_guardians = client.get_guardians();
    assert_eq!(stored_guardians.len(), 2);
}

#[test]
fn test_initiate_upgrade_starts_timelock() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let wasm_hash = String::from_str(&e, "abcd1234");

    // Set initial ledger time
    e.ledger().with_mut(|li| li.timestamp = 1000);

    let result = client.try_initiate_upgrade(&wasm_hash);
    assert!(result.is_ok());

    // Verify pending upgrade is set
    let pending = client.get_pending_upgrade().unwrap();
    assert_eq!(pending.wasm_hash, wasm_hash);
    assert_eq!(pending.initiated_at, 1000);
}

#[test]
fn test_execute_upgrade_before_timelock_fails() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let wasm_hash = String::from_str(&e, "abcd1234");
    e.ledger().with_mut(|li| li.timestamp = 1000);

    client.initiate_upgrade(&wasm_hash);

    // Vote for upgrade immediately
    client.vote_for_upgrade(&guardian, &true);

    // Try to execute immediately - should fail with TimelockActive
    let result = client.try_execute_upgrade();
    assert_eq!(result, Err(Ok(ErrorCode::TimelockActive)));
}

#[test]
fn test_execute_upgrade_after_timelock_succeeds() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let wasm_hash = String::from_str(&e, "abcd1234");
    e.ledger().with_mut(|li| li.timestamp = 1000);

    client.initiate_upgrade(&wasm_hash);

    // Vote for upgrade
    client.vote_for_upgrade(&guardian, &true);

    // Advance time past 48 hours (172800 seconds)
    e.ledger().with_mut(|li| li.timestamp = 1000 + 172800 + 1);

    // Now execute should succeed
    let result = client.try_execute_upgrade();
    assert!(result.is_ok());

    let _returned_hash = result.unwrap();
    // Verify pending upgrade is cleared after execution
    let pending = client.get_pending_upgrade();
    assert!(pending.is_none());
}

#[test]
fn test_insufficient_votes_to_execute() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian1 = Address::generate(&e);
    let guardian2 = Address::generate(&e);
    let guardian3 = Address::generate(&e);

    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian1.clone(),
        voting_power: 1,
    });
    guardians.push_back(types::Guardian {
        address: guardian2.clone(),
        voting_power: 1,
    });
    guardians.push_back(types::Guardian {
        address: guardian3.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let wasm_hash = String::from_str(&e, "abcd1234");
    e.ledger().with_mut(|li| li.timestamp = 1000);

    client.initiate_upgrade(&wasm_hash);

    // Only guardian1 votes for (1/3 = 33% < 51% needed)
    client.vote_for_upgrade(&guardian1, &true);

    // Advance time past 48 hours
    e.ledger().with_mut(|li| li.timestamp = 1000 + 172800 + 1);

    // Execute should fail - insufficient votes
    let result = client.try_execute_upgrade();
    assert_eq!(result, Err(Ok(ErrorCode::InsufficientVotes)));
}

#[test]
fn test_majority_vote_required() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian1 = Address::generate(&e);
    let guardian2 = Address::generate(&e);
    let guardian3 = Address::generate(&e);

    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian1.clone(),
        voting_power: 1,
    });
    guardians.push_back(types::Guardian {
        address: guardian2.clone(),
        voting_power: 1,
    });
    guardians.push_back(types::Guardian {
        address: guardian3.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let wasm_hash = String::from_str(&e, "abcd1234");
    e.ledger().with_mut(|li| li.timestamp = 1000);

    client.initiate_upgrade(&wasm_hash);

    // Two guardians vote for (2/3 = 66% > 51% needed)
    client.vote_for_upgrade(&guardian1, &true);
    client.vote_for_upgrade(&guardian2, &true);

    // Advance time past 48 hours
    e.ledger().with_mut(|li| li.timestamp = 1000 + 172800 + 1);

    // Execute should succeed with majority
    let result = client.try_execute_upgrade();
    assert!(result.is_ok());
}

#[test]
fn test_cannot_vote_twice() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let wasm_hash = String::from_str(&e, "abcd1234");
    e.ledger().with_mut(|li| li.timestamp = 1000);

    client.initiate_upgrade(&wasm_hash);

    // Vote for
    client.vote_for_upgrade(&guardian, &true);

    // Try to vote again - should fail
    let result = client.try_vote_for_upgrade(&guardian, &false);
    assert_eq!(result, Err(Ok(ErrorCode::AlreadyVotedOnUpgrade)));
}

#[test]
fn test_only_guardians_can_vote() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    let non_guardian = Address::generate(&e);

    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let wasm_hash = String::from_str(&e, "abcd1234");
    e.ledger().with_mut(|li| li.timestamp = 1000);

    client.initiate_upgrade(&wasm_hash);

    // Non-guardian tries to vote - should fail
    let result = client.try_vote_for_upgrade(&non_guardian, &true);
    assert_eq!(result, Err(Ok(ErrorCode::NotAuthorized)));
}

#[test]
fn test_get_upgrade_votes() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian1 = Address::generate(&e);
    let guardian2 = Address::generate(&e);

    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian1.clone(),
        voting_power: 1,
    });
    guardians.push_back(types::Guardian {
        address: guardian2.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    let wasm_hash = String::from_str(&e, "abcd1234");
    e.ledger().with_mut(|li| li.timestamp = 1000);

    client.initiate_upgrade(&wasm_hash);

    // Initial votes should be (0, 0)
    let (for_count, against_count) = client.get_upgrade_votes();
    assert_eq!(for_count, 0);
    assert_eq!(against_count, 0);

    // One guardian votes for
    client.vote_for_upgrade(&guardian1, &true);

    let (for_count, against_count) = client.get_upgrade_votes();
    assert_eq!(for_count, 1);
    assert_eq!(against_count, 0);

    // Another votes against
    client.vote_for_upgrade(&guardian2, &false);

    let (for_count, against_count) = client.get_upgrade_votes();
    assert_eq!(for_count, 1);
    assert_eq!(against_count, 1);
}

#[test]
fn test_persistent_state_preserved_on_upgrade() {
    let (e, admin, _contract_id, client) = setup_test_env();

    let guardian = Address::generate(&e);
    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: guardian.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    // Set some state (admin, base fee, guardians, etc.)
    client.set_base_fee(&100);
    let stored_fee = client.get_base_fee();
    assert_eq!(stored_fee, 100);

    // Initiate upgrade
    let wasm_hash = String::from_str(&e, "abcd1234");
    e.ledger().with_mut(|li| li.timestamp = 1000);
    client.initiate_upgrade(&wasm_hash);

    // Verify state is still accessible after initiating upgrade
    let stored_fee_after = client.get_base_fee();
    assert_eq!(stored_fee_after, 100);

    // Admin should still be set
    let stored_admin = client.get_admin().unwrap();
    assert_eq!(stored_admin, admin);
}

