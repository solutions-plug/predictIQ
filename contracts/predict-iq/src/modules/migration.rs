use crate::errors::ErrorCode;
use crate::types::{ConfigKey, Guardian};
use soroban_sdk::{contracttype, Address, Env, Vec};

/// Storage migration context for tracking version changes
#[contracttype]
#[derive(Clone)]
pub struct MigrationContext {
    pub from_version: u32,
    pub to_version: u32,
}

/// A single recorded migration, keyed by its destination version so that
/// history across multiple migrations remains independently queryable.
#[contracttype]
#[derive(Clone)]
pub struct MigrationLogEntry {
    pub from_version: u32,
    pub to_version: u32,
    pub timestamp: u64,
}

/// Snapshot of the storage this module guarantees to restore on rollback.
/// Scope is intentionally limited to the keys `verify_migration_integrity` checks
/// (`ConfigKey::Admin`, `ConfigKey::GuardianSet`).
#[contracttype]
#[derive(Clone)]
struct MigrationBackup {
    admin: Option<Address>,
    guardian_set: Option<Vec<Guardian>>,
}

/// Execute a storage migration with rollback capability.
///
/// Rollback scope: only `ConfigKey::Admin` and `ConfigKey::GuardianSet` -- the keys
/// `verify_migration_integrity` validates -- are snapshotted before `migration_fn`
/// runs and are genuinely restored to their prior values if `migration_fn` errors or
/// post-migration validation fails. Any other storage mutated by `migration_fn` is
/// NOT automatically reverted; migrations that touch state outside these two keys
/// must be manually reversible or safely re-driveable.
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

            // Clean up the backup snapshot now that the migration has succeeded.
            // Without this, every successful migration leaks an orphaned
            // `MigrationBackup` entry in persistent storage forever.
            let backup_key = format!("migration:backup:v{}", from_version);
            e.storage().persistent().remove(&backup_key);

            Ok(())
        }
        Err(err) => {
            // Rollback on failure
            restore_storage_state(e, from_version)?;
            Err(err)
        }
    }
}

/// Snapshot the actual pre-migration values of `ConfigKey::Admin` and
/// `ConfigKey::GuardianSet` (the keys `verify_migration_integrity` checks).
fn backup_storage_state(e: &Env, version: u32) -> Result<(), ErrorCode> {
    let backup_key = format!("migration:backup:v{}", version);
    let backup = MigrationBackup {
        admin: e.storage().persistent().get(&ConfigKey::Admin),
        guardian_set: e.storage().persistent().get(&ConfigKey::GuardianSet),
    };

    e.storage().persistent().set(&backup_key, &backup);

    Ok(())
}

/// Restore `ConfigKey::Admin` and `ConfigKey::GuardianSet` to the values captured
/// by `backup_storage_state`, genuinely reverting them rather than just clearing
/// a marker.
fn restore_storage_state(e: &Env, version: u32) -> Result<(), ErrorCode> {
    let backup_key = format!("migration:backup:v{}", version);

    let backup: MigrationBackup = e
        .storage()
        .persistent()
        .get(&backup_key)
        .ok_or(ErrorCode::NotAuthorized)?;

    match backup.admin {
        Some(admin) => e.storage().persistent().set(&ConfigKey::Admin, &admin),
        None => e.storage().persistent().remove(&ConfigKey::Admin),
    }
    match backup.guardian_set {
        Some(guardians) => e
            .storage()
            .persistent()
            .set(&ConfigKey::GuardianSet, &guardians),
        None => e.storage().persistent().remove(&ConfigKey::GuardianSet),
    }

    e.storage().persistent().remove(&backup_key);
    Ok(())
}

/// Record migration completion.
///
/// Each migration is stored under its own per-version key (`migration:log:v{to_version}`)
/// so that history across multiple migrations is preserved and independently
/// queryable, rather than being overwritten by the latest migration.
fn record_migration(e: &Env, from_version: u32, to_version: u32) -> Result<(), ErrorCode> {
    let timestamp = e.ledger().timestamp();

    let entry = MigrationLogEntry {
        from_version,
        to_version,
        timestamp,
    };

    let log_key = format!("migration:log:v{}", to_version);
    e.storage().persistent().set(&log_key, &entry);

    Ok(())
}

/// Query the migration log entry recorded for a specific destination version.
/// Returns `None` if no migration to that version has been recorded.
pub fn get_migration_log(e: &Env, to_version: u32) -> Option<MigrationLogEntry> {
    let log_key = format!("migration:log:v{}", to_version);
    e.storage().persistent().get(&log_key)
}

/// Verify data integrity after migration
/// Post-migration validation function that checks key invariants.
/// Returns Ok(true) if all invariants pass, Ok(false) if validation fails.
pub fn verify_migration_integrity(e: &Env) -> Result<bool, ErrorCode> {
    // Check critical storage keys exist
    let required_keys = vec![ConfigKey::Admin, ConfigKey::GuardianSet];

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
            sum_outcome_stakes = sum_outcome_stakes
                .checked_add(stake)
                .ok_or(ErrorCode::ArithmeticOverflow)?;
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

/// Reverse a migration to previous version.
///
/// Same rollback scope as `execute_migration`: genuinely restores
/// `ConfigKey::Admin` and `ConfigKey::GuardianSet` from the snapshot taken
/// before the migration ran.
pub fn reverse_migration(e: &Env, from_version: u32, to_version: u32) -> Result<(), ErrorCode> {
    if from_version >= to_version {
        return Err(ErrorCode::NotAuthorized);
    }

    restore_storage_state(e, from_version)?;

    // Clear the per-version migration log entry for this migration only,
    // leaving other versions' history intact.
    let log_key = format!("migration:log:v{}", to_version);
    e.storage().persistent().remove(&log_key);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migration_version_validation() {
        // Version must progress forward
        let result = execute_migration(&soroban_sdk::Env::default(), 2, 1, |_| Ok(()));
        assert!(result.is_err());
    }

    #[test]
    fn test_migration_with_rollback() {
        let env = soroban_sdk::Env::default();

        let result = execute_migration(&env, 1, 2, |_| Err(ErrorCode::NotAuthorized));

        assert!(result.is_err());
    }

    #[test]
    fn test_migration_validation_failure_rolls_back() {
        use soroban_sdk::Address;

        let env = soroban_sdk::Env::default();
        let admin = Address::generate(&env);
        let original_guardians = Vec::from_array(
            &env,
            [Guardian {
                address: Address::generate(&env),
                voting_power: 1,
            }],
        );

        env.storage().persistent().set(&ConfigKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&ConfigKey::GuardianSet, &original_guardians);

        // Migration that removes Admin and overwrites GuardianSet. Removing Admin
        // invalidates state (verify_migration_integrity fails), which must trigger
        // a genuine rollback of both snapshotted keys -- not just marker cleanup.
        let tampered_guardians: Vec<Guardian> = Vec::new(&env);
        let result = execute_migration(&env, 1, 2, |_e| {
            _e.storage()
                .persistent()
                .remove(&ConfigKey::Admin);
            _e.storage()
                .persistent()
                .set(&ConfigKey::GuardianSet, &tampered_guardians);
            Ok(())
        });

        assert_eq!(result, Err(ErrorCode::MigrationValidationError));
        // Both keys must be genuinely restored to their pre-migration values.
        let restored_admin: Address = env
            .storage()
            .persistent()
            .get(&ConfigKey::Admin)
            .expect("admin should be restored");
        assert_eq!(restored_admin, admin);
        assert_eq!(
            env.storage()
                .persistent()
                .get::<ConfigKey, Vec<Guardian>>(&ConfigKey::GuardianSet),
            Some(original_guardians)
        );
    }

    #[test]
    fn test_migration_history_preserved_across_sequential_migrations() {
        use soroban_sdk::Address;

        let env = soroban_sdk::Env::default();
        let admin = Address::generate(&env);
        let guardians = Vec::from_array(
            &env,
            [Guardian {
                address: Address::generate(&env),
                voting_power: 1,
            }],
        );

        env.storage().persistent().set(&ConfigKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&ConfigKey::GuardianSet, &guardians);

        // First migration: v1 -> v2
        execute_migration(&env, 1, 2, |_| Ok(())).unwrap();
        // Second migration: v2 -> v3
        execute_migration(&env, 2, 3, |_| Ok(())).unwrap();

        // Both entries must be independently recoverable.
        let first = get_migration_log(&env, 2).expect("v2 entry missing");
        assert_eq!(first.from_version, 1);
        assert_eq!(first.to_version, 2);

        let second = get_migration_log(&env, 3).expect("v3 entry missing");
        assert_eq!(second.from_version, 2);
        assert_eq!(second.to_version, 3);

        // The earlier entry must not have been overwritten by the later one.
        assert_ne!(first.to_version, second.to_version);

        // The backup snapshot must be cleaned up on the rollback path.
        let backup_key = format!("migration:backup:v{}", 1);
        assert!(!env.storage().persistent().has(&backup_key));
    }

    #[test]
    fn test_successful_migration_removes_backup_snapshot() {
        use soroban_sdk::Address;

        let env = soroban_sdk::Env::default();
        let admin = Address::generate(&env);
        let guardians = Vec::from_array(
            &env,
            [Guardian {
                address: Address::generate(&env),
                voting_power: 1,
            }],
        );

        env.storage().persistent().set(&ConfigKey::Admin, &admin);
        env.storage()
            .persistent()
            .set(&ConfigKey::GuardianSet, &guardians);

        // A migration that leaves the validated invariants intact succeeds.
        let result = execute_migration(&env, 1, 2, |_e| Ok(()));
        assert_eq!(result, Ok(()));

        // The backup snapshot must not be left orphaned in persistent storage.
        let backup_key = format!("migration:backup:v{}", 1);
        assert!(!env.storage().persistent().has(&backup_key));
    }
}
