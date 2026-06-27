use crate::errors::ErrorCode;
use crate::types::{ConfigKey, GOV_TTL_HIGH_THRESHOLD, GOV_TTL_LOW_THRESHOLD};
use soroban_sdk::{Address, Env};

fn bump_gov_ttl(e: &Env, key: &ConfigKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, GOV_TTL_LOW_THRESHOLD, GOV_TTL_HIGH_THRESHOLD);
}

pub fn set_admin(e: &Env, admin: Address) {
    e.storage().persistent().set(&ConfigKey::Admin, &admin);
    bump_gov_ttl(e, &ConfigKey::Admin);
}

pub fn get_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::Admin)
}

/// Require master Admin role - reserved for structural changes (upgrades, role assignments)
pub fn require_admin(e: &Env) -> Result<(), ErrorCode> {
    let admin: Address = get_admin(e).ok_or(ErrorCode::NotAuthorized)?;
    admin.require_auth();
    Ok(())
}

/// Step 1: current admin proposes a new owner. The new owner must call `accept_admin` to complete.
pub fn propose_admin(e: &Env, new_admin: Address) -> Result<(), ErrorCode> {
    require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::PendingAdmin, &new_admin);
    bump_gov_ttl(e, &ConfigKey::PendingAdmin);
    Ok(())
}

/// Step 2: the pending admin accepts ownership, completing the transfer.
pub fn accept_admin(e: &Env, caller: Address) -> Result<(), ErrorCode> {
    caller.require_auth();
    let pending: Address = e
        .storage()
        .persistent()
        .get(&ConfigKey::PendingAdmin)
        .ok_or(ErrorCode::PendingTransferNotFound)?;
    if pending != caller {
        return Err(ErrorCode::NotPendingOwner);
    }
    set_admin(e, pending);
    e.storage().persistent().remove(&ConfigKey::PendingAdmin);
    Ok(())
}

/// Cancel a pending ownership transfer (current admin only).
pub fn cancel_admin_transfer(e: &Env) -> Result<(), ErrorCode> {
    require_admin(e)?;
    if !e.storage().persistent().has(&ConfigKey::PendingAdmin) {
        return Err(ErrorCode::PendingTransferNotFound);
    }
    e.storage().persistent().remove(&ConfigKey::PendingAdmin);
    Ok(())
}

pub fn set_guardian(e: &Env, guardian: Address) -> Result<(), ErrorCode> {
    require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::GuardianAccount, &guardian);
    bump_gov_ttl(e, &ConfigKey::GuardianAccount);
    Ok(())
}

pub fn get_guardian(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::GuardianAccount)
}

pub fn require_guardian(e: &Env) -> Result<(), ErrorCode> {
    let guardian: Address = get_guardian(e).ok_or(ErrorCode::NotAuthorized)?;
    guardian.require_auth();
    Ok(())
}

pub fn set_governance_token(e: &Env, token: Address) -> Result<(), ErrorCode> {
    require_admin(e)?;
    e.storage()
        .instance()
        .set(&ConfigKey::GovernanceToken, &token);
    Ok(())
}

pub fn set_fee_admin(e: &Env, fee_admin: Address) -> Result<(), ErrorCode> {
    require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::GuardianAccount, &fee_admin);
    bump_gov_ttl(e, &ConfigKey::GuardianAccount);
    Ok(())
}

pub fn get_fee_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::GuardianAccount)
}

#[cfg(test)]
mod ownership_transfer_tests {
    use super::{accept_admin, cancel_admin_transfer, get_admin, propose_admin, set_admin};
    use crate::errors::ErrorCode;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn successful_two_step_transfer() {
        let e = Env::default();
        e.mock_all_auths();
        let owner = Address::generate(&e);
        let new_owner = Address::generate(&e);
        set_admin(&e, owner.clone());

        propose_admin(&e, new_owner.clone()).unwrap();
        accept_admin(&e, new_owner.clone()).unwrap();

        assert_eq!(get_admin(&e), Some(new_owner));
    }

    #[test]
    fn wrong_address_cannot_accept() {
        let e = Env::default();
        e.mock_all_auths();
        let owner = Address::generate(&e);
        let new_owner = Address::generate(&e);
        let attacker = Address::generate(&e);
        set_admin(&e, owner.clone());

        propose_admin(&e, new_owner.clone()).unwrap();
        let err = accept_admin(&e, attacker).unwrap_err();
        assert_eq!(err, ErrorCode::NotPendingOwner);
        // Original owner unchanged
        assert_eq!(get_admin(&e), Some(owner));
    }

    #[test]
    fn admin_can_cancel_pending_transfer() {
        let e = Env::default();
        e.mock_all_auths();
        let owner = Address::generate(&e);
        let new_owner = Address::generate(&e);
        set_admin(&e, owner.clone());

        propose_admin(&e, new_owner).unwrap();
        cancel_admin_transfer(&e).unwrap();

        // Accepting after cancellation should fail
        let err = cancel_admin_transfer(&e).unwrap_err();
        assert_eq!(err, ErrorCode::PendingTransferNotFound);
        assert_eq!(get_admin(&e), Some(owner));
    }

    #[test]
    fn accept_without_proposal_fails() {
        let e = Env::default();
        e.mock_all_auths();
        let owner = Address::generate(&e);
        set_admin(&e, owner);
        let caller = Address::generate(&e);
        let err = accept_admin(&e, caller).unwrap_err();
        assert_eq!(err, ErrorCode::PendingTransferNotFound);
    }
}
