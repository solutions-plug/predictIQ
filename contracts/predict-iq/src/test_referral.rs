#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _};
use soroban_sdk::{Address, Env, Vec, String, token};

fn setup_test() -> (Env, PredictIQClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let admin = Address::generate(&env);
    let contract_id = env.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&env, &contract_id);
    
    let user_a = Address::generate(&env);
    let user_b = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    
    client.initialize(&admin, &100); // 1% fee (100 basis points)
    
    (env, client, user_a, user_b, token_address, contract_id)
}

#[test]
fn test_referral_reward_calculation() {
    let (env, client, user_a, user_b, token_address, _contract_id) = setup_test();
    
    let oracle = Address::generate(&env);
    let mut options = Vec::new(&env);
    options.push_back(String::from_str(&env, "Yes"));
    options.push_back(String::from_str(&env, "No"));
    
    let market_id = client.create_market(
        &user_a,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
            max_staleness_seconds: 300,
            max_confidence_bps: 200,
        },
        &token_address,
    );
    
    // User B bets 1000 tokens with User A as referrer
    let bet_amount = 1000i128;
    client.place_bet(&user_b, &market_id, &0, &bet_amount, &token_address, &Some(user_a.clone()));
    
    // Fee is 1% of 1000 = 10 tokens
    // Referral reward is 10% of fee = 1 token
    let expected_reward = 1i128;
    
    // Claim referral rewards
    let claimed = client.claim_referral_rewards(&user_a, &token_address);
    assert_eq!(claimed, expected_reward);
}

#[test]
fn test_multiple_referral_claims() {
    let (env, client, user_a, user_b, token_address, _contract_id) = setup_test();
    
    let oracle = Address::generate(&env);
    let mut options = Vec::new(&env);
    options.push_back(String::from_str(&env, "Yes"));
    options.push_back(String::from_str(&env, "No"));
    
    let market_id = client.create_market(
        &user_a,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
            max_staleness_seconds: 300,
            max_confidence_bps: 200,
        },
        &token_address,
    );
    
    // First bet
    client.place_bet(&user_b, &market_id, &0, &1000, &token_address, &Some(user_a.clone()));
    
    // Claim first reward
    let first_claim = client.claim_referral_rewards(&user_a, &token_address);
    assert_eq!(first_claim, 1i128);
    
    // Second bet
    client.place_bet(&user_b, &market_id, &0, &2000, &token_address, &Some(user_a.clone()));
    
    // Claim second reward
    let second_claim = client.claim_referral_rewards(&user_a, &token_address);
    assert_eq!(second_claim, 2i128); // 10% of (1% of 2000) = 2
    
    // Verify market is still active
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Active);
}

#[test]
fn test_no_referrer_no_crash() {
    let (env, client, user_a, user_b, token_address, _contract_id) = setup_test();
    
    let oracle = Address::generate(&env);
    let mut options = Vec::new(&env);
    options.push_back(String::from_str(&env, "Yes"));
    options.push_back(String::from_str(&env, "No"));
    
    let market_id = client.create_market(
        &user_a,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
            max_staleness_seconds: 300,
            max_confidence_bps: 200,
        },
        &token_address,
    );
    
    // Bet without referrer
    client.place_bet(&user_b, &market_id, &0, &1000, &token_address, &None);
    
    // Should not crash, market should be active
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Active);
}

#[test]
fn test_nonexistent_referrer_no_crash() {
    let (env, client, user_a, user_b, token_address, _contract_id) = setup_test();
    
    let oracle = Address::generate(&env);
    let mut options = Vec::new(&env);
    options.push_back(String::from_str(&env, "Yes"));
    options.push_back(String::from_str(&env, "No"));
    
    let market_id = client.create_market(
        &user_a,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
            max_staleness_seconds: 300,
            max_confidence_bps: 200,
        },
        &token_address,
    );
    
    // Use a random address as referrer
    let random_referrer = Address::generate(&env);
    client.place_bet(&user_b, &market_id, &0, &1000, &token_address, &Some(random_referrer.clone()));
    
    // Should not crash
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Active);
    
    // Random referrer should be able to claim their reward
    let claimed = client.claim_referral_rewards(&random_referrer, &token_address);
    assert_eq!(claimed, 1i128);
}
