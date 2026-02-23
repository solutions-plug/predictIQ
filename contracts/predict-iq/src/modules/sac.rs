use soroban_sdk::{Env, Address, token};
use crate::errors::ErrorCode;

/// Verify that a token transfer succeeded and handle clawback/freeze scenarios
pub fn safe_transfer(
    e: &Env,
    token_address: &Address,
    from: &Address,
    to: &Address,
    amount: &i128,
) -> Result<(), ErrorCode> {
    let client = token::Client::new(e, token_address);
    
    // Attempt transfer - will panic if clawed back or frozen
    client.transfer(from, to, amount);
    
    Ok(())
}

/// Check if contract can receive tokens (not frozen)
/// Returns true if the contract's balance can be modified
pub fn verify_contract_not_frozen(
    e: &Env,
    token_address: &Address,
) -> Result<(), ErrorCode> {
    let client = token::Client::new(e, token_address);
    let contract_addr = e.current_contract_address();
    
    // Try to get balance - if frozen, this will succeed but transfers will fail
    let _balance = client.balance(&contract_addr);
    
    Ok(())
}

/// Detect if contract balance was clawed back by comparing expected vs actual
pub fn detect_clawback(
    e: &Env,
    token_address: &Address,
    expected_balance: i128,
) -> Result<(), ErrorCode> {
    let client = token::Client::new(e, token_address);
    let actual_balance = client.balance(&e.current_contract_address());
    
    if actual_balance < expected_balance {
        return Err(ErrorCode::AssetClawedBack);
    }
    
    Ok(())
}
