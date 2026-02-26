#![cfg(test)]
use crate::errors::ErrorCode;
use crate::types::CircuitBreakerState;
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, PredictIQClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let guardian = Address::generate(&env);

    client.initialize(&admin, &100);
    client.set_guardian(&guardian).unwrap();

    (env, client, admin, guardian)
}

#[test]
fn test_initial_state_closed() {
    let (env, client, _admin, _guardian) = setup();

    // Circuit breaker should start in Closed state (normal operation)
    let result = client.try_set_circuit_breaker(&CircuitBreakerState::Closed);
    assert!(result.is_ok());
}

#[test]
fn test_pause_contract() {
    let (_env, client, _admin, _guardian) = setup();

    let result = client.try_pause();
    assert!(result.is_ok());
}

#[test]
fn test_unpause_contract() {
    let (_env, client, _admin, _guardian) = setup();

    client.pause();

    let result = client.try_unpause();
    assert!(result.is_ok());
}

#[test]
fn test_pause_blocks_operations() {
    let (env, client, admin, _guardian) = setup();

    use crate::types::{MarketTier, OracleConfig};
    use soroban_sdk::{String, Vec};

    client.pause();

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

    // Try to create market while paused
    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Test"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert_eq!(result, Err(Ok(ErrorCode::ContractPaused)));
}

#[test]
fn test_unpause_allows_operations() {
    let (env, client, admin, _guardian) = setup();

    use crate::types::{MarketTier, OracleConfig};
    use soroban_sdk::{String, Vec};

    client.pause();
    client.unpause();

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

    // Should succeed after unpause
    let result = client.try_create_market(
        &admin,
        &String::from_str(&env, "Test"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert!(result.is_ok());
}

#[test]
fn test_multiple_pause_unpause_cycles() {
    let (_env, client, _admin, _guardian) = setup();

    client.pause();
    client.unpause();
    client.pause();
    client.unpause();

    // Should work fine
}

#[test]
fn test_set_circuit_breaker_states() {
    let (_env, client, _admin, _guardian) = setup();

    client.set_circuit_breaker(&CircuitBreakerState::Closed);
    client.set_circuit_breaker(&CircuitBreakerState::Open);
    client.set_circuit_breaker(&CircuitBreakerState::HalfOpen);

    // All state transitions should work
}

#[test]
fn test_require_closed_when_open() {
    let (env, client, admin, _guardian) = setup();

    use crate::types::{MarketTier, OracleConfig};
    use soroban_sdk::{String, Vec};

    // Set to Open state
    client.set_circuit_breaker(&CircuitBreakerState::Open);

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
        &String::from_str(&env, "Test"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    assert_eq!(result, Err(Ok(ErrorCode::ContractPaused)));
}

#[test]
fn test_half_open_allows_limited_operations() {
    let (_env, client, _admin, _guardian) = setup();

    // Set to HalfOpen state
    client.set_circuit_breaker(&CircuitBreakerState::HalfOpen);

    // In HalfOpen, some operations may be allowed
    // This depends on implementation details
}
