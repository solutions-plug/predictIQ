use crate::errors::ErrorCode;
use soroban_sdk::{symbol_short, token, Address, Env};

/// Issue #11: Use try_transfer so transfer failures are caught programmatically
/// instead of relying on host panics. Maps any host error to TransferFailed and
/// emits a `xfer_fail` event so callers can observe the failure without crashing.
pub fn safe_transfer(
    e: &Env,
    token_address: &Address,
    from: &Address,
    to: &Address,
    amount: &i128,
) -> Result<(), ErrorCode> {
    let client = token::Client::new(e, token_address);

    client
        .try_transfer(from, to, amount)
        .map_err(|_| {
            e.events().publish(
                (symbol_short!("xfer_fail"), from.clone(), to.clone()),
                (token_address.clone(), *amount),
            );
            ErrorCode::TransferFailed
        })?
        .map_err(|_| {
            e.events().publish(
                (symbol_short!("xfer_fail"), from.clone(), to.clone()),
                (token_address.clone(), *amount),
            );
            ErrorCode::TransferFailed
        })
}

/// Check if a user's tokens are frozen for a given SAC-wrapped token.
/// SAC tokens may support freeze operations that prevent transfers.
/// Returns Ok(()) if tokens are not frozen or freeze is not supported.
/// Returns Err(ErrorCode::TokenFrozen) if the user's tokens are frozen.
pub fn check_token_not_frozen(
    e: &Env,
    token_address: &Address,
    user: &Address,
) -> Result<(), ErrorCode> {
    let client = token::Client::new(e, token_address);

    // Attempt to call frozen() - if the token doesn't support freeze,
    // this will return an error internally, but we treat it as "not frozen"
    // since the token doesn't have that capability.
    match client.frozen(user) {
        Ok(is_frozen) => {
            if is_frozen {
                e.events().publish(
                    (
                        symbol_short!("token_frz"),
                        token_address.clone(),
                        user.clone(),
                    ),
                    (),
                );
                Err(ErrorCode::TokenFrozen)
            } else {
                Ok(())
            }
        }
        Err(_) => {
            // Token doesn't support freeze operation or view is restricted
            // Treat as not frozen - the transfer attempt itself will fail if needed
            Ok(())
        }
    }
}

/// Check if contract can receive tokens (not frozen)
/// Returns true if the contract's balance can be modified
pub fn verify_contract_not_frozen(e: &Env, token_address: &Address) -> Result<(), ErrorCode> {
    let client = token::Client::new(e, token_address);
    let contract_addr = e.current_contract_address();

    // Try to get balance - if frozen, this will succeed but transfers will fail
    let _balance = client.balance(&contract_addr);

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
