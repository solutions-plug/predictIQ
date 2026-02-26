#![cfg(test)]
use crate::errors::ErrorCode;
use crate::types::{BetKey, Market, MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::Address as _, token, Address, Env, String, Vec,
};

fn setup_test_with_token() -> (Env, PredictIQClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &100);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    let user = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&user, &100_000);

    (env, client, admin, user, token_address)
}

fn create_simple_market(
    client: &PredictIQClient,
    env: &Env,
    creator: &Address,
    token: &Address,
) -> u64 {
    let options = Vec::from_array(
        env,
        [
            String::from_str(env, "Yes"),
            String::from_str(env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "test"),
        min_responses: Some(1),
    };

    client.create_market(
        creator,
        &String::from_str(env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &oracle_config,
        &MarketTier::Basic,
        token,
        &0,
        &0,
    )
}

#[test]
fn test_place_bet_success() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert!(result.is_ok());
}

#[test]
fn test_place_bet_zero_amount() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    let result = client.try_place_bet(&user, &market_id, &0, &0, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidAmount)));
}

#[test]
fn test_place_bet_negative_amount() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    let result = client.try_place_bet(&user, &market_id, &0, &-100, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidAmount)));
}

#[test]
fn test_place_bet_invalid_outcome() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Market has outcomes 0 and 1, try outcome 2
    let result = client.try_place_bet(&user, &market_id, &2, &1000, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}

#[test]
fn test_place_bet_after_deadline() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Advance time past deadline
    env.ledger().with_mut(|li| li.timestamp = 1500);

    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::MarketClosed)));
}

#[test]
fn test_place_bet_on_resolved_market() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Resolve market
    client.resolve_market(&market_id, &0);

    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::MarketClosed)));
}

#[test]
fn test_place_multiple_bets_same_outcome() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user, &market_id, &0, &2000, &token, &None);

    // Both bets should succeed
}

#[test]
fn test_place_bets_different_outcomes() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user, &market_id, &1, &2000, &token, &None);

    // User can bet on different outcomes
}

#[test]
fn test_claim_winnings_success() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);

    // Resolve market with outcome 0 (user wins)
    client.resolve_market(&market_id, &0);

    let result = client.try_claim_winnings(&user, &market_id, &token);
    assert!(result.is_ok());
}

#[test]
fn test_claim_winnings_losing_bet() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);

    // Resolve market with outcome 1 (user loses)
    client.resolve_market(&market_id, &1);

    let result = client.try_claim_winnings(&user, &market_id, &token);
    assert_eq!(result, Err(Ok(ErrorCode::NoWinnings)));
}

#[test]
fn test_claim_winnings_before_resolution() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);

    let result = client.try_claim_winnings(&user, &market_id, &token);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotResolved)));
}

#[test]
fn test_claim_winnings_twice() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.resolve_market(&market_id, &0);

    client.claim_winnings(&user, &market_id, &token);

    // Second claim should fail
    let result = client.try_claim_winnings(&user, &market_id, &token);
    assert_eq!(result, Err(Ok(ErrorCode::AlreadyClaimed)));
}

#[test]
fn test_claim_winnings_no_bet_placed() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.resolve_market(&market_id, &0);

    let result = client.try_claim_winnings(&user, &market_id, &token);
    assert_eq!(result, Err(Ok(ErrorCode::NoWinnings)));
}

#[test]
fn test_winnings_calculation_single_winner() {
    let (env, client, _admin, user1, token) = setup_test_with_token();

    let user2 = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&user2, &100_000);

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user1, &token);

    // User1 bets 1000 on outcome 0
    client.place_bet(&user1, &market_id, &0, &1000, &token, &None);

    // User2 bets 2000 on outcome 1
    client.place_bet(&user2, &market_id, &1, &2000, &token, &None);

    // Resolve with outcome 0 (user1 wins)
    client.resolve_market(&market_id, &0);

    let balance_before = token_client.balance(&user1);
    let winnings = client.claim_winnings(&user1, &market_id, &token);
    let balance_after = token_client.balance(&user1);

    // User1 should get their 1000 back plus share of losing pool (minus fees)
    assert!(winnings > 1000);
    assert_eq!(balance_after - balance_before, winnings);
}

#[test]
fn test_winnings_calculation_multiple_winners() {
    let (env, client, _admin, user1, token) = setup_test_with_token();

    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token);
    token_client.mint(&user2, &100_000);
    token_client.mint(&user3, &100_000);

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user1, &token);

    // User1 and User2 bet on outcome 0
    client.place_bet(&user1, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user2, &market_id, &0, &2000, &token, &None);

    // User3 bets on outcome 1
    client.place_bet(&user3, &market_id, &1, &3000, &token, &None);

    // Resolve with outcome 0
    client.resolve_market(&market_id, &0);

    let winnings1 = client.claim_winnings(&user1, &market_id, &token);
    let winnings2 = client.claim_winnings(&user2, &market_id, &token);

    // User2 bet twice as much, should get twice the winnings
    assert!(winnings2 > winnings1);
    assert!(winnings1 > 1000); // More than original bet
    assert!(winnings2 > 2000); // More than original bet
}

#[test]
fn test_referral_rewards_tracked() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    let referrer = Address::generate(&env);

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Place bet with referrer
    client.place_bet(&user, &market_id, &0, &1000, &token, &Some(referrer.clone()));

    // Referrer should have pending rewards
    let rewards = client.claim_referral_rewards(&referrer, &token);
    assert!(rewards.is_ok());
}

#[test]
fn test_bet_with_self_referral_rejected() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Try to refer yourself
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &Some(user.clone()));
    assert_eq!(result, Err(Ok(ErrorCode::InvalidReferrer)));
}
