// Common test utilities and helpers

use predict_iq::{PredictIQ, PredictIQClient};
use soroban_sdk::{token, Address, Env, String, Vec};

/// Setup test environment with initialized contract
pub fn setup() -> (Env, PredictIQClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &100);

    (env, client, admin)
}

/// Setup test environment with token contract
pub fn setup_with_token() -> (Env, PredictIQClient<'static>, Address, Address) {
    let (env, client, admin) = setup();

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    (env, client, admin, token_address)
}

/// Create a simple test market
pub fn create_market(
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

    let oracle_config = predict_iq::types::OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "test_feed"),
        min_responses: Some(1),
    };

    client.create_market(
        creator,
        &String::from_str(env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &oracle_config,
        &predict_iq::types::MarketTier::Basic,
        token,
        &0,
        &0,
    )
}

/// Create a market with custom parameters
pub fn create_custom_market(
    client: &PredictIQClient,
    env: &Env,
    creator: &Address,
    token: &Address,
    description: &str,
    num_outcomes: u32,
    deadline: u64,
    resolution_deadline: u64,
    tier: predict_iq::types::MarketTier,
) -> u64 {
    let mut options = Vec::new(env);
    for i in 0..num_outcomes {
        options.push_back(String::from_str(env, &format!("Outcome {}", i)));
    }

    let oracle_config = predict_iq::types::OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "test_feed"),
        min_responses: Some(1),
    };

    client.create_market(
        creator,
        &String::from_str(env, description),
        &options,
        &deadline,
        &resolution_deadline,
        &oracle_config,
        &tier,
        token,
        &0,
        &0,
    )
}

/// Setup user with token balance
pub fn setup_user_with_balance(env: &Env, token: &Address, amount: i128) -> Address {
    let user = Address::generate(env);
    let token_client = token::StellarAssetClient::new(env, token);
    token_client.mint(&user, &amount);
    user
}

/// Advance ledger time
pub fn advance_time(env: &Env, seconds: u64) {
    env.ledger().with_mut(|li| {
        li.timestamp += seconds;
    });
}

/// Assert market status
pub fn assert_market_status(
    client: &PredictIQClient,
    market_id: u64,
    expected_status: predict_iq::types::MarketStatus,
) {
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, expected_status);
}

/// Assert balance change
pub fn assert_balance_change(
    env: &Env,
    token: &Address,
    user: &Address,
    expected_delta: i128,
    operation: impl FnOnce(),
) {
    let token_client = token::Client::new(env, token);
    let balance_before = token_client.balance(user);

    operation();

    let balance_after = token_client.balance(user);
    let actual_delta = balance_after - balance_before;

    assert_eq!(
        actual_delta, expected_delta,
        "Expected balance change of {}, got {}",
        expected_delta, actual_delta
    );
}

/// Create multiple users with balances
pub fn create_users(env: &Env, token: &Address, count: u32, balance: i128) -> Vec<Address> {
    let mut users = Vec::new(env);
    let token_client = token::StellarAssetClient::new(env, token);

    for _ in 0..count {
        let user = Address::generate(env);
        token_client.mint(&user, &balance);
        users.push_back(user);
    }

    users
}

/// Setup guardians for governance
pub fn setup_guardians(
    client: &PredictIQClient,
    env: &Env,
    count: u32,
) -> Vec<predict_iq::types::Guardian> {
    let mut guardians = Vec::new(env);

    for _ in 0..count {
        guardians.push_back(predict_iq::types::Guardian {
            address: Address::generate(env),
            voting_power: 1,
        });
    }

    client.initialize_guardians(&guardians);
    guardians
}

/// Place bet and return bet amount
pub fn place_bet_helper(
    client: &PredictIQClient,
    user: &Address,
    market_id: u64,
    outcome: u32,
    amount: i128,
    token: &Address,
) -> i128 {
    client.place_bet(user, &market_id, &outcome, &amount, token, &None);
    amount
}

/// Resolve market and verify status
pub fn resolve_and_verify(
    client: &PredictIQClient,
    market_id: u64,
    winning_outcome: u32,
) {
    client.resolve_market(&market_id, &winning_outcome);
    assert_market_status(client, market_id, predict_iq::types::MarketStatus::Resolved);
}
