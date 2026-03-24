use crate::errors::ErrorCode;
use crate::types::{
    ConfigKey, Guardian, PendingUpgrade, MAJORITY_THRESHOLD_PERCENT, TIMELOCK_DURATION,
    GOV_TTL_LOW_THRESHOLD, GOV_TTL_HIGH_THRESHOLD,
};
use soroban_sdk::{Address, Env, String, Vec};

/// Extend TTL for a governance key so it never expires during long inactivity.
/// Called after every write to a governance storage slot.
fn bump_gov_ttl(e: &Env, key: &ConfigKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, GOV_TTL_LOW_THRESHOLD, GOV_TTL_HIGH_THRESHOLD);
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

/// Remove a guardian from the set. Only callable by admin.
pub fn remove_guardian(e: &Env, address: Address) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;

    let guardians = get_guardians(e);
    let mut new_guardians: Vec<Guardian> = Vec::new(e);

    let mut found = false;
    for g in guardians.iter() {
        if g.address != address {
            new_guardians.push_back(g.clone());
        } else {
            found = true;
        }
    }

    if !found {
        return Err(ErrorCode::GuardianNotSet);
    }

    e.storage()
        .persistent()
        .set(&ConfigKey::GuardianSet, &new_guardians);
    bump_gov_ttl(e, &ConfigKey::GuardianSet);
    Ok(())
}

/// Initiate a contract upgrade. Requires admin authorization.
/// Starts a 48-hour timelock and requires majority vote to execute.
pub fn initiate_upgrade(e: &Env, wasm_hash: String) -> Result<(), ErrorCode> {
    crate::modules::admin::require_admin(e)?;

    // Validate WASM hash is not empty
    if wasm_hash.is_empty() {
        return Err(ErrorCode::InvalidWasmHash);
    }

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
    bump_gov_ttl(e, &ConfigKey::PendingUpgrade);
    Ok(())
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

    e.storage()
        .persistent()
        .set(&ConfigKey::PendingUpgrade, &pending_upgrade);
    bump_gov_ttl(e, &ConfigKey::PendingUpgrade);
    Ok(true)
}

/// Check if 48-hour timelock has passed.
pub fn is_timelock_satisfied(e: &Env) -> Result<bool, ErrorCode> {
    let pending_upgrade = get_pending_upgrade(e).ok_or(ErrorCode::UpgradeNotInitiated)?;
    let current_time = e.ledger().timestamp();
    let elapsed = current_time.saturating_sub(pending_upgrade.initiated_at);
    Ok(elapsed >= TIMELOCK_DURATION)
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
/// This does NOT directly call update_current_contract_wasm (that's a host function).
/// Instead, it validates conditions and clears the pending upgrade.
/// The caller is responsible for invoking the host function.
pub fn execute_upgrade(e: &Env) -> Result<String, ErrorCode> {
    // Verify timelock has passed
    if !is_timelock_satisfied(e)? {
        return Err(ErrorCode::TimelockActive);
    }

    let pending_upgrade = get_pending_upgrade(e).ok_or(ErrorCode::UpgradeNotInitiated)?;

    // Verify majority vote
    if !is_majority_met(e, &pending_upgrade) {
        return Err(ErrorCode::InsufficientVotes);
    }

    let wasm_hash = pending_upgrade.wasm_hash.clone();

    // Clear pending upgrade
    e.storage().persistent().remove(&ConfigKey::PendingUpgrade);

    Ok(wasm_hash)
}

/// Get vote statistics for the pending upgrade.
pub fn get_upgrade_votes(e: &Env) -> Result<(u32, u32), ErrorCode> {
    let pending_upgrade = get_pending_upgrade(e).ok_or(ErrorCode::UpgradeNotInitiated)?;
    Ok((
        pending_upgrade.votes_for.len() as u32,
        pending_upgrade.votes_against.len() as u32,
    ))
}
