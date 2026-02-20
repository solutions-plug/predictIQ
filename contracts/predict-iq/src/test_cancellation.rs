#![cfg(test)]
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::Address as _,
    token, Address, Env, String, Vec,
};

fn setup_test() -> (Env, PredictIQClient<'static>, Address, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();
    
    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    let user1 = Address::generate(&env);
    let user2 = Address::generate(&env);
    
    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    
    let token_client = token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&user1, &10000);
    token_client.mint(&user2, &10000);
    
    client.initialize(&admin, &100);
    
    (env, client, admin, user1, user2, token_address)
}

#[test]
fn test_admin_cancel_market() {
    let (env, client, _admin, user1, _user2, token_address) = setup_test();
    
    let oracle = Address::generate(&env);
    let options = Vec::from_array(&env, [String::from_str(&env, "Yes"), String::from_str(&env, "No")]);
    
    let market_id = client.create_market(
        &user1,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &crate::types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
        },
        &token_address,
    );
    
    client.place_bet(&user1, &market_id, &0, &1000, &token_address);
    
    client.cancel_market_admin(&market_id);
    
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, crate::types::MarketStatus::Cancelled);
}

#[test]
fn test_withdraw_refund_full_amount() {
    let (env, client, _admin, user1, user2, token_address) = setup_test();
    
    let token_client = token::Client::new(&env, &token_address);
    let oracle = Address::generate(&env);
    let options = Vec::from_array(&env, [String::from_str(&env, "Yes"), String::from_str(&env, "No")]);
    
    let market_id = client.create_market(
        &user1,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &crate::types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
        },
        &token_address,
    );
    
    let bet_amount_user1 = 1000i128;
    let bet_amount_user2 = 2000i128;
    
    client.place_bet(&user1, &market_id, &0, &bet_amount_user1, &token_address);
    client.place_bet(&user2, &market_id, &1, &bet_amount_user2, &token_address);
    
    let user1_balance_before = token_client.balance(&user1);
    let user2_balance_before = token_client.balance(&user2);
    
    client.cancel_market_admin(&market_id);
    
    let refund1 = client.withdraw_refund(&user1, &market_id);
    let refund2 = client.withdraw_refund(&user2, &market_id);
    
    assert_eq!(refund1, bet_amount_user1);
    assert_eq!(refund2, bet_amount_user2);
    
    assert_eq!(token_client.balance(&user1), user1_balance_before + bet_amount_user1);
    assert_eq!(token_client.balance(&user2), user2_balance_before + bet_amount_user2);
}

#[test]
fn test_refund_no_fee_collected() {
    let (env, client, _admin, user1, _user2, token_address) = setup_test();
    
    let oracle = Address::generate(&env);
    let options = Vec::from_array(&env, [String::from_str(&env, "Yes"), String::from_str(&env, "No")]);
    
    let market_id = client.create_market(
        &user1,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &crate::types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
        },
        &token_address,
    );
    
    client.place_bet(&user1, &market_id, &0, &1000, &token_address);
    
    let revenue_before = client.get_revenue(&token_address);
    
    client.cancel_market_admin(&market_id);
    client.withdraw_refund(&user1, &market_id);
    
    let revenue_after = client.get_revenue(&token_address);
    
    assert_eq!(revenue_before, revenue_after);
}

#[test]
#[should_panic(expected = "Error(Contract, #129)")]
fn test_refund_only_on_cancelled_market() {
    let (env, client, _admin, user1, _user2, token_address) = setup_test();
    
    let oracle = Address::generate(&env);
    let options = Vec::from_array(&env, [String::from_str(&env, "Yes"), String::from_str(&env, "No")]);
    
    let market_id = client.create_market(
        &user1,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &crate::types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
        },
        &token_address,
    );
    
    client.place_bet(&user1, &market_id, &0, &1000, &token_address);
    
    client.withdraw_refund(&user1, &market_id);
}

#[test]
#[should_panic(expected = "Error(Contract, #121)")]
fn test_refund_only_once() {
    let (env, client, _admin, user1, _user2, token_address) = setup_test();
    
    let oracle = Address::generate(&env);
    let options = Vec::from_array(&env, [String::from_str(&env, "Yes"), String::from_str(&env, "No")]);
    
    let market_id = client.create_market(
        &user1,
        &String::from_str(&env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &crate::types::OracleConfig {
            oracle_address: oracle,
            feed_id: String::from_str(&env, "test"),
            min_responses: 1,
        },
        &token_address,
    );
    
    client.place_bet(&user1, &market_id, &0, &1000, &token_address);
    
    client.cancel_market_admin(&market_id);
    client.withdraw_refund(&user1, &market_id);
    client.withdraw_refund(&user1, &market_id);
}
