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
            crate::modules::monitoring::track_error(e);
            ErrorCode::TransferFailed
        })?
        .map_err(|_| {
            e.events().publish(
                (symbol_short!("xfer_fail"), from.clone(), to.clone()),
                (token_address.clone(), *amount),
            );
            crate::modules::monitoring::track_error(e);
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

/// Check if the contract itself can receive/send tokens (not frozen).
/// Classic Stellar assets wrapped by a SAC expose freeze as deauthorization:
/// a deauthorized (frozen) account fails `authorized()` and cannot transfer.
/// Returns Ok(()) if the contract is authorized or the token doesn't support
/// authorization (freeze is not applicable).
/// Returns Err(ErrorCode::TokenFrozen) if the contract has been deauthorized.
pub fn verify_contract_not_frozen(e: &Env, token_address: &Address) -> Result<(), ErrorCode> {
    let client = token::Client::new(e, token_address);
    let contract_addr = e.current_contract_address();

    match client.try_authorized(&contract_addr) {
        Ok(Ok(is_authorized)) => {
            if is_authorized {
                Ok(())
            } else {
                e.events().publish(
                    (symbol_short!("ctr_frz"), token_address.clone()),
                    (),
                );
                Err(ErrorCode::TokenFrozen)
            }
        }
        // Token doesn't support authorization or the view call failed -
        // treat as not frozen, matching check_token_not_frozen's behavior.
        _ => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::testutils::Address as _;

    #[test]
    fn verify_contract_not_frozen_ok_when_authorized() {
        let e = Env::default();
        e.mock_all_auths();

        let token_admin = Address::generate(&e);
        let sac = e.register_stellar_asset_contract_v2(token_admin);
        let token_address = sac.address();
        let contract_addr = Address::generate(&e);

        let result = e.as_contract(&contract_addr, || verify_contract_not_frozen(&e, &token_address));
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn verify_contract_not_frozen_detects_frozen_contract() {
        let e = Env::default();
        e.mock_all_auths();

        let token_admin = Address::generate(&e);
        let sac = e.register_stellar_asset_contract_v2(token_admin);
        let token_address = sac.address();
        let asset_client = token::StellarAssetClient::new(&e, &token_address);
        let contract_addr = Address::generate(&e);

        // Deauthorize the contract's trustline - this is how Classic Stellar
        // assets implement freeze.
        asset_client.set_authorized(&contract_addr, &false);

        let result = e.as_contract(&contract_addr, || verify_contract_not_frozen(&e, &token_address));
        assert_eq!(result, Err(ErrorCode::TokenFrozen));
    }
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
