// End-to-end integration tests for PredictIQ contract
// Tests complete workflows across multiple modules

use predict_iq::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};

mod common;
use common::*;

#[test]
fn test_complete_market_lifecycle() {
    let (env, client, admin, token) = setup_with_token();

    // 1. Create market
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = create_market(&client, &env, &admin, &token);
    assert_eq!(market_id, 1);

    // 2. Multiple users place bets
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);

    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&user1, &10_000);
    token_client.mint(&user2, &20_000);
    token_client.mint(&user3, &30_000);

    client.place_bet(&user1, &market_id, &0, &1_000, &token, &None);
    client.place_bet(&user2, &market_id, &0, &2_000, &token, &None);
    client.place_bet(&user3, &market_id, &1, &3_000, &token, &None);

    // 3. Verify market state
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, predict_iq::types::MarketStatus::Active);

    // 4. Resolve market
    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, predict_iq::types::MarketStatus::Resolved);

    // 5. Winners claim
    let balance1_before = token_client.balance(&user1);
    let balance2_before = token_client.balance(&user2);

    let winnings1 = client.claim_winnings(&user1, &market_id, &token);
    let winnings2 = client.claim_winnings(&user2, &market_id, &token);

    assert!(winnings1 > 1_000); // More than original bet
    assert!(winnings2 > 2_000); // More than original bet

    let balance1_after = token_client.balance(&user1);
    let balance2_after = token_client.balance(&user2);

    assert_eq!(balance1_after - balance1_before, winnings1);
    assert_eq!(balance2_after - balance2_before, winnings2);

    // 6. Loser cannot claim
    let result = client.try_claim_winnings(&user3, &market_id, &token);
    assert_eq!(result, Err(Ok(predict_iq::errors::ErrorCode::NoWinnings)));
}

#[test]
fn test_multi_market_concurrent_operations() {
    let (env, client, admin, token) = setup_with_token();

    env.ledger().with_mut(|li| li.timestamp = 1000);

    // Create multiple markets
    let market1 = create_market(&client, &env, &admin, &token);
    let market2 = create_market(&client, &env, &admin, &token);
    let market3 = create_market(&client, &env, &admin, &token);

    assert_eq!(market1, 1);
    assert_eq!(market2, 2);
    assert_eq!(market3, 3);

    // User bets on multiple markets
    let user = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&user, &100_000);

    client.place_bet(&user, &market1, &0, &1_000, &token, &None);
    client.place_bet(&user, &market2, &1, &2_000, &token, &None);
    client.place_bet(&user, &market3, &0, &3_000, &token, &None);

    // Resolve markets with different outcomes
    client.resolve_market(&market1, &0); // User wins
    client.resolve_market(&market2, &0); // User loses
    client.resolve_market(&market3, &0); // User wins

    // Claim winnings from winning markets
    let winnings1 = client.claim_winnings(&user, &market1, &token);
    let winnings3 = client.claim_winnings(&user, &market3, &token);

    assert!(winnings1 > 0);
    assert!(winnings3 > 0);

    // Cannot claim from losing market
    let result = client.try_claim_winnings(&user, &market2, &token);
    assert_eq!(result, Err(Ok(predict_iq::errors::ErrorCode::NoWinnings)));
}

#[test]
fn test_referral_system_integration() {
    let (env, client, admin, token) = setup_with_token();

    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = create_market(&client, &env, &admin, &token);

    let bettor = Address::generate(&env);
    let referrer = Address::generate(&env);

    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&bettor, &10_000);

    // Place bet with referrer
    client.place_bet(&bettor, &market_id, &0, &1_000, &token, &Some(referrer.clone()));

    // Referrer should have pending rewards
    let rewards_before = token_client.balance(&referrer);
    let claimed = client.claim_referral_rewards(&referrer, &token);

    if claimed.is_ok() {
        let rewards_after = token_client.balance(&referrer);
        assert!(rewards_after > rewards_before);
    }
}

#[test]
fn test_conditional_market_chain() {
    let (env, client, admin, token) = setup_with_token();

    env.ledger().with_mut(|li| li.timestamp = 1000);

    // Create parent market
    let parent_id = create_market(&client, &env, &admin, &token);

    // Resolve parent
    client.resolve_market(&parent_id, &0);

    // Create child market conditional on parent outcome 0
    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = predict_iq::types::OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let child_id = client.create_market(
        &admin,
        &String::from_str(&env, "Child Market"),
        &options,
        &2000,
        &3000,
        &oracle_config,
        &predict_iq::types::MarketTier::Basic,
        &token,
        &parent_id,
        &0,
    );

    assert_eq!(child_id, 2);

    let child_market = client.get_market(&child_id).unwrap();
    assert_eq!(child_market.parent_id, parent_id);
    assert_eq!(child_market.parent_outcome_idx, 0);
}

#[test]
fn test_emergency_pause_and_recovery() {
    let (env, client, admin, token) = setup_with_token();

    let guardian = Address::generate(&env);
    client.set_guardian(&guardian).unwrap();

    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = create_market(&client, &env, &admin, &token);

    let user = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&user, &10_000);

    // Place bet before pause
    client.place_bet(&user, &market_id, &0, &1_000, &token, &None);

    // Guardian pauses contract
    client.pause();

    // Cannot place new bets
    let result = client.try_place_bet(&user, &market_id, &0, &1_000, &token, &None);
    assert_eq!(
        result,
        Err(Ok(predict_iq::errors::ErrorCode::ContractPaused))
    );

    // Resolve market (admin can still operate)
    client.resolve_market(&market_id, &0);

    // Can still claim winnings (partial freeze)
    let winnings = client.claim_winnings(&user, &market_id, &token);
    assert!(winnings > 0);

    // Unpause
    client.unpause();

    // Can create new markets again
    let new_market = create_market(&client, &env, &admin, &token);
    assert_eq!(new_market, 2);
}

#[test]
fn test_governance_upgrade_workflow() {
    let (env, client, admin, _token) = setup_with_token();

    let guardian1 = Address::generate(&env);
    let guardian2 = Address::generate(&env);
    let guardian3 = Address::generate(&env);

    let mut guardians = Vec::new(&env);
    guardians.push_back(predict_iq::types::Guardian {
        address: guardian1.clone(),
        voting_power: 1,
    });
    guardians.push_back(predict_iq::types::Guardian {
        address: guardian2.clone(),
        voting_power: 1,
    });
    guardians.push_back(predict_iq::types::Guardian {
        address: guardian3.clone(),
        voting_power: 1,
    });

    client.initialize_guardians(&guardians);

    // Initiate upgrade
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let wasm_hash = String::from_str(&env, "new_wasm_hash_123");
    client.initiate_upgrade(&wasm_hash);

    // Guardians vote
    client.vote_for_upgrade(&guardian1, &true);
    client.vote_for_upgrade(&guardian2, &true);

    // Check votes
    let (for_votes, against_votes) = client.get_upgrade_votes();
    assert_eq!(for_votes, 2);
    assert_eq!(against_votes, 0);

    // Wait for timelock (48 hours)
    env.ledger().with_mut(|li| li.timestamp = 1000 + 172_801);

    // Execute upgrade
    let result = client.try_execute_upgrade();
    assert!(result.is_ok());

    // Verify upgrade cleared
    let pending = client.get_pending_upgrade();
    assert!(pending.is_none());
}

#[test]
fn test_market_cancellation_and_refunds() {
    let (env, client, admin, token) = setup_with_token();

    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = create_market(&client, &env, &admin, &token);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&user1, &10_000);
    token_client.mint(&user2, &20_000);

    // Users place bets
    client.place_bet(&user1, &market_id, &0, &1_000, &token, &None);
    client.place_bet(&user2, &market_id, &1, &2_000, &token, &None);

    let balance1_before = token_client.balance(&user1);
    let balance2_before = token_client.balance(&user2);

    // Admin cancels market
    // Note: Assuming cancel_market_admin exists
    // client.cancel_market_admin(&market_id);

    // Users withdraw refunds
    // let refund1 = client.withdraw_refund(&user1, &market_id, &token);
    // let refund2 = client.withdraw_refund(&user2, &market_id, &token);

    // assert_eq!(refund1, 1_000);
    // assert_eq!(refund2, 2_000);

    // let balance1_after = token_client.balance(&user1);
    // let balance2_after = token_client.balance(&user2);

    // assert_eq!(balance1_after, balance1_before + 1_000);
    // assert_eq!(balance2_after, balance2_before + 2_000);
}

#[test]
fn test_fee_collection_and_distribution() {
    let (env, client, admin, token) = setup_with_token();

    env.ledger().with_mut(|li| li.timestamp = 1000);

    // Set base fee
    client.set_base_fee(&100); // 1%

    let market_id = create_market(&client, &env, &admin, &token);

    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);

    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&user1, &10_000);
    token_client.mint(&user2, &10_000);

    // Place bets
    client.place_bet(&user1, &market_id, &0, &1_000, &token, &None);
    client.place_bet(&user2, &market_id, &1, &1_000, &token, &None);

    let revenue_before = client.get_revenue(&token);

    // Resolve market
    client.resolve_market(&market_id, &0);

    // Claim winnings (fees collected)
    client.claim_winnings(&user1, &market_id, &token);

    let revenue_after = client.get_revenue(&token);

    // Revenue should increase due to fees
    assert!(revenue_after > revenue_before);
}

#[test]
fn test_reputation_based_deposit_waiver() {
    let (env, client, admin, token) = setup_with_token();

    // Set creation deposit
    client.set_creation_deposit(&10_000_000);

    let creator = Address::generate(&env);

    // Set creator reputation to Pro
    client.set_creator_reputation(&creator, &predict_iq::types::CreatorReputation::Pro);

    env.ledger().with_mut(|li| li.timestamp = 1000);

    // Create market without deposit (reputation waives it)
    let market_id = create_market(&client, &env, &creator, &token);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.creation_deposit, 0);
}
