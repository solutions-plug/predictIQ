use crate::types::CircuitBreakerState;
use soroban_sdk::{contracttype, Env};

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

    if count > 10 {
        // Threshold for automatic trigger
        // Automatically open the circuit breaker
        e.storage().persistent().set(
            &crate::types::ConfigKey::CircuitBreakerState,
            &CircuitBreakerState::Open,
        );

        // Emit standardized circuit breaker event using soroban_sdk
        use soroban_sdk::symbol_short;
        e.events().publish((symbol_short!("cb_auto"),), count);
    }
}

pub fn reset_monitoring(e: &Env) {
    e.storage().instance().set(&DataKey::ErrorCount, &0u32);
}
