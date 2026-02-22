use soroban_sdk::Env;
use crate::types::{ConfigKey, CircuitBreakerState};
use crate::modules::admin;
use crate::errors::ErrorCode;

pub fn set_state(e: &Env, state: CircuitBreakerState) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    e.storage().persistent().set(&ConfigKey::CircuitBreakerState, &state);

    // Emit standardized CircuitBreakerTriggered event
    // Topics: [CircuitBreakerTriggered, 0, contract_address]
    let contract_addr = e.current_contract_address();
    let state_str = match state {
        CircuitBreakerState::Closed => soroban_sdk::String::from_str(e, "closed"),
        CircuitBreakerState::Open => soroban_sdk::String::from_str(e, "open"),
        CircuitBreakerState::HalfOpen => soroban_sdk::String::from_str(e, "half_open"),
        CircuitBreakerState::Paused => soroban_sdk::String::from_str(e, "paused"),
    };
    crate::modules::events::emit_circuit_breaker_triggered(e, contract_addr, state_str);
    
    Ok(())
}

pub fn get_state(e: &Env) -> CircuitBreakerState {
    e.storage().persistent().get(&ConfigKey::CircuitBreakerState).unwrap_or(CircuitBreakerState::Closed)
}

pub fn require_closed(e: &Env) -> Result<(), ErrorCode> {
    let state = get_state(e);
    if state == CircuitBreakerState::Open {
        return Err(ErrorCode::CircuitBreakerOpen);
    }
    if state == CircuitBreakerState::Paused {
        return Err(ErrorCode::ContractPaused);
    }
    Ok(())
}

pub fn pause(e: &Env) -> Result<(), ErrorCode> {
    admin::require_guardian(e)?;
    e.storage().persistent().set(&ConfigKey::CircuitBreakerState, &CircuitBreakerState::Paused);
    
    // Emit standardized CircuitBreakerTriggered event
    let contract_addr = e.current_contract_address();
    crate::modules::events::emit_circuit_breaker_triggered(
        e,
        contract_addr,
        soroban_sdk::String::from_str(e, "paused"),
    );
    
    Ok(())
}

pub fn unpause(e: &Env) -> Result<(), ErrorCode> {
    admin::require_guardian(e)?;
    e.storage().persistent().set(&ConfigKey::CircuitBreakerState, &CircuitBreakerState::Closed);
    
    // Emit standardized CircuitBreakerTriggered event
    let contract_addr = e.current_contract_address();
    crate::modules::events::emit_circuit_breaker_triggered(
        e,
        contract_addr,
        soroban_sdk::String::from_str(e, "closed"),
    );
    
    Ok(())
}

pub fn require_not_paused_for_high_risk(e: &Env) -> Result<(), ErrorCode> {
    if get_state(e) == CircuitBreakerState::Paused {
        return Err(ErrorCode::ContractPaused);
    }
    Ok(())
}
