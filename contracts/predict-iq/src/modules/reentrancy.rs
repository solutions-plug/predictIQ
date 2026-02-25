use soroban_sdk::{Env, contracttype};
use crate::errors::ErrorCode;

#[contracttype]
pub enum DataKey {
    ProtocolLock,
    OracleLastUpdate(u64), // market_id -> ledger_sequence
}

pub struct ReentrancyGuard<'a> {
    env: &'a Env,
}

impl<'a> ReentrancyGuard<'a> {
    pub fn new(env: &'a Env) -> Result<Self, ErrorCode> {
        let is_locked: bool = env.storage().instance().get(&DataKey::ProtocolLock).unwrap_or(false);
        
        if is_locked {
            return Err(ErrorCode::ProtocolLocked);
        }
        
        env.storage().instance().set(&DataKey::ProtocolLock, &true);
        
        Ok(ReentrancyGuard { env })
    }
}

impl<'a> Drop for ReentrancyGuard<'a> {
    fn drop(&mut self) {
        self.env.storage().instance().set(&DataKey::ProtocolLock, &false);
    }
}

// Oracle manipulation prevention
pub fn record_oracle_update(e: &Env, market_id: u64) {
    let current_ledger = e.ledger().sequence();
    e.storage().instance().set(&DataKey::OracleLastUpdate(market_id), &current_ledger);
}

pub fn check_oracle_freshness(e: &Env, market_id: u64) -> Result<(), ErrorCode> {
    let current_ledger = e.ledger().sequence();
    let last_update: Option<u32> = e.storage().instance().get(&DataKey::OracleLastUpdate(market_id));
    
    if let Some(last_ledger) = last_update {
        if last_ledger == current_ledger {
            return Err(ErrorCode::OracleUpdateTooRecent);
        }
    }
    
    Ok(())
}
