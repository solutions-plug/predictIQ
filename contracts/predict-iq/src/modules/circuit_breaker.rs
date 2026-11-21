use crate::errors::ErrorCode;
use crate::modules::admin;
use crate::types::{CircuitBreakerState, ConfigKey};
use soroban_sdk::{Address, Env};

/// Cool-down period before Open transitions to HalfOpen (Issue #12).
const COOLDOWN_SECONDS: u64 = 6 * 3600; // 6 hours
/// Max operations allowed while in HalfOpen before auto-closing back to Closed.
const HALF_OPEN_MAX_OPS: u32 = 5;
/// Default threshold (max loss per block in stroops) used when none is stored.
pub const DEFAULT_CIRCUIT_BREAKER_THRESHOLD: i128 = 1_000_000_000;

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

/// Issue #1190/#1191: entrypoint for governance's guardian-majority emergency
/// pause. Writes/TTL-manages CircuitBreakerState the same way every other
/// transition does, avoiding the persistent-storage TTL mismatch from directly
/// poking instance storage elsewhere.
pub(crate) fn force_pause(e: &Env) -> Result<(), ErrorCode> {
    _set_state_internal(e, CircuitBreakerState::Paused)
}

fn _set_state_internal(e: &Env, state: CircuitBreakerState) -> Result<(), ErrorCode> {
    match state {
        CircuitBreakerState::Open => {
            e.storage()
                .instance()
                .set(&DataKey::OpenedAt, &e.ledger().timestamp());
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

    let opened_at: u64 = e.storage().instance().get(&DataKey::OpenedAt).unwrap_or(0);

    if e.ledger().timestamp() >= opened_at + COOLDOWN_SECONDS {
        let _ = _set_state_internal(e, CircuitBreakerState::HalfOpen);
    }
}

pub fn require_closed(e: &Env) -> Result<(), ErrorCode> {
    maybe_recover(e);
    let state = get_state(e);
    match state {
        CircuitBreakerState::Open | CircuitBreakerState::Paused => Err(ErrorCode::ContractPaused),
        CircuitBreakerState::HalfOpen => {
            let ops: u32 = e
                .storage()
                .instance()
                .get(&DataKey::HalfOpenOps)
                .unwrap_or(0);
            if ops >= HALF_OPEN_MAX_OPS {
                // Probe limit exceeded — trip back to Open
                let _ = _set_state_internal(e, CircuitBreakerState::Open);
                return Err(ErrorCode::ContractPaused);
            }
            e.storage()
                .instance()
                .set(&DataKey::HalfOpenOps, &(ops + 1));
            Ok(())
        }
        CircuitBreakerState::Closed => Ok(()),
    }
}

/// Issue #50: Guardian majority can pause without Admin consent.
/// Issue #1848: the admin may also pause even when a guardian is configured,
/// so an unavailable guardian cannot block an emergency pause.
pub fn pause(e: &Env) -> Result<(), ErrorCode> {
    require_guardian_or_admin(e)?;
    _set_state_internal(e, CircuitBreakerState::Paused)
}

/// Admin-or-guardian pause. The admin may always pause, even when a guardian is set.
pub fn pause_as(e: &Env, caller: Address) -> Result<(), ErrorCode> {
    require_admin_or_guardian(e, &caller)?;
    _set_state_internal(e, CircuitBreakerState::Paused)
}

/// Admin-or-guardian unpause. Same state transition as `unpause`.
pub fn unpause_as(e: &Env, caller: Address) -> Result<(), ErrorCode> {
    require_admin_or_guardian(e, &caller)?;
    _set_state_internal(e, CircuitBreakerState::Closed)
}

fn require_admin_or_guardian(e: &Env, caller: &Address) -> Result<(), ErrorCode> {
    let is_admin = admin::get_admin(e).as_ref() == Some(caller);
    let is_guardian = admin::get_guardian(e).as_ref() == Some(caller);
    if !is_admin && !is_guardian {
        return Err(ErrorCode::NotAuthorized);
    }
    caller.require_auth();
    Ok(())
}

/// Issue #1848: the admin may also unpause even when a guardian is configured.
pub fn unpause(e: &Env) -> Result<(), ErrorCode> {
    require_guardian_or_admin(e)?;
    _set_state_internal(e, CircuitBreakerState::Closed)
}

/// Issue #1848: authorize a pause/unpause via the configured guardian or, as a
/// fallback, the admin. When no guardian is set this is admin-only, matching
/// the previous behaviour.
fn require_guardian_or_admin(e: &Env) -> Result<(), ErrorCode> {
    if let Some(guardian) = admin::get_guardian(e) {
        if guardian.try_auth().is_ok() {
            return Ok(());
        }
    }
    admin::require_admin(e)
}

/// Issue #1705: Open and Paused block high-risk operations; HalfOpen admits a
/// limited number of probe operations (counted via `require_closed`).
pub fn require_not_paused_for_high_risk(e: &Env) -> Result<(), ErrorCode> {
    require_closed(e)
}

/// Issue #1898: guard for the bet path. Rejects while the breaker is Open (or
/// Paused) and counts HalfOpen probes so the breaker can transition correctly.
/// Call this at the start of the bet flow, before any state mutation.
pub fn require_can_bet(e: &Env) -> Result<(), ErrorCode> {
    require_closed(e)
}

/// Issue #1898: guard for the claim path. Rejects while the breaker is Open (or
/// Paused) and counts HalfOpen probes so the breaker can transition correctly.
/// Call this at the start of the claim flow, before any state mutation.
pub fn require_can_claim(e: &Env) -> Result<(), ErrorCode> {
    require_closed(e)
}

/// Governance: update the circuit breaker threshold (admin only).
/// Issue #1544: emits a standardized event (old/new) so off-chain monitoring
/// can observe threshold changes, matching other governance config setters.
/// Issue #1543: rejects non-positive values — the threshold represents a max
/// loss in stroops, so zero or negative values are not meaningful and would
/// either permanently trip or permanently disable loss-based protection.
pub fn set_threshold(e: &Env, threshold: i128) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    if threshold <= 0 {
        return Err(ErrorCode::InvalidAmount);
    }
    let old_threshold = get_threshold(e);
    e.storage()
        .instance()
        .set(&ConfigKey::CircuitBreakerThreshold, &threshold);
    crate::modules::events::emit_circuit_breaker_threshold_set(e, old_threshold, threshold);
    Ok(())
}

/// Query the current circuit breaker threshold.
pub fn get_threshold(e: &Env) -> i128 {
    e.storage()
        .instance()
        .get(&ConfigKey::CircuitBreakerThreshold)
        .unwrap_or(DEFAULT_CIRCUIT_BREAKER_THRESHOLD)
}

#[cfg(test)]
mod threshold_tests {
    use super::{get_threshold, set_threshold, DEFAULT_CIRCUIT_BREAKER_THRESHOLD};
    use crate::modules::admin;
    use crate::types::ConfigKey;
    use soroban_sdk::{testutils::Address as _, testutils::Events as _, Address, Env, IntoVal, Symbol};

    fn setup_admin(e: &Env) -> Address {
        let admin = Address::generate(e);
        admin::set_admin(e, admin.clone());
        admin
    }

    #[test]
    fn default_threshold_returned_when_not_set() {
        let e = Env::default();
        assert_eq!(get_threshold(&e), DEFAULT_CIRCUIT_BREAKER_THRESHOLD);
    }

    #[test]
    fn admin_can_update_threshold() {
        let e = Env::default();
        e.mock_all_auths();
        setup_admin(&e);
        set_threshold(&e, 500_000_000).unwrap();
        assert_eq!(get_threshold(&e), 500_000_000);
    }

    #[test]
    fn threshold_stored_in_instance_storage() {
        let e = Env::default();
        e.mock_all_auths();
        setup_admin(&e);
        set_threshold(&e, 42).unwrap();
        let stored: Option<i128> = e
            .storage()
            .instance()
            .get(&ConfigKey::CircuitBreakerThreshold);
        assert_eq!(stored, Some(42));
    }

    #[test]
    fn set_threshold_emits_event_with_old_and_new_values() {
        let e = Env::default();
        e.mock_all_auths();
        setup_admin(&e);

        set_threshold(&e, 500_000_000).unwrap();

        let expected_topics = (
            Symbol::new(&e, "circuit_breaker"),
            Symbol::new(&e, "threshold_set"),
        );
        let expected_data = (DEFAULT_CIRCUIT_BREAKER_THRESHOLD, 500_000_000i128);

        let events = e.events().all();
        let found = events.iter().any(|(_, topics, data)| {
            topics == expected_topics.clone().into_val(&e) && data == expected_data.into_val(&e)
        });
        assert!(found, "expected circuit breaker threshold_set event");
    }

    // ── Issue #1543: non-positive threshold rejection ──────────────────────────

    /// Zero is not a valid threshold — it would trip the circuit breaker on
    #[test]
    fn zero_threshold_rejected() {
        let e = Env::default();
        e.mock_all_auths();
        setup_admin(&e);
        assert!(set_threshold(&e, 0).is_err());
    }

    #[test]
    fn negative_threshold_rejected() {
        let e = Env::default();
        e.mock_all_auths();
        setup_admin(&e);
        assert!(set_threshold(&e, -1).is_err());
    }

    // ── Issue #1898: Open/HalfOpen enforcement on bet and claim paths ──────────

    use super::{require_can_bet, require_can_claim, set_state, DataKey};
    use crate::types::CircuitBreakerState;

    fn trip_open(e: &Env) {
        e.mock_all_auths();
        setup_admin(e);
        set_state(e, CircuitBreakerState::Open).unwrap();
    }

    #[test]
    fn bet_rejected_while_open() {
        let e = Env::default();
        trip_open(&e);
        assert!(require_can_bet(&e).is_err());
    }

    #[test]
    fn claim_rejected_while_open() {
        let e = Env::default();
        trip_open(&e);
        assert!(require_can_claim(&e).is_err());
    }

    #[test]
    fn bet_counts_half_open_probe() {
        let e = Env::default();
        e.mock_all_auths();
        setup_admin(&e);
        set_state(&e, CircuitBreakerState::HalfOpen).unwrap();
        require_can_bet(&e).unwrap();
        let ops: u32 = e.storage().instance().get(&DataKey::HalfOpenOps).unwrap_or(0);
        assert_eq!(ops, 1);
    }
}

    #[test]
    fn claim_counts_half_open_probe() {
        let e = Env::default();
        e.mock_all_auths();
        setup_admin(&e);
        set_state(&e, CircuitBreakerState::HalfOpen).unwrap();
        require_can_claim(&e).unwrap();
        let ops: u32 = e.storage().instance().get(&DataKey::HalfOpenOps).unwrap_or(0);
        assert_eq!(ops, 1);
    }
}
