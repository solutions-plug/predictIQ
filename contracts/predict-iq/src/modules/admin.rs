use crate::errors::ErrorCode;
use crate::types::ConfigKey;
use soroban_sdk::{Address, Env};

pub fn set_admin(e: &Env, admin: Address) {
    e.storage().persistent().set(&ConfigKey::Admin, &admin);
}

pub fn get_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::Admin)
}

pub fn require_admin(e: &Env) -> Result<(), ErrorCode> {
    let admin: Address = get_admin(e).ok_or(ErrorCode::AdminNotSet)?;
    admin.require_auth();
    Ok(())
}

pub fn set_market_admin(e: &Env, admin: Address) -> Result<(), ErrorCode> {
    require_admin(e)?;
    e.storage()
        .persistent()
        .set(&ConfigKey::MarketAdmin, &admin);
    Ok(())
}

pub fn get_market_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::MarketAdmin)
}

pub fn set_fee_admin(e: &Env, admin: Address) -> Result<(), ErrorCode> {
    require_admin(e)?;
    e.storage().persistent().set(&ConfigKey::FeeAdmin, &admin);
    Ok(())
}

pub fn get_fee_admin(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::FeeAdmin)
}

pub fn set_guardian(e: &Env, guardian: Address) -> Result<(), ErrorCode> {
    require_admin(e)?;
    e.storage().persistent().set(&ConfigKey::GuardianAccount, &guardian);
    Ok(())
}

pub fn get_guardian(e: &Env) -> Option<Address> {
    e.storage().persistent().get(&ConfigKey::GuardianAccount)
}

pub fn require_guardian(e: &Env) -> Result<(), ErrorCode> {
    let guardian: Address = get_guardian(e).ok_or(ErrorCode::GuardianNotSet)?;
    guardian.require_auth();
    Ok(())
}
