use soroban_sdk::{Env, contracttype};
use crate::types::OracleConfig;
use crate::errors::ErrorCode;

#[contracttype]
pub enum OracleData {
    Result(u64, u32), // market_id -> outcome
    LastUpdate(u64, u64), // market_id -> timestamp
}

pub fn get_oracle_result(e: &Env, market_id: u64, _config: &OracleConfig) -> Option<u32> {
    // In a real implementation, this would call the external oracle contract (Reflector/Pyth)
    // using config.oracle_address and config.feed_id.
    // For this replication, we use a storage-backed mock-ready structure.
    e.storage().persistent().get(&OracleData::Result(market_id, 0)) // Note: 0 is dummy key part
}

pub fn set_oracle_result(e: &Env, market_id: u64, outcome: u32) -> Result<(), ErrorCode> {
    // Mock oracle result for testing/demonstration
    e.storage().persistent().set(&OracleData::Result(market_id, 0), &outcome);
    e.storage().persistent().set(&OracleData::LastUpdate(market_id, 0), &e.ledger().timestamp());
    
    // Emit standardized OracleResultSet event
    // Topics: [OracleResultSet, market_id, oracle_address]
    let oracle_addr = e.current_contract_address();
    crate::modules::events::emit_oracle_result_set(e, market_id, oracle_addr, outcome);
    
    Ok(())
}

pub fn verify_oracle_health(_e: &Env, config: &OracleConfig) -> bool {
    // Check if oracle address is valid and responding (contract check)
    // For now, assume healthy if configured
    !config.feed_id.is_empty()
}
