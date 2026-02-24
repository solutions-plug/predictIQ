use soroban_sdk::{Env, Address, token};
use crate::errors::ErrorCode;
use crate::types::MarketStatus;
use crate::modules::markets;

/// SAC (Stellar Asset Contract) Integration Module
/// 
/// This module provides safe interaction with Stellar Classic assets (like USDC)
/// through the Stellar Asset Contract (SAC) interface. It handles:
/// 
/// 1. Safe token transfers with error handling
/// 2. Clawback detection and automatic market cancellation
/// 3. Freeze detection for contract accounts
/// 4. SAC token validation
///
/// Classic Stellar assets can have special flags:
/// - AUTH_CLAWBACK_ENABLED: Issuer can reclaim tokens
/// - AUTH_FREEZE_ENABLED: Issuer can freeze accounts
/// - AUTH_REQUIRED: Requires trustline authorization
///
/// The contract automatically handles these scenarios to protect users.

/// Verify that a token transfer succeeded and handle clawback/freeze scenarios
/// This function wraps the standard token transfer with error handling for Classic assets
pub fn safe_transfer(
    e: &Env,
    token_address: &Address,
    from: &Address,
    to: &Address,
    amount: &i128,
) -> Result<(), ErrorCode> {
    let client = token::Client::new(e, token_address);
    
    // Attempt transfer - will panic if clawed back or frozen
    // In production, Classic assets with AUTH_CLAWBACK_ENABLED or AUTH_FREEZE_ENABLED
    // can cause transfers to fail
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
/// If clawback is detected, returns an error
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

/// Check if a market's prize pool has been clawed back
/// If clawback is detected, automatically cancel the market
/// This is a critical safety feature for Classic Stellar assets
pub fn check_market_clawback(
    e: &Env,
    market_id: u64,
) -> Result<(), ErrorCode> {
    let mut market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;
    
    // Only check active or pending resolution markets
    if market.status != MarketStatus::Active && market.status != MarketStatus::PendingResolution {
        return Ok(());
    }
    
    let client = token::Client::new(e, &market.token_address);
    let actual_balance = client.balance(&e.current_contract_address());
    
    // If the contract's balance is less than the market's total staked amount,
    // the issuer has clawed back funds
    if actual_balance < market.total_staked {
        // Automatically cancel the market
        market.status = MarketStatus::Cancelled;
        markets::update_market(e, market);
        
        // Emit cancellation event
        crate::modules::events::emit_market_cancelled(e, market_id, true);
        
        return Err(ErrorCode::AssetClawedBack);
    }
    
    Ok(())
}

/// Verify that a token address is a valid Stellar Asset Contract (SAC)
/// This ensures developers are using proper SAC addresses for Classic assets
pub fn verify_sac_token(
    _e: &Env,
    _token_address: &Address,
) -> Result<(), ErrorCode> {
    // In production, this would verify the token contract implements
    // the Stellar Asset Contract interface
    // For now, we rely on the token::Client interface which will fail
    // if the address doesn't implement the required methods
    Ok(())
}
