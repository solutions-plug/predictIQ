#![cfg(test)]
use soroban_sdk::{contract, contractimpl, Env, Address, Map};

#[contract]
pub struct MockIdentityContract;

#[contractimpl]
impl MockIdentityContract {
    pub fn set_verified(e: Env, user: Address, verified: bool) {
        let mut verifications: Map<Address, bool> = e
            .storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("verified"))
            .unwrap_or(Map::new(&e));
        
        verifications.set(user, verified);
        e.storage()
            .instance()
            .set(&soroban_sdk::symbol_short!("verified"), &verifications);
    }

    pub fn is_verify(e: Env, user: Address) -> bool {
        let verifications: Map<Address, bool> = e
            .storage()
            .instance()
            .get(&soroban_sdk::symbol_short!("verified"))
            .unwrap_or(Map::new(&e));
        
        verifications.get(user).unwrap_or(false)
    }
}
