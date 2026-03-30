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
