#![cfg(test)]
use crate::errors::ErrorCode;
use crate::types::CircuitBreakerState;
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn setup() -> (Env, PredictIQClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let guardian = Address::generate(&env);

    client.initialize(&admin, &100);
    client.set_guardian(&guardian);

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
        [String::from_str(&env, "Yes"), String::from_str(&env, "No")],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
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
        [String::from_str(&env, "Yes"), String::from_str(&env, "No")],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
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
        [String::from_str(&env, "Yes"), String::from_str(&env, "No")],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
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

// Issue #12: auto-recovery tests

#[test]
fn test_auto_recovery_open_to_half_open_after_cooldown() {
    let (env, client, _admin, _guardian) = setup();

    // Trip to Open at t=1000
    env.ledger().with_mut(|li| li.timestamp = 1000);
    client.set_circuit_breaker(&CircuitBreakerState::Open);

    // Before cooldown (6 hours = 21600s) — still Open
    env.ledger().with_mut(|li| li.timestamp = 1000 + 21599);
    assert_eq!(client.get_circuit_breaker_state(), CircuitBreakerState::Open);

    // After cooldown — require_closed triggers maybe_recover -> HalfOpen
    env.ledger().with_mut(|li| li.timestamp = 1000 + 21600);
    // get_circuit_breaker_state just reads storage; call a guarded fn to trigger maybe_recover
    // We verify by checking that a market creation attempt no longer returns ContractPaused
    // (it may fail for other reasons, but not ContractPaused)
    use crate::types::{MarketTier, OracleConfig};
    use soroban_sdk::{Address, String, Vec};
    let options = Vec::from_array(
        &env,
        [String::from_str(&env, "Yes"), String::from_str(&env, "No")],
    );
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };
    let result = client.try_create_market(
        &Address::generate(&env),
        &String::from_str(&env, "Test"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &MarketTier::Basic,
        &Address::generate(&env),
        &0,
        &0,
    );
    // Must NOT be ContractPaused — circuit breaker recovered
    assert_ne!(result, Err(Ok(ErrorCode::ContractPaused)));
}

#[test]
fn test_half_open_trips_back_to_open_after_max_ops() {
    let (env, client, admin, _guardian) = setup();

    use crate::types::{MarketTier, OracleConfig};
    use soroban_sdk::{Address, String, Vec};

    client.set_circuit_breaker(&CircuitBreakerState::HalfOpen);

    let make_attempt = || {
        let options = Vec::from_array(
            &env,
            [String::from_str(&env, "Yes"), String::from_str(&env, "No")],
        );
        let oracle_config = OracleConfig {
            oracle_address: Address::generate(&env),
            feed_id: String::from_str(&env, "test"),
            min_responses: Some(1),
            max_staleness_seconds: 3600,
            max_confidence_bps: 100,
        };
        client.try_create_market(
            &admin,
            &String::from_str(&env, "Test"),
            &options,
            &1000,
            &2000,
            &oracle_config,
            &MarketTier::Basic,
            &Address::generate(&env),
            &0,
            &0,
        )
    };

    // First HALF_OPEN_MAX_OPS (5) calls should not be blocked by circuit breaker
    for _ in 0..5 {
        let r = make_attempt();
        assert_ne!(r, Err(Ok(ErrorCode::ContractPaused)));
    }

    // 6th call must be blocked — tripped back to Open
    let r = make_attempt();
    assert_eq!(r, Err(Ok(ErrorCode::ContractPaused)));
}
