use crate::errors::ErrorCode;
use crate::modules::admin;
use crate::types::{CircuitBreakerState, ConfigKey, GOV_TTL_LOW_THRESHOLD, GOV_TTL_HIGH_THRESHOLD};
use soroban_sdk::Env;

/// Cool-down period before Open transitions to HalfOpen (Issue #12).
const COOLDOWN_SECONDS: u64 = 3600; // 1 hour

use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    OpenedAt,
}

fn bump_gov_ttl(e: &Env) {
    e.storage()
        .persistent()
        .extend_ttl(&ConfigKey::CircuitBreakerState, GOV_TTL_LOW_THRESHOLD, GOV_TTL_HIGH_THRESHOLD);
}

pub fn set_state(e: &Env, state: CircuitBreakerState) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    _set_state_internal(e, state)
}

fn _set_state_internal(e: &Env, state: CircuitBreakerState) -> Result<(), ErrorCode> {
    if state == CircuitBreakerState::Open {
        // Record when it was opened for cool-down tracking
        e.storage()
            .instance()
            .set(&crate::modules::circuit_breaker::DataKey::OpenedAt, &e.ledger().timestamp());
    }

    e.storage()
        .persistent()
        .set(&ConfigKey::CircuitBreakerState, &state);
    bump_gov_ttl(e);

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
    e.storage()
        .persistent()
        .get(&ConfigKey::CircuitBreakerState)
        .unwrap_or(CircuitBreakerState::Closed)
}

/// Issue #12: Automatically transition Open -> HalfOpen after cool-down.
pub fn maybe_recover(e: &Env) {
    if get_state(e) != CircuitBreakerState::Open {
        return;
    }

    let opened_at: u64 = e
        .storage()
        .instance()
        .get(&DataKey::OpenedAt)
        .unwrap_or(0);

    if e.ledger().timestamp() >= opened_at + COOLDOWN_SECONDS {
        let _ = _set_state_internal(e, CircuitBreakerState::HalfOpen);
    }
}

pub fn require_closed(e: &Env) -> Result<(), ErrorCode> {
    maybe_recover(e);
    let state = get_state(e);
    if state == CircuitBreakerState::Open {
        return Err(ErrorCode::CircuitBreakerOpen);
    }
    if state == CircuitBreakerState::Paused {
        return Err(ErrorCode::ContractPaused);
    }
    Ok(())
}

/// Issue #50: Guardian majority can pause without Admin consent.
pub fn pause(e: &Env) -> Result<(), ErrorCode> {
    // Try guardian first; fall back to admin
    let guardian_ok = admin::get_guardian(e)
        .map(|g| g.try_require_auth().is_ok())
        .unwrap_or(false);

    if !guardian_ok {
        admin::require_admin(e)?;
    }

    _set_state_internal(e, CircuitBreakerState::Paused)
}

pub fn unpause(e: &Env) -> Result<(), ErrorCode> {
    let guardian_ok = admin::get_guardian(e)
        .map(|g| g.try_require_auth().is_ok())
        .unwrap_or(false);

    if !guardian_ok {
        admin::require_admin(e)?;
    }

    _set_state_internal(e, CircuitBreakerState::Closed)
}

pub fn require_not_paused_for_high_risk(e: &Env) -> Result<(), ErrorCode> {
    if get_state(e) == CircuitBreakerState::Paused {
        return Err(ErrorCode::ContractPaused);
    }
    Ok(())
}
