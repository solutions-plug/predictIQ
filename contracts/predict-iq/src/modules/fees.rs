use soroban_sdk::{Env, Address, contracttype};
use crate::types::{ConfigKey, MarketTier};
use crate::modules::admin;
use crate::errors::ErrorCode;

#[contracttype]
pub enum DataKey {
    TotalFeesCollected,
    FeeRevenue(Address), // token_address -> amount
}

pub fn get_base_fee(e: &Env) -> i128 {
    e.storage().persistent().get(&ConfigKey::BaseFee).unwrap_or(0)
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

    let mut overall: i128 = e.storage().persistent().get(&DataKey::TotalFeesCollected).unwrap_or(0);
    overall += amount; // Simplified overall tracking (assuming normalized units for analytics)
    e.storage().persistent().set(&DataKey::TotalFeesCollected, &overall);

    // Emit standardized fee collection event using soroban_sdk
    use soroban_sdk::symbol_short;
    e.events().publish(
        (symbol_short!("fee_colct"),),
        amount,
    );
}

pub fn get_revenue(e: &Env, token: Address) -> i128 {
    e.storage().persistent().get(&DataKey::FeeRevenue(token)).unwrap_or(0)
}
