#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Address, String, Vec};

mod types;
mod errors;
mod modules;
mod test;
mod test_snapshot_voting;
mod test_resolution_state_machine;
mod test_multi_token;
mod test_cancellation;

use crate::types::{ConfigKey, CircuitBreakerState};
use crate::modules::admin;
use crate::errors::ErrorCode;

#[contract]
pub struct PredictIQ;

#[contractimpl]
impl PredictIQ {
    pub fn initialize(e: Env, admin: Address, base_fee: i128) -> Result<(), ErrorCode> {
        if e.storage().persistent().has(&ConfigKey::Admin) {
            return Err(ErrorCode::AlreadyInitialized);
        }
        
        admin::set_admin(&e, admin);
        e.storage().persistent().set(&ConfigKey::BaseFee, &base_fee);
        e.storage().persistent().set(&ConfigKey::CircuitBreakerState, &CircuitBreakerState::Closed);
        Ok(())
    }

    pub fn get_admin(e: Env) -> Option<Address> {
        admin::get_admin(&e)
    }

    pub fn create_market(
        e: Env,
        creator: Address,
        description: String,
        options: Vec<String>,
        deadline: u64,
        resolution_deadline: u64,
        oracle_config: crate::types::OracleConfig,
        token_address: Address,
    ) -> Result<u64, ErrorCode> {
        crate::modules::markets::create_market(
            &e,
            creator,
            description,
            options,
            deadline,
            resolution_deadline,
            oracle_config,
            token_address,
        )
    }

    pub fn place_bet(
        e: Env,
        bettor: Address,
        market_id: u64,
        outcome: u32,
        amount: i128,
        token_address: Address,
    ) -> Result<(), ErrorCode> {
        crate::modules::bets::place_bet(&e, bettor, market_id, outcome, amount, token_address)
    }

    pub fn claim_winnings(
        e: Env,
        bettor: Address,
        market_id: u64,
    ) -> Result<i128, ErrorCode> {
        crate::modules::bets::claim_winnings(&e, bettor, market_id)
    }

    pub fn get_market(e: Env, id: u64) -> Option<crate::types::Market> {
        crate::modules::markets::get_market(&e, id)
    }

    pub fn cast_vote(e: Env, voter: Address, market_id: u64, outcome: u32, weight: i128) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::require_closed(&e)?;
        crate::modules::voting::cast_vote(&e, voter, market_id, outcome, weight)
    }

    pub fn file_dispute(e: Env, disciplinarian: Address, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::require_closed(&e)?;
        crate::modules::disputes::file_dispute(&e, disciplinarian, market_id)
    }

    pub fn set_circuit_breaker(e: Env, state: crate::types::CircuitBreakerState) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::set_state(&e, state)
    }

    pub fn set_base_fee(e: Env, amount: i128) -> Result<(), ErrorCode> {
        crate::modules::fees::set_base_fee(&e, amount)
    }

    pub fn get_revenue(e: Env, token: Address) -> i128 {
        crate::modules::fees::get_revenue(&e, token)
    }

    pub fn set_oracle_result(e: Env, market_id: u64, outcome: u32) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::oracles::set_oracle_result(&e, market_id, outcome)
    }

    pub fn resolve_market(e: Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::disputes::resolve_market(&e, market_id, winning_outcome)
    }

    pub fn reset_monitoring(e: Env) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::monitoring::reset_monitoring(&e);
        Ok(())
    }

    pub fn set_governance_token(e: Env, token: Address) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        e.storage().instance().set(&ConfigKey::GovernanceToken, &token);
        Ok(())
    }

    pub fn unlock_tokens(e: Env, voter: Address, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::voting::unlock_tokens(&e, voter, market_id)
    }

    pub fn attempt_oracle_resolution(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::require_closed(&e)?;
        crate::modules::resolution::attempt_oracle_resolution(&e, market_id)
    }

    pub fn finalize_resolution(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::require_closed(&e)?;
        crate::modules::resolution::finalize_resolution(&e, market_id)
    }

    pub fn cancel_market_admin(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::cancellation::cancel_market_admin(&e, market_id)
    }

    pub fn cancel_market_vote(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::cancellation::cancel_market_vote(&e, market_id)
    }

    pub fn withdraw_refund(e: Env, bettor: Address, market_id: u64) -> Result<i128, ErrorCode> {
        crate::modules::cancellation::withdraw_refund(&e, bettor, market_id)
    }
}
