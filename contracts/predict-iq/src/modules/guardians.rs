use soroban_sdk::{Env, Address, Vec, contracttype};
use crate::errors::ErrorCode;

const TIMELOCK_SECONDS: u64 = 259200; // 72 hours
const REQUIRED_GUARDIANS: u32 = 3;
const TOTAL_GUARDIANS: u32 = 5;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryState {
    pub new_admin: Address,
    pub approvals: Vec<Address>,
    pub initiated_at: u64,
}

#[contracttype]
pub enum DataKey {
    Guardians,
    Recovery,
}

pub fn set_guardians(e: &Env, guardians: Vec<Address>) -> Result<(), ErrorCode> {
    if guardians.len() != TOTAL_GUARDIANS {
        return Err(ErrorCode::InsufficientGuardians);
    }
    e.storage().persistent().set(&DataKey::Guardians, &guardians);
    Ok(())
}

pub fn get_guardians(e: &Env) -> Option<Vec<Address>> {
    e.storage().persistent().get(&DataKey::Guardians)
}

pub fn sign_reset_admin(e: &Env, guardian: Address, new_admin: Address) -> Result<(), ErrorCode> {
    guardian.require_auth();
    
    let guardians = get_guardians(e).ok_or(ErrorCode::InsufficientGuardians)?;
    if !guardians.contains(&guardian) {
        return Err(ErrorCode::NotAuthorized);
    }

    let mut recovery: RecoveryState = e.storage().persistent().get(&DataKey::Recovery).unwrap_or(RecoveryState {
        new_admin: new_admin.clone(),
        approvals: Vec::new(e),
        initiated_at: e.ledger().timestamp(),
    });

    if recovery.new_admin != new_admin {
        return Err(ErrorCode::RecoveryAlreadyActive);
    }

    if !recovery.approvals.contains(&guardian) {
        recovery.approvals.push_back(guardian);
    }

    e.storage().persistent().set(&DataKey::Recovery, &recovery);
    Ok(())
}

pub fn get_recovery_state(e: &Env) -> Option<RecoveryState> {
    e.storage().persistent().get(&DataKey::Recovery)
}

pub fn is_recovery_active(e: &Env) -> bool {
    if let Some(recovery) = get_recovery_state(e) {
        recovery.approvals.len() >= REQUIRED_GUARDIANS
    } else {
        false
    }
}

pub fn finalize_recovery(e: &Env) -> Result<Address, ErrorCode> {
    let recovery = get_recovery_state(e).ok_or(ErrorCode::RecoveryNotActive)?;
    
    if recovery.approvals.len() < REQUIRED_GUARDIANS {
        return Err(ErrorCode::InsufficientGuardians);
    }

    let elapsed = e.ledger().timestamp() - recovery.initiated_at;
    if elapsed < TIMELOCK_SECONDS {
        return Err(ErrorCode::RecoveryTimelockNotExpired);
    }

    e.storage().persistent().remove(&DataKey::Recovery);
    crate::modules::admin::set_admin(e, recovery.new_admin.clone());
    
    Ok(recovery.new_admin)
}
