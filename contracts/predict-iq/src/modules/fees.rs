use crate::errors::ErrorCode;
use crate::modules::admin;
use crate::types::{ConfigKey, MarketTier, GOV_TTL_LOW_THRESHOLD, GOV_TTL_HIGH_THRESHOLD};
use soroban_sdk::{contracttype, symbol_short, Address, Env, Symbol};

#[contracttype]
pub enum DataKey {
    TotalFeesCollected,
    FeeRevenue(Address),
    /// Issue #1: Key is now (referrer, token) to prevent cross-asset mixing.
    ReferrerBalance(Address, Address),
}

fn bump_config_ttl(e: &Env, key: &ConfigKey) {
    e.storage()
        .persistent()
        .extend_ttl(key, GOV_TTL_LOW_THRESHOLD, GOV_TTL_HIGH_THRESHOLD);
}

pub fn get_base_fee(e: &Env) -> i128 {
    e.storage()
        .persistent()
        .get(&ConfigKey::BaseFee)
        .unwrap_or(0)
}

pub fn set_base_fee(e: &Env, amount: i128) -> Result<(), ErrorCode> {
    admin::require_admin(e)?;
    e.storage().persistent().set(&ConfigKey::BaseFee, &amount);
    bump_config_ttl(e, &ConfigKey::BaseFee);
    Ok(())
}

pub fn calculate_fee(e: &Env, amount: i128) -> i128 {
    let base_fee = get_base_fee(e);
    (amount * base_fee) / 10000
}

/// Issue #39: Multiply first, then divide to avoid precision loss on small amounts.
pub fn calculate_tiered_fee(e: &Env, amount: i128, tier: &MarketTier) -> i128 {
    let base_fee = get_base_fee(e);

    let adjusted_fee = match tier {
        MarketTier::Basic => base_fee,
        MarketTier::Pro => (base_fee * 75) / 100,
        MarketTier::Institutional => (base_fee * 50) / 100,
    };

    (amount * adjusted_fee) / 10000
}

pub fn collect_fee(e: &Env, token: Address, amount: i128) {
    let key = DataKey::FeeRevenue(token.clone());
    let mut total: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    total += amount;
    e.storage().persistent().set(&key, &total);

    let mut overall: i128 = e
        .storage()
        .persistent()
        .get(&DataKey::TotalFeesCollected)
        .unwrap_or(0);
    overall += amount;
    e.storage()
        .persistent()
        .set(&DataKey::TotalFeesCollected, &overall);

    e.events().publish((symbol_short!("fee_colct"),), amount);
}

pub fn get_revenue(e: &Env, token: Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::FeeRevenue(token))
        .unwrap_or(0)
}

/// Issue #26: Allow FeeAdmin/Admin to withdraw accumulated protocol fees.
pub fn withdraw_protocol_fees(
    e: &Env,
    token: &Address,
    recipient: &Address,
) -> Result<i128, ErrorCode> {
    // Allow either admin or fee_admin to withdraw
    let is_admin = admin::get_admin(e)
        .map(|a| {
            a.try_require_auth().is_ok()
        })
        .unwrap_or(false);
    let is_fee_admin = admin::get_fee_admin(e)
        .map(|a| {
            a.try_require_auth().is_ok()
        })
        .unwrap_or(false);

    if !is_admin && !is_fee_admin {
        return Err(ErrorCode::NotAuthorized);
    }

    let key = DataKey::FeeRevenue(token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);

    if balance == 0 {
        return Err(ErrorCode::InsufficientBalance);
    }

    e.storage().persistent().set(&key, &0i128);

    let client = soroban_sdk::token::Client::new(e, token);
    client.transfer(&e.current_contract_address(), recipient, &balance);

    e.events()
        .publish((Symbol::new(e, "fees_withdrawn"), recipient), balance);

    Ok(balance)
}

/// Issue #1: Referral reward keyed by (referrer, token) to prevent cross-asset mixing.
pub fn add_referral_reward(e: &Env, referrer: &Address, token: &Address, fee_amount: i128) {
    let reward = (fee_amount * 10) / 100;
    let key = DataKey::ReferrerBalance(referrer.clone(), token.clone());
    let mut balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    balance += reward;
    e.storage().persistent().set(&key, &balance);

    e.events()
        .publish((Symbol::new(e, "referral_reward"), referrer), reward);
}

/// Issue #1: Claim referral rewards for a specific token only.
pub fn claim_referral_rewards(
    e: &Env,
    address: &Address,
    token: &Address,
) -> Result<i128, ErrorCode> {
    address.require_auth();

    let key = DataKey::ReferrerBalance(address.clone(), token.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);

    if balance == 0 {
        return Err(ErrorCode::InsufficientBalance);
    }

    e.storage().persistent().set(&key, &0i128);

    let client = soroban_sdk::token::Client::new(e, token);
    client.transfer(&e.current_contract_address(), address, &balance);

    e.events()
        .publish((Symbol::new(e, "referral_claimed"), address), balance);

    Ok(balance)
}
