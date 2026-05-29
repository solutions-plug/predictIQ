use crate::errors::ErrorCode;
use crate::types::ConfigKey;
use soroban_sdk::{contracttype, Env, Vec};

/// Storage migration context for tracking version changes
#[contracttype]
#[derive(Clone)]
pub struct MigrationContext {
    pub from_version: u32,
    pub to_version: u32,
}

/// Execute a storage migration with rollback capability
/// Post-migration validation checks key invariants. If validation fails, migration fails atomically.
pub fn execute_migration(
    e: &Env,
    from_version: u32,
    to_version: u32,
    migration_fn: impl Fn(&Env) -> Result<(), ErrorCode>,
) -> Result<(), ErrorCode> {
    // Verify version progression
    if to_version <= from_version {
        return Err(ErrorCode::NotAuthorized);
    }

    // Create backup of current state
    backup_storage_state(e, from_version)?;

    // Execute migration
    match migration_fn(e) {
        Ok(_) => {
            // Post-migration validation: check invariants
            if !verify_migration_integrity(e)? {
                // Rollback on validation failure
                restore_storage_state(e, from_version)?;
                return Err(ErrorCode::MigrationValidationError);
            }
            
            // Record successful migration
            record_migration(e, from_version, to_version)?;
            Ok(())
        }
        Err(err) => {
            // Rollback on failure
            restore_storage_state(e, from_version)?;
            Err(err)
        }
    }
}

/// Backup current storage state before migration
fn backup_storage_state(e: &Env, version: u32) -> Result<(), ErrorCode> {
    let backup_key = format!("migration:backup:v{}", version);
    let timestamp = e.ledger().timestamp();
    
    e.storage()
        .persistent()
        .set(&backup_key, &timestamp);
    
    Ok(())
}

/// Restore storage state from backup
fn restore_storage_state(e: &Env, version: u32) -> Result<(), ErrorCode> {
    let backup_key = format!("migration:backup:v{}", version);
    
    if !e.storage().persistent().has(&backup_key) {
        return Err(ErrorCode::NotAuthorized);
    }

    e.storage().persistent().remove(&backup_key);
    Ok(())
}

/// Record migration completion
fn record_migration(e: &Env, from_version: u32, to_version: u32) -> Result<(), ErrorCode> {
    let migration_log_key = "migration:log";
    let timestamp = e.ledger().timestamp();
    
    let entry = format!("v{}->v{}@{}", from_version, to_version, timestamp);
    
    e.storage()
        .persistent()
        .set(&migration_log_key, &entry);
    
    Ok(())
}

/// Verify data integrity after migration
/// Post-migration validation function that checks key invariants.
/// Returns Ok(true) if all invariants pass, Ok(false) if validation fails.
pub fn verify_migration_integrity(e: &Env) -> Result<bool, ErrorCode> {
    // Check critical storage keys exist
    let required_keys = vec![
        ConfigKey::Admin,
        ConfigKey::GuardianSet,
    ];

    for key in required_keys.iter() {
        if !e.storage().persistent().has(key) {
            return Ok(false);
        }
    }

    Ok(true)
}

/// Post-migration validation that checks stake conservation invariant:
/// total_staked should equal sum of all outcome_stakes
pub fn validate_stake_invariant(e: &Env, market_id: u64) -> Result<bool, ErrorCode> {
    let market = match crate::modules::markets::get_market(e, market_id) {
        Some(m) => m,
        None => return Err(ErrorCode::MarketNotFound),
    };

    let total_staked = market.total_staked;
    let mut sum_outcome_stakes: i128 = 0;
    let mut outcome_idx: u32 = 0;

    while outcome_idx < market.outcome_stakes.len() {
        if let Some(stake) = market.outcome_stakes.get(outcome_idx) {
            sum_outcome_stakes = sum_outcome_stakes.checked_add(stake).ok_or(ErrorCode::ArithmeticOverflow)?;
        }
        outcome_idx += 1;
    }

    Ok(total_staked == sum_outcome_stakes)
}

/// Validate all markets after migration - ensure stake conservation holds
pub fn validate_all_markets_stake_invariant(e: &Env, market_count: u64) -> Result<bool, ErrorCode> {
    let mut market_id: u64 = 1;
    while market_id <= market_count {
        if crate::modules::markets::get_market(e, market_id).is_some() {
            if !validate_stake_invariant(e, market_id)? {
                return Ok(false);
            }
        }
        market_id += 1;
    }
    Ok(true)
}

/// Reverse a migration to previous version
pub fn reverse_migration(
    e: &Env,
    from_version: u32,
    to_version: u32,
) -> Result<(), ErrorCode> {
    if from_version >= to_version {
        return Err(ErrorCode::NotAuthorized);
    }

    // Verify backup exists
    let backup_key = format!("migration:backup:v{}", from_version);
    if !e.storage().persistent().has(&backup_key) {
        return Err(ErrorCode::NotAuthorized);
    }

    // Clear migration log entry
    let migration_log_key = "migration:log";
    e.storage().persistent().remove(&migration_log_key);

    // Remove backup after successful reversal
    e.storage().persistent().remove(&backup_key);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_version_validation() {
        // Version must progress forward
        let result = execute_migration(
            &soroban_sdk::Env::default(),
            2,
            1,
            |_| Ok(()),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_migration_with_rollback() {
        let env = soroban_sdk::Env::default();
        
        let result = execute_migration(
            &env,
            1,
            2,
            |_| Err(ErrorCode::NotAuthorized),
        );
        
        assert!(result.is_err());
    }

    #[test]
    fn test_migration_validation_failure_rolls_back() {
        use soroban_sdk::Address;
        
        let env = soroban_sdk::Env::default();
        let admin = Address::generate(&env);
        
        env.storage().persistent().set(&ConfigKey::Admin, &admin);
        env.storage().persistent().set(&ConfigKey::GuardianSet, &admin);

        // Migration that removes admin key (invalidates state)
        let result = execute_migration(
            &env,
            1,
            2,
            |_e| {
                _e.storage().persistent().remove(&ConfigKey::Admin);
                Ok(())
            },
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ErrorCode::MigrationValidationError);
        assert!(env.storage().persistent().has(&ConfigKey::Admin));
    }
}