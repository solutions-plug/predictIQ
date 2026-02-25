use soroban_sdk::{Env, Address, contracttype};
use crate::errors::ErrorCode;

#[contracttype]
pub enum DataKey {
    IdentityContract,
}

pub fn set_identity_contract(e: &Env, contract: Address) {
    e.storage().instance().set(&DataKey::IdentityContract, &contract);
}

pub fn get_identity_contract(e: &Env) -> Option<Address> {
    e.storage().instance().get(&DataKey::IdentityContract)
}

pub fn require_verified(e: &Env, user: &Address) -> Result<(), ErrorCode> {
    if let Some(identity_contract) = get_identity_contract(e) {
        let is_verified: bool = e.invoke_contract(
            &identity_contract,
            &soroban_sdk::symbol_short!("is_verify"),
            soroban_sdk::vec![e, user.to_val()],
        );
        
        if !is_verified {
            return Err(ErrorCode::IdentityVerificationRequired);
        }
    }
    Ok(())
}
