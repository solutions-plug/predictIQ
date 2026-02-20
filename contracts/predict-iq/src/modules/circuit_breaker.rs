use soroban_sdk::{Env, Symbol};
use crate::types::{ConfigKey, CircuitBreakerState};
use crate::modules::admin;
use crate::errors::ErrorCode;

pub fn set_state(e: &Env, state: CircuitBreakerState) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    e.storage().persistent().set(&ConfigKey::CircuitBreakerState, &state);

    // Event format: (Topic, MarketID, SubjectAddr, Data) - no market_id for global state
    e.events().publish(
        (Symbol::new(e, "circuit_breaker_updated"),),
        state,
    );
    
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
    
    e.events().publish(
        (Symbol::new(e, "contract_paused"),),
        (),
    );
    
    Ok(())
}

pub fn unpause(e: &Env) -> Result<(), ErrorCode> {
    admin::require_guardian(e)?;
    e.storage().persistent().set(&ConfigKey::CircuitBreakerState, &CircuitBreakerState::Closed);
    
    e.events().publish(
        (Symbol::new(e, "contract_unpaused"),),
        (),
    );
    
    Ok(())
}

pub fn require_not_paused_for_high_risk(e: &Env) -> Result<(), ErrorCode> {
    if get_state(e) == CircuitBreakerState::Paused {
        return Err(ErrorCode::ContractPaused);
    }
    Ok(())
}
