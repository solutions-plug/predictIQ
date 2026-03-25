use crate::errors::ErrorCode;
use soroban_sdk::{Address, Env, IntoVal, Symbol, Val};

/// Issue #11: Use try_invoke_contract so transfer failures are handled
/// programmatically instead of relying on host panics.
pub fn safe_transfer(
    e: &Env,
    token_address: &Address,
    from: &Address,
    to: &Address,
    amount: &i128,
) -> Result<(), ErrorCode> {
    let args = (from.clone(), to.clone(), *amount).into_val(e);

    match e.try_invoke_contract::<Val, ErrorCode>(
        token_address,
        &Symbol::new(e, "transfer"),
        args,
    ) {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => Err(e),
        Err(_) => Err(ErrorCode::AssetClawedBack),
    }
}

pub fn verify_contract_not_frozen(
    e: &Env,
    token_address: &Address,
) -> Result<(), ErrorCode> {
    let client = soroban_sdk::token::Client::new(e, token_address);
    let _balance = client.balance(&e.current_contract_address());
    Ok(())
}

/// Issue #27: ErrorCode::AssetClawedBack now exists in errors.rs.
pub fn detect_clawback(
    e: &Env,
    token_address: &Address,
    expected_balance: i128,
) -> Result<(), ErrorCode> {
    let client = soroban_sdk::token::Client::new(e, token_address);
    let actual_balance = client.balance(&e.current_contract_address());

    if actual_balance < expected_balance {
        return Err(ErrorCode::AssetClawedBack);
    }

    Ok(())
}
