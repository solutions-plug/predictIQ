#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

mod errors;
mod modules;
mod test;
pub mod types;

use crate::errors::ErrorCode;
use crate::modules::admin;
use crate::types::{CircuitBreakerState, ConfigKey};

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
        e.storage().persistent().set(
            &ConfigKey::CircuitBreakerState,
            &CircuitBreakerState::Closed,
        );
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
        tier: crate::types::MarketTier,
        native_token: Address,
        parent_id: u64,
        parent_outcome_idx: u32,
    ) -> Result<u64, ErrorCode> {
        crate::modules::markets::create_market(
            &e,
            creator,
            description,
            options,
            deadline,
            resolution_deadline,
            oracle_config,
            tier,
            native_token,
            parent_id,
            parent_outcome_idx,
        )
    }

    pub fn place_bet(
        e: Env,
        bettor: Address,
        market_id: u64,
        outcome: u32,
        amount: i128,
        token_address: Address,
        referrer: Option<Address>,
    ) -> Result<(), ErrorCode> {
        crate::modules::bets::place_bet(&e, bettor, market_id, outcome, amount, token_address, referrer)
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

    pub fn cast_vote(
        e: Env,
        voter: Address,
        market_id: u64,
        outcome: u32,
        weight: i128,
    ) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::require_closed(&e)?;
        crate::modules::voting::cast_vote(&e, voter, market_id, outcome, weight)
    }

    pub fn file_dispute(e: Env, disciplinarian: Address, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::require_closed(&e)?;
        crate::modules::disputes::file_dispute(&e, disciplinarian, market_id)
    }

    pub fn set_circuit_breaker(
        e: Env,
        state: crate::types::CircuitBreakerState,
    ) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::set_state(&e, state)
    }

    pub fn set_base_fee(e: Env, amount: i128) -> Result<(), ErrorCode> {
        crate::modules::fees::set_base_fee(&e, amount)
    }

    pub fn get_base_fee(e: Env) -> i128 {
        crate::modules::fees::get_base_fee(&e)
    }

    pub fn get_revenue(e: Env, token: Address) -> i128 {
        crate::modules::fees::get_revenue(&e, token)
    }

    pub fn claim_referral_rewards(e: Env, address: Address, token: Address) -> Result<i128, ErrorCode> {
        crate::modules::fees::claim_referral_rewards(&e, &address, &token)
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

    pub fn set_guardian(e: Env, guardian: Address) -> Result<(), ErrorCode> {
        crate::modules::admin::set_guardian(&e, guardian)
    }

    pub fn get_guardian(e: Env) -> Option<Address> {
        crate::modules::admin::get_guardian(&e)
    }

    pub fn pause(e: Env) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::pause(&e)
    }

    pub fn unpause(e: Env) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::unpause(&e)
    }

    pub fn claim_winnings(
        e: Env,
        bettor: Address,
        market_id: u64,
        token_address: Address,
    ) -> Result<i128, ErrorCode> {
        crate::modules::bets::claim_winnings(&e, bettor, market_id, token_address)
    }

    pub fn withdraw_refund(
        e: Env,
        bettor: Address,
        market_id: u64,
        token_address: Address,
    ) -> Result<i128, ErrorCode> {
        crate::modules::bets::withdraw_refund(&e, bettor, market_id, token_address)
    }

    pub fn resolve_market(e: Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::disputes::resolve_market(&e, market_id, winning_outcome)
    }

    pub fn get_resolution_metrics(
        e: Env,
        market_id: u64,
        outcome: u32,
    ) -> crate::modules::disputes::ResolutionMetrics {
        crate::modules::disputes::get_resolution_metrics(&e, market_id, outcome)
    }

    pub fn set_creator_reputation(
        e: Env,
        creator: Address,
        reputation: crate::types::CreatorReputation,
    ) -> Result<(), ErrorCode> {
        crate::modules::markets::set_creator_reputation(&e, creator, reputation)
    }

    pub fn get_creator_reputation(e: Env, creator: Address) -> crate::types::CreatorReputation {
        crate::modules::markets::get_creator_reputation(&e, &creator)
    }

    pub fn set_creation_deposit(e: Env, amount: i128) -> Result<(), ErrorCode> {
        crate::modules::markets::set_creation_deposit(&e, amount)
    }

    pub fn get_creation_deposit(e: Env) -> i128 {
        crate::modules::markets::get_creation_deposit(&e)
    }

    pub fn release_creation_deposit(
        e: Env,
        market_id: u64,
        native_token: Address,
    ) -> Result<(), ErrorCode> {
        crate::modules::markets::release_creation_deposit(&e, market_id, native_token)
    }

    // Governance and Upgrade Functions
    pub fn initialize_guardians(
        e: Env,
        guardians: Vec<crate::types::Guardian>,
    ) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::governance::initialize_guardians(&e, guardians)
    }

    pub fn add_guardian(e: Env, guardian: crate::types::Guardian) -> Result<(), ErrorCode> {
        crate::modules::governance::add_guardian(&e, guardian)
    }

    pub fn remove_guardian(e: Env, address: Address) -> Result<(), ErrorCode> {
        crate::modules::governance::remove_guardian(&e, address)
    }

    pub fn get_guardians(e: Env) -> Vec<crate::types::Guardian> {
        crate::modules::governance::get_guardians(&e)
    }

    pub fn initiate_upgrade(e: Env, wasm_hash: String) -> Result<(), ErrorCode> {
        crate::modules::governance::initiate_upgrade(&e, wasm_hash)
    }

    pub fn vote_for_upgrade(e: Env, voter: Address, vote_for: bool) -> Result<bool, ErrorCode> {
        crate::modules::governance::vote_for_upgrade(&e, voter, vote_for)
    }

    pub fn execute_upgrade(e: Env) -> Result<String, ErrorCode> {
        crate::modules::governance::execute_upgrade(&e)
    }

    pub fn get_pending_upgrade(e: Env) -> Option<crate::types::PendingUpgrade> {
        crate::modules::governance::get_pending_upgrade(&e)
    }

    pub fn get_upgrade_votes(e: Env) -> Result<(u32, u32), ErrorCode> {
        crate::modules::governance::get_upgrade_votes(&e)
    }

    pub fn is_timelock_satisfied(e: Env) -> Result<bool, ErrorCode> {
        crate::modules::governance::is_timelock_satisfied(&e)
    }
}
