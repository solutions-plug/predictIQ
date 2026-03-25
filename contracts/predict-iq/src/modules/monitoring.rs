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
        .persistent()
        .get(&DataKey::ErrorCount)
        .unwrap_or(0);
    count += 1;
    e.storage().persistent().set(&DataKey::ErrorCount, &count);

    if count > 10 {
        e.storage().persistent().set(
            &crate::types::ConfigKey::CircuitBreakerState,
            &CircuitBreakerState::Open,
        );

        e.events()
            .publish((symbol_short!("cb_auto"),), count);
    }
}

/// Issue #44: Emit MonitorReset event so devops can track resets on-chain.
pub fn reset_monitoring(e: &Env) {
    e.storage().persistent().set(&DataKey::ErrorCount, &0u32);

    e.events()
        .publish((symbol_short!("mon_reset"),), 0u32);
}
