use crate::errors::ErrorCode;
use crate::modules::admin;
use crate::types::{ConfigKey, MarketTier};
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    TotalFeesCollected,
    FeeRevenue(Address), // token_address -> amount
    ReferrerBalance(Address), // referrer_address -> amount
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
    Ok(())
}

pub fn calculate_fee(e: &Env, amount: i128) -> i128 {
    let base_fee = get_base_fee(e);
    // base_fee is in basis points (1/10000)
    (amount * base_fee) / 10000
}

pub fn calculate_tiered_fee(e: &Env, amount: i128, tier: &MarketTier) -> i128 {
    let base_fee = get_base_fee(e);

    // Apply tier multiplier: Basic=100%, Pro=75%, Institutional=50%
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
    overall += amount; // Simplified overall tracking (assuming normalized units for analytics)
    e.storage()
        .persistent()
        .set(&DataKey::TotalFeesCollected, &overall);

    // Emit standardized fee collection event using soroban_sdk
    use soroban_sdk::symbol_short;
    e.events().publish((symbol_short!("fee_colct"),), amount);
}

pub fn get_revenue(e: &Env, token: Address) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::FeeRevenue(token))
        .unwrap_or(0)
}

pub fn add_referral_reward(e: &Env, referrer: &Address, fee_amount: i128) {
    let reward = (fee_amount * 10) / 100; // 10% of fee
    let key = DataKey::ReferrerBalance(referrer.clone());
    let mut balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    balance += reward;
    e.storage().persistent().set(&key, &balance);
    
    e.events().publish(
        (Symbol::new(e, "referral_reward"), referrer),
        reward,
    );
}

pub fn claim_referral_rewards(e: &Env, address: &Address, token: &Address) -> Result<i128, ErrorCode> {
    address.require_auth();
    
    let key = DataKey::ReferrerBalance(address.clone());
    let balance: i128 = e.storage().persistent().get(&key).unwrap_or(0);
    
    if balance == 0 {
        return Err(ErrorCode::InsufficientBalance);
    }
    
    e.storage().persistent().set(&key, &0);
    
    let client = soroban_sdk::token::Client::new(e, token);
    client.transfer(&e.current_contract_address(), address, &balance);
    
    e.events().publish(
        (Symbol::new(e, "referral_claimed"), address),
        balance,
    );
    
    Ok(balance)
}
