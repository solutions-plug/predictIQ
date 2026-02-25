#![cfg(test)]
use crate::errors::ErrorCode;
use crate::types::{CreatorReputation, MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec};

fn setup() -> (Env, PredictIQClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &100);

    (env, client, admin)
}

#[test]
fn test_create_market_basic() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Test Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert_eq!(market_id, 1);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, MarketStatus::Active);
    assert_eq!(market.tier, MarketTier::Basic);
}

#[test]
fn test_create_market_with_single_option_fails() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(&env, [String::from_str(&env, "Only One")]);

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Invalid Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}

#[test]
fn test_create_market_with_too_many_outcomes() {
    let (env, client, admin) = setup();

    let mut options = Vec::new(&env);
    for i in 0..101 {
        options.push_back(String::from_str(&env, &format!("Option{}", i)));
    }

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Too Many Outcomes"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert_eq!(result, Err(Ok(ErrorCode::TooManyOutcomes)));
}

#[test]
fn test_create_market_deadline_in_past() {
    let (env, client, admin) = setup();

    env.ledger().with_mut(|li| li.timestamp = 1000);

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Past Deadline"),
        &options,
        &500, // Deadline in the past
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert_eq!(result, Err(Ok(ErrorCode::InvalidDeadline)));
}

#[test]
fn test_create_market_resolution_before_deadline() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Invalid Deadlines"),
        &options,
        &2000,
        &1000, // Resolution deadline before betting deadline
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert_eq!(result, Err(Ok(ErrorCode::InvalidDeadline)));
}

#[test]
fn test_market_id_increments() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    let id1 = client.create_market(
        &admin,
        &String::from_str(&env, "Market 1"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    let id2 = client.create_market(
        &admin,
        &String::from_str(&env, "Market 2"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert_eq!(id1, 1);
    assert_eq!(id2, 2);
}

#[test]
fn test_get_nonexistent_market() {
    let (_env, client, _admin) = setup();

    let market = client.get_market(&999);
    assert!(market.is_none());
}

#[test]
fn test_creator_reputation_default() {
    let (env, client, _admin) = setup();

    let creator = Address::generate(&env);
    let reputation = client.get_creator_reputation(&creator);

    assert_eq!(reputation, CreatorReputation::None);
}

#[test]
fn test_set_creator_reputation() {
    let (env, client, _admin) = setup();

    let creator = Address::generate(&env);

    client.set_creator_reputation(&creator, &CreatorReputation::Pro);

    let reputation = client.get_creator_reputation(&creator);
    assert_eq!(reputation, CreatorReputation::Pro);
}

#[test]
fn test_creation_deposit_default() {
    let (_env, client, _admin) = setup();

    let deposit = client.get_creation_deposit();
    assert_eq!(deposit, 0);
}

#[test]
fn test_set_creation_deposit() {
    let (_env, client, _admin) = setup();

    client.set_creation_deposit(&10_000_000);

    let deposit = client.get_creation_deposit();
    assert_eq!(deposit, 10_000_000);
}

#[test]
fn test_market_tiers() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    // Create markets with different tiers
    let basic_id = client.create_market(
        &admin,
        &String::from_str(&env, "Basic Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    let pro_id = client.create_market(
        &admin,
        &String::from_str(&env, "Pro Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Pro,
        &token,
        &0,
        &0,
    );

    let inst_id = client.create_market(
        &admin,
        &String::from_str(&env, "Institutional Market"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Institutional,
        &token,
        &0,
        &0,
    );

    assert_eq!(client.get_market(&basic_id).unwrap().tier, MarketTier::Basic);
    assert_eq!(client.get_market(&pro_id).unwrap().tier, MarketTier::Pro);
    assert_eq!(
        client.get_market(&inst_id).unwrap().tier,
        MarketTier::Institutional
    );
}

#[test]
fn test_prune_market_before_grace_period() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Test Market"),
        &options,
        &2000,
        &3000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Resolve market
    client.resolve_market(&market_id, &0);

    // Try to prune immediately (before 30 days)
    let result = client.try_prune_market(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::GracePeriodActive)));
}

#[test]
fn test_prune_market_after_grace_period() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Test Market"),
        &options,
        &2000,
        &3000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Resolve market
    client.resolve_market(&market_id, &0);

    // Advance time past 30 days (2,592,000 seconds)
    env.ledger().with_mut(|li| li.timestamp = 1000 + 2_592_001);

    // Prune should succeed
    let result = client.try_prune_market(&market_id);
    assert!(result.is_ok());
}

#[test]
fn test_prune_unresolved_market_fails() {
    let (env, client, admin) = setup();

    let options = Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
    };

    let token = Address::generate(&env);

    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Test Market"),
        &options,
        &2000,
        &3000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Try to prune without resolving
    let result = client.try_prune_market(&market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotResolved)));
}
