use crate::errors::ErrorCode;
use crate::types::{
    ConfigKey, Guardian, PendingUpgrade, MAJORITY_THRESHOLD_PERCENT, TIMELOCK_DURATION,
    TIMELOCK_MAX_SECONDS, TIMELOCK_MIN_SECONDS, TTL_HIGH_THRESHOLD, TTL_LOW_THRESHOLD,
    UPGRADE_COOLDOWN_DURATION,
};
use soroban_sdk::{Address, BytesN, Env, Vec};

/// Extend TTL for a governance key so it never expires during long inactivity.
/// Called after every write to a governance storage slot.
fn bump_gov_ttl(e: &Env, key: &ConfigKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, TTL_LOW_THRESHOLD, TTL_HIGH_THRESHOLD);
}

/// Initialize the GuardianSet with a list of guardians and their voting power.
/// Only callable by admin during contract initialization.
pub fn initialize_guardians(e: &Env, guardians: Vec<Guardian>) -> Result<(), ErrorCode> {
    if e.storage().persistent().has(&ConfigKey::GuardianSet) {
        return Err(ErrorCode::AlreadyInitialized);
    }

    if guardians.is_empty() {
        return Err(ErrorCode::NotAuthorized);
    }

    // Issue #19: None of the initial guardians may be the Admin.
    if let Some(admin) = crate::modules::admin::get_admin(e) {
        for g in guardians.iter() {
            if g.address == admin {
                return Err(ErrorCode::NotAuthorized);
            }
        }
    }

    e.storage()
        .persistent()
        .set(&ConfigKey::GuardianSet, &guardians);
    bump_gov_ttl(e, &ConfigKey::GuardianSet);
    Ok(())
}

/// Get the current GuardianSet.
pub fn get_guardians(e: &Env) -> Vec<Guardian> {
    e.storage()
        .persistent()
        .get(&ConfigKey::GuardianSet)
        .unwrap_or_else(|| Vec::new(&e))
}

/// Add a guardian to the set. Only callable by admin.
pub fn add_guardian(e: &Env, guardian: Guardian) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;

    // Issue #19: Admin must not be in the Guardian set — enforces separation of powers.
    if let Some(admin) = crate::modules::admin::get_admin(e) {
        if guardian.address == admin {
            return Err(ErrorCode::NotAuthorized);
        }
    }

    let mut guardians = get_guardians(e);

    // Check if guardian already exists
    for g in guardians.iter() {
        if g.address == guardian.address {
            return Err(ErrorCode::NotAuthorized);
        }
    }

    guardians.push_back(guardian);
    e.storage()
        .persistent()
        .set(&ConfigKey::GuardianSet, &guardians);
    bump_gov_ttl(e, &ConfigKey::GuardianSet);
    Ok(())
}

/// Remove a guardian from the set. Requires majority consensus from other guardians.
pub fn remove_guardian(e: &Env, address: Address) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;

    let guardians = get_guardians(e);

    // Check if guardian exists
    let mut found = false;
    for g in guardians.iter() {
        if g.address == address {
            found = true;
            break;
        }
    }

    if !found {
        return Err(ErrorCode::GuardianNotSet);
    }

    // Initiate removal proposal
    let pending_removal = crate::types::PendingGuardianRemoval {
        target_guardian: address.clone(),
        initiated_at: e.ledger().timestamp(),
        votes_for: Vec::new(e),
    };

    e.storage()
        .persistent()
        .set(&ConfigKey::PendingGuardianRemoval, &pending_removal);
    e.storage()
        .persistent()
        .remove(&ConfigKey::PendingGuardianRemovalPassedAt);
    bump_gov_ttl(e, &ConfigKey::PendingGuardianRemoval);
    Ok(())
}

/// Vote on a pending guardian removal. Requires majority of other guardians.
pub fn vote_on_guardian_removal(e: &Env, voter: Address, approve: bool) -> Result<(), ErrorCode> {
    let guardians = get_guardians(e);

    // Verify voter is a guardian
    let mut voter_is_guardian = false;
    for g in guardians.iter() {
        if g.address == voter {
            voter_is_guardian = true;
            break;
        }
    }

    if !voter_is_guardian {
        return Err(ErrorCode::NotAuthorized);
    }

    let mut pending_removal = e
        .storage()
        .persistent()
        .get::<_, crate::types::PendingGuardianRemoval>(&ConfigKey::PendingGuardianRemoval)
        .ok_or(ErrorCode::GuardianNotSet)?;

    // Check if voter already voted
    for v in pending_removal.votes_for.iter() {
        if v == voter {
            return Err(ErrorCode::AlreadyVotedOnUpgrade);
        }
    }

    if approve {
        pending_removal.votes_for.push_back(voter);
    }

    // Calculate if majority reached (excluding target guardian)
    let other_guardians_count = guardians.len() as u32 - 1;
    let votes_needed = (other_guardians_count * MAJORITY_THRESHOLD_PERCENT) / 100 + 1;

    if pending_removal.votes_for.len() as u32 >= votes_needed
        && get_guardian_removal_passed_at(e).is_none()
    {
        set_guardian_removal_passed_at(e);
    }

    e.storage()
        .persistent()
        .set(&ConfigKey::PendingGuardianRemoval, &pending_removal);
    bump_gov_ttl(e, &ConfigKey::PendingGuardianRemoval);

    Ok(())
}

/// Execute a passed guardian removal after the configured governance timelock.
pub fn execute_guardian_removal(e: &Env) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;

    let pending_removal = e
        .storage()
        .persistent()
        .get::<_, crate::types::PendingGuardianRemoval>(&ConfigKey::PendingGuardianRemoval)
        .ok_or(ErrorCode::GuardianNotSet)?;

    let passed_at = get_guardian_removal_passed_at(e).ok_or(ErrorCode::InsufficientVotes)?;
    let current_time = e.ledger().timestamp();
    let elapsed = current_time.saturating_sub(passed_at);
    if elapsed < get_timelock_duration(e) {
        return Err(ErrorCode::TimelockActive);
    }

    let guardians = get_guardians(e);
    let mut found = false;
    let mut new_guardians: Vec<Guardian> = Vec::new(e);
    for g in guardians.iter() {
        if g.address == pending_removal.target_guardian {
            found = true;
        } else {
            new_guardians.push_back(g.clone());
        }
    }

    if !found {
        return Err(ErrorCode::GuardianNotSet);
    }

    e.storage()
        .persistent()
        .set(&ConfigKey::GuardianSet, &new_guardians);
    bump_gov_ttl(e, &ConfigKey::GuardianSet);

    e.storage()
        .persistent()
        .remove(&ConfigKey::PendingGuardianRemoval);
    e.storage()
        .persistent()
        .remove(&ConfigKey::PendingGuardianRemovalPassedAt);

    Ok(())
}

fn get_guardian_removal_passed_at(e: &Env) -> Option<u64> {
    e.storage()
        .persistent()
        .get(&ConfigKey::PendingGuardianRemovalPassedAt)
}

fn set_guardian_removal_passed_at(e: &Env) {
    e.storage().persistent().set(
        &ConfigKey::PendingGuardianRemovalPassedAt,
        &e.ledger().timestamp(),
    );
    bump_gov_ttl(e, &ConfigKey::PendingGuardianRemovalPassedAt);
}

/// Initiate a contract upgrade. Requires admin authorization.
/// Creates a pending upgrade; the timelock starts when majority approval is reached.
pub fn initiate_upgrade(e: &Env, wasm_hash: BytesN<32>) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;

    require_no_upgrade_collision(e, &wasm_hash)?;

    // Check if an upgrade is already pending
    if e.storage().persistent().has(&ConfigKey::PendingUpgrade) {
        return Err(ErrorCode::NotAuthorized);
    }

    let current_time = e.ledger().timestamp();
    let empty_votes: Vec<Address> = Vec::new(e);

    let pending_upgrade = PendingUpgrade {
        wasm_hash,
        initiated_at: current_time,
        votes_for: empty_votes.clone(),
        votes_against: empty_votes,
    };

    e.storage()
        .persistent()
        .set(&ConfigKey::PendingUpgrade, &pending_upgrade);
    e.storage()
        .persistent()
        .remove(&ConfigKey::PendingUpgradePassedAt);
    bump_gov_ttl(e, &ConfigKey::PendingUpgrade);

    let admin = crate::modules::admin::get_admin(e).unwrap_or(e.current_contract_address());
    crate::modules::events::emit_upgrade_initiated(e, admin, wasm_hash);

    Ok(())
}

fn require_no_upgrade_collision(e: &Env, wasm_hash: &BytesN<32>) -> Result<(), ErrorCode> {
    if let Some(pending_upgrade) = get_pending_upgrade(e) {
        if pending_upgrade.wasm_hash == *wasm_hash {
            return Err(ErrorCode::UpgradeAlreadyPending);
        }
    }

    if let Some(rejected_at) = get_upgrade_rejected_at(e, wasm_hash) {
        let current_time = e.ledger().timestamp();
        let elapsed = current_time.saturating_sub(rejected_at);
        if elapsed <= UPGRADE_COOLDOWN_DURATION {
            return Err(ErrorCode::UpgradeHashInCooldown);
        }
    }

    Ok(())
}

fn get_upgrade_rejected_at(e: &Env, wasm_hash: &BytesN<32>) -> Option<u64> {
    e.storage()
        .persistent()
        .get(&ConfigKey::UpgradeRejectedAt(wasm_hash.clone()))
}

fn set_upgrade_rejected_at(e: &Env, wasm_hash: &BytesN<32>) {
    e.storage().persistent().set(
        &ConfigKey::UpgradeRejectedAt(wasm_hash.clone()),
        &e.ledger().timestamp(),
    );
}

fn clear_upgrade_rejected_at(e: &Env, wasm_hash: &BytesN<32>) {
    e.storage()
        .persistent()
        .remove(&ConfigKey::UpgradeRejectedAt(wasm_hash.clone()));
}

/// Get the currently pending upgrade, if any.
pub fn get_pending_upgrade(e: &Env) -> Option<PendingUpgrade> {
    e.storage().persistent().get(&ConfigKey::PendingUpgrade)
}

/// Vote on the pending upgrade. Guardian must authenticate.
/// Returns true if vote was recorded (not already voted).
pub fn vote_for_upgrade(e: &Env, voter: Address, vote_for: bool) -> Result<bool, ErrorCode> {
    voter.require_auth();

    // Verify voter is a guardian
    let guardians = get_guardians(e);
    let mut is_guardian = false;
    for g in guardians.iter() {
        if g.address == voter {
            is_guardian = true;
            break;
        }
    }

    if !is_guardian {
        return Err(ErrorCode::NotAuthorized);
    }

    // Get pending upgrade
    let mut pending_upgrade = get_pending_upgrade(e).ok_or(ErrorCode::UpgradeNotInitiated)?;

    // Check if voter has already voted
    for v in pending_upgrade.votes_for.iter() {
        if v == voter {
            return Err(ErrorCode::AlreadyVotedOnUpgrade);
        }
    }
    for v in pending_upgrade.votes_against.iter() {
        if v == voter {
            return Err(ErrorCode::AlreadyVotedOnUpgrade);
        }
    }

    // Record vote
    if vote_for {
        pending_upgrade.votes_for.push_back(voter);
    } else {
        pending_upgrade.votes_against.push_back(voter);
    }

    if is_majority_met(e, &pending_upgrade) && get_upgrade_passed_at(e).is_none() {
        set_upgrade_passed_at(e);
    }

    e.storage()
        .persistent()
        .set(&ConfigKey::PendingUpgrade, &pending_upgrade);
    bump_gov_ttl(e, &ConfigKey::PendingUpgrade);

    crate::modules::events::emit_upgrade_voted(e, voter, vote_for);

    Ok(true)
}

/// Issue #13: Get the effective timelock duration (storage override or default constant).
pub fn get_timelock_duration(e: &Env) -> u64 {
    e.storage()
        .persistent()
        .get(&ConfigKey::TimelockDuration)
        .unwrap_or(TIMELOCK_DURATION)
}

/// Issue #13/#690: Allow admin to set a new timelock duration within [24h, 7d].
pub fn set_timelock_duration(e: &Env, seconds: u64) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;
    if seconds < TIMELOCK_MIN_SECONDS || seconds > TIMELOCK_MAX_SECONDS {
        return Err(ErrorCode::InvalidAmount);
    }
    e.storage()
        .persistent()
        .set(&ConfigKey::TimelockDuration, &seconds);
    bump_gov_ttl(e, &ConfigKey::TimelockDuration);
    Ok(())
}

/// Check if the configurable timelock has passed.
pub fn is_timelock_satisfied(e: &Env) -> Result<bool, ErrorCode> {
    let pending_upgrade = get_pending_upgrade(e).ok_or(ErrorCode::UpgradeNotInitiated)?;
    let timelock_started_at = get_upgrade_passed_at(e).unwrap_or(pending_upgrade.initiated_at);
    let current_time = e.ledger().timestamp();
    let elapsed = current_time.saturating_sub(timelock_started_at);
    Ok(elapsed >= get_timelock_duration(e))
}

/// Check if majority vote threshold has been met.
fn is_majority_met(e: &Env, pending_upgrade: &PendingUpgrade) -> bool {
    let guardians = get_guardians(e);

    if guardians.is_empty() {
        return false;
    }

    // Sum total voting power across all guardians
    let mut total_power: u32 = 0;
    for i in 0..guardians.len() {
        total_power += guardians.get(i).unwrap().voting_power;
    }

    if total_power == 0 {
        return false;
    }

    // Sum voting power of guardians who voted for
    let mut power_for: u32 = 0;
    for i in 0..pending_upgrade.votes_for.len() {
        let voter = pending_upgrade.votes_for.get(i).unwrap();
        for j in 0..guardians.len() {
            let g = guardians.get(j).unwrap();
            if g.address == voter {
                power_for += g.voting_power;
                break;
            }
        }
    }

    // Calculate percentage: (power_for / total_power) * 100
    let percentage = (power_for * 100) / total_power;
    percentage >= MAJORITY_THRESHOLD_PERCENT
}

/// Execute the upgrade if timelock is satisfied and majority voted in favor.
/// This directly invokes the Soroban host upgrade function.
pub fn execute_upgrade(e: &Env) -> Result<BytesN<32>, ErrorCode> {
    // Verify timelock has passed
    if !is_timelock_satisfied(e)? {
        return Err(ErrorCode::TimelockActive);
    }

    let pending_upgrade = get_pending_upgrade(e).ok_or(ErrorCode::UpgradeNotInitiated)?;

    // Verify majority vote
    if !is_majority_met(e, &pending_upgrade) {
        set_upgrade_rejected_at(e, &pending_upgrade.wasm_hash);
        e.storage().persistent().remove(&ConfigKey::PendingUpgrade);
        e.storage()
            .persistent()
            .remove(&ConfigKey::PendingUpgradePassedAt);
        crate::modules::events::emit_upgrade_rejected(e, pending_upgrade.wasm_hash);
        return Err(ErrorCode::InsufficientVotes);
    }

    let wasm_hash = pending_upgrade.wasm_hash.clone();

    // Clear pending upgrade
    e.storage().persistent().remove(&ConfigKey::PendingUpgrade);
    e.storage()
        .persistent()
        .remove(&ConfigKey::PendingUpgradePassedAt);
    clear_upgrade_rejected_at(e, &wasm_hash);

    let executor = crate::modules::admin::get_admin(e).unwrap_or(e.current_contract_address());
    crate::modules::events::emit_upgrade_executed(e, executor, wasm_hash.clone());

    // Execute host-level contract code upgrade.
    e.deployer().update_current_contract_wasm(wasm_hash.clone());

    Ok(wasm_hash)
}

fn get_upgrade_passed_at(e: &Env) -> Option<u64> {
    e.storage()
        .persistent()
        .get(&ConfigKey::PendingUpgradePassedAt)
}

fn set_upgrade_passed_at(e: &Env) {
    e.storage()
        .persistent()
        .set(&ConfigKey::PendingUpgradePassedAt, &e.ledger().timestamp());
    bump_gov_ttl(e, &ConfigKey::PendingUpgradePassedAt);
}

/// Get vote statistics for the pending upgrade.
pub fn get_upgrade_votes(e: &Env) -> Result<crate::types::UpgradeVoteStats, ErrorCode> {
    let pending_upgrade = get_pending_upgrade(e).ok_or(ErrorCode::UpgradeNotInitiated)?;
    Ok(crate::types::UpgradeVoteStats {
        votes_for: pending_upgrade.votes_for.len() as u32,
        votes_against: pending_upgrade.votes_against.len() as u32,
    })
}

/// Emergency pause triggered by 2/3 Guardian majority (community panic override)
pub fn emergency_pause(e: &Env, voter: Address) -> Result<(), ErrorCode> {
    voter.require_auth();

    let guardians = get_guardians(e);
    if guardians.is_empty() {
        return Err(ErrorCode::NotAuthorized);
    }

    // Verify voter is a guardian
    let mut voter_power: u32 = 0;
    let mut total_power: u32 = 0;
    for g in guardians.iter() {
        total_power += g.voting_power;
        if g.address == voter {
            voter_power = g.voting_power;
        }
    }

    if voter_power == 0 {
        return Err(ErrorCode::NotAuthorized);
    }

    // Check if this guardian's vote alone meets 2/3 threshold
    let threshold = (total_power * 2) / 3;
    if voter_power < threshold {
        return Err(ErrorCode::InsufficientVotes);
    }

    // Trigger emergency pause
    e.storage().instance().set(
        &ConfigKey::CircuitBreakerState,
        &crate::types::CircuitBreakerState::Paused,
    );
    bump_gov_ttl(e, &ConfigKey::CircuitBreakerState);

    Ok(())
}
