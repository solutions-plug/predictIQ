/// Issue #38: All monitoring state moved to persistent storage so it stays
/// consistent with the circuit breaker state (also persistent).
/// Issue #44: Emit MonitorReset event when counters are cleared.
use crate::types::CircuitBreakerState;
use soroban_sdk::{contracttype, symbol_short, Env};

#[contracttype]
pub enum DataKey {
    ErrorCount,
    LastObservation,
}

pub fn track_error(e: &Env) {
    let mut count: u32 = e
        .storage()
        .instance()
        .get(&DataKey::ErrorCount)
        .unwrap_or(0);
    count += 1;
    e.storage().instance().set(&DataKey::ErrorCount, &count);
    e.storage()
        .instance()
        .set(&DataKey::LastObservation, &e.ledger().timestamp());

    if count > 10 {
        e.storage().instance().set(
            &crate::types::ConfigKey::CircuitBreakerState,
            &crate::types::CircuitBreakerState::Open,
        );

        e.events()
            .publish((symbol_short!("cb_auto"),), count);
    }
}

/// Issue #44: Emit MonitorReset event so devops can track resets on-chain.
pub fn reset_monitoring(e: &Env) {
    let previous_error_count: u32 = e
        .storage()
        .instance()
        .get(&DataKey::ErrorCount)
        .unwrap_or(0);
    let previous_last_observation: u64 = e
        .storage()
        .instance()
        .get(&DataKey::LastObservation)
        .unwrap_or(0);

    e.storage().instance().set(&DataKey::ErrorCount, &0u32);
    e.storage().instance().set(&DataKey::LastObservation, &0u64);

    let resetter = crate::modules::admin::get_admin(e).unwrap_or(e.current_contract_address());
    crate::modules::events::emit_monitoring_state_reset(
        e,
        resetter,
        previous_error_count,
        previous_last_observation,
    );
}

#[cfg(test)]
mod tests {
    use super::{reset_monitoring, track_error, DataKey};
    use soroban_sdk::{testutils::{Events, Ledger}, Env};

    #[test]
    fn reset_monitoring_clears_error_trackers() {
        let e = Env::default();
        e.ledger().set_timestamp(777);

        track_error(&e);
        track_error(&e);

        let before_count: u32 = e.storage().instance().get(&DataKey::ErrorCount).unwrap_or(0);
        let before_obs: u64 = e
            .storage()
            .instance()
            .get(&DataKey::LastObservation)
            .unwrap_or(0);
        assert_eq!(before_count, 2);
        assert_eq!(before_obs, 777);

        reset_monitoring(&e);

        let after_count: u32 = e.storage().instance().get(&DataKey::ErrorCount).unwrap_or(1);
        let after_obs: u64 = e
            .storage()
            .instance()
            .get(&DataKey::LastObservation)
            .unwrap_or(1);

        assert_eq!(after_count, 0);
        assert_eq!(after_obs, 0);
    }

    #[test]
    fn reset_monitoring_emits_event_with_previous_values() {
        let e = Env::default();
        e.ledger().set_timestamp(1234);

        track_error(&e);
        track_error(&e);

        reset_monitoring(&e);

        let events = e.events().all();
        assert!(!events.is_empty());

        let events_debug = format!("{:?}", events);
        assert!(events_debug.contains("mon_reset"));
        assert!(events_debug.contains("2"));
        assert!(events_debug.contains("1234"));
    }
}
