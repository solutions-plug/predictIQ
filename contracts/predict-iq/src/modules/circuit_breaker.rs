use crate::errors::ErrorCode;
use crate::modules::admin;
use crate::types::{CircuitBreakerState, ConfigKey};
use soroban_sdk::Env;

/// Cool-down period before Open transitions to HalfOpen (Issue #12).
const COOLDOWN_SECONDS: u64 = 6 * 3600; // 6 hours
/// Max operations allowed while in HalfOpen before auto-closing back to Closed.
const HALF_OPEN_MAX_OPS: u32 = 5;

use soroban_sdk::contracttype;

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    OpenedAt,
    HalfOpenOps,
}

fn bump_gov_ttl(_e: &Env) {
    // CircuitBreakerState is now in instance storage; no persistent TTL bump needed.
}

pub fn set_state(e: &Env, state: CircuitBreakerState) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    _set_state_internal(e, state)
}

fn _set_state_internal(e: &Env, state: CircuitBreakerState) -> Result<(), ErrorCode> {
    match state {
        CircuitBreakerState::Open => {
            e.storage().instance().set(&DataKey::OpenedAt, &e.ledger().timestamp());
        }
        CircuitBreakerState::HalfOpen => {
            e.storage().instance().set(&DataKey::HalfOpenOps, &0u32);
        }
        _ => {}
    }

    // Issue #38: CircuitBreakerState moved to instance storage so it stays
    // co-located with OpenedAt and monitoring counters — all expire together.
    e.storage()
        .instance()
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
        .instance()
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
    match state {
        CircuitBreakerState::Open | CircuitBreakerState::Paused => {
            Err(ErrorCode::ContractPaused)
        }
        CircuitBreakerState::HalfOpen => {
            let ops: u32 = e.storage().instance().get(&DataKey::HalfOpenOps).unwrap_or(0);
            if ops >= HALF_OPEN_MAX_OPS {
                // Probe limit exceeded — trip back to Open
                let _ = _set_state_internal(e, CircuitBreakerState::Open);
                return Err(ErrorCode::ContractPaused);
            }
            e.storage().instance().set(&DataKey::HalfOpenOps, &(ops + 1));
            Ok(())
        }
        CircuitBreakerState::Closed => Ok(()),
    }
}

/// Issue #50: Guardian majority can pause without Admin consent.
pub fn pause(e: &Env) -> Result<(), ErrorCode> {
    if let Some(guardian) = admin::get_guardian(e) {
        guardian.require_auth();
    } else {
        admin::require_admin(e)?;
    }

    _set_state_internal(e, CircuitBreakerState::Paused)
}

pub fn unpause(e: &Env) -> Result<(), ErrorCode> {
    if let Some(guardian) = admin::get_guardian(e) {
        guardian.require_auth();
    } else {
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
