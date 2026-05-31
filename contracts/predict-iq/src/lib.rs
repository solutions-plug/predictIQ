#![no_std]
use soroban_sdk::{contract, contractimpl, Address, Env, String, Vec};

mod errors;
mod modules;
pub mod pyth_client;
mod test;
mod test_pyth_integration;
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

    /// Step 1: propose a new admin (current admin only). New admin must call accept_admin.
    pub fn propose_admin(e: Env, new_admin: Address) -> Result<(), ErrorCode> {
        admin::propose_admin(&e, new_admin)
    }

    /// Step 2: accept a pending admin transfer (pending admin only).
    pub fn accept_admin(e: Env, caller: Address) -> Result<(), ErrorCode> {
        admin::accept_admin(&e, caller)
    }

    /// Cancel a pending admin transfer (current admin only).
    pub fn cancel_admin_transfer(e: Env) -> Result<(), ErrorCode> {
        admin::cancel_admin_transfer(&e)
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

    pub fn create_market_with_dispute_window(
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
        dispute_window_seconds: Option<u64>,
    ) -> Result<u64, ErrorCode> {
        crate::modules::markets::create_market_with_dispute_window(
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
            dispute_window_seconds,
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
        crate::modules::bets::place_bet(
            &e,
            bettor,
            market_id,
            outcome,
            amount,
            token_address,
            referrer,
        )
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

    pub fn set_dispute_window(e: Env, seconds: u64) -> Result<(), ErrorCode> {
        crate::modules::resolution::set_dispute_window(&e, seconds)
    }

    pub fn set_dispute_window_bounds(
        e: Env,
        min_seconds: u64,
        max_seconds: u64,
    ) -> Result<(), ErrorCode> {
        crate::modules::resolution::set_dispute_window_bounds(&e, min_seconds, max_seconds)
    }

    pub fn get_market_dispute_window(e: Env, market_id: u64) -> u64 {
        crate::modules::markets::get_market_dispute_window(&e, market_id)
    }

    pub fn set_circuit_breaker(
        e: Env,
        state: crate::types::CircuitBreakerState,
    ) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::set_state(&e, state)
    }

    /// Governance: update the circuit breaker threshold (admin only).
    pub fn set_circuit_breaker_threshold(e: Env, threshold: i128) -> Result<(), ErrorCode> {
        crate::modules::circuit_breaker::set_threshold(&e, threshold)
    }

    /// Query the current circuit breaker threshold.
    pub fn get_circuit_breaker_threshold(e: Env) -> i128 {
        crate::modules::circuit_breaker::get_threshold(&e)
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

    pub fn set_fee_admin(e: Env, fee_admin: Address) -> Result<(), ErrorCode> {
        crate::modules::fees::set_fee_admin(&e, fee_admin)
    }

    pub fn withdraw_protocol_fees(
        e: Env,
        token: Address,
        recipient: Address,
    ) -> Result<i128, ErrorCode> {
        crate::modules::fees::withdraw_protocol_fees(&e, &token, &recipient)
    }

    pub fn claim_referral_rewards(
        e: Env,
        address: Address,
        token: Address,
    ) -> Result<i128, ErrorCode> {
        crate::modules::fees::claim_referral_rewards(&e, &address, &token)
    }

    pub fn set_oracle_result(
        e: Env,
        market_id: u64,
        oracle_id: u32,
        outcome: u32,
    ) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::oracles::set_oracle_result(&e, market_id, oracle_id, outcome)
    }

    pub fn get_oracle_result(e: Env, market_id: u64, oracle_id: u32) -> Option<u32> {
        crate::modules::oracles::get_oracle_result(&e, market_id, oracle_id)
    }

    pub fn get_oracle_last_update(e: Env, market_id: u64, oracle_id: u32) -> Option<u64> {
        crate::modules::oracles::get_last_update(&e, market_id, oracle_id)
    }

    /// Issue #508: Validate oracle staleness for a market
    pub fn validate_oracle_staleness(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        let market =
            crate::modules::markets::get_market(&e, market_id).ok_or(ErrorCode::MarketNotFound)?;
        crate::modules::oracles::validate_oracle_staleness(&e, market_id, &market.oracle_config)
    }

    pub fn resolve_market(e: Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::disputes::resolve_market(&e, market_id, winning_outcome)
    }

    /// Set the governance token used for dispute voting weights.
    pub fn set_governance_token(e: Env, token: Address) -> Result<(), ErrorCode> {
        crate::modules::admin::set_governance_token(&e, token)
    }

    /// Attempt to resolve a market via the oracle after the resolution deadline.
    /// Transitions status: Active → PendingResolution.
    pub fn attempt_oracle_resolution(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::resolution::attempt_oracle_resolution(&e, market_id)
    }

    /// Finalize resolution after the dispute window has closed.
    /// Handles both the no-dispute path (PendingResolution → Resolved) and
    /// the post-vote path (Disputed → Resolved).
    pub fn finalize_resolution(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::resolution::finalize_resolution(&e, market_id)
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

    /// Issue #507: Set market creation fee (admin only)
    pub fn set_creation_fee(e: Env, amount: i128) -> Result<(), ErrorCode> {
        crate::modules::markets::set_creation_fee(&e, amount)
    }

    /// Issue #507: Get market creation fee
    pub fn get_creation_fee(e: Env) -> i128 {
        crate::modules::markets::get_creation_fee(&e)
    }

    /// Issue #507: Set protocol treasury address (admin only)
    pub fn set_protocol_treasury(e: Env, treasury: Address) -> Result<(), ErrorCode> {
        crate::modules::markets::set_protocol_treasury(&e, treasury)
    }

    /// Issue #507: Get protocol treasury address
    pub fn get_protocol_treasury(e: Env) -> Address {
        crate::modules::markets::get_protocol_treasury(&e)
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

    pub fn vote_on_guardian_removal(
        e: Env,
        voter: Address,
        approve: bool,
    ) -> Result<(), ErrorCode> {
        crate::modules::governance::vote_on_guardian_removal(&e, voter, approve)
    }

    pub fn execute_guardian_removal(e: Env) -> Result<(), ErrorCode> {
        crate::modules::governance::execute_guardian_removal(&e)
    }

    pub fn get_guardians(e: Env) -> Vec<crate::types::Guardian> {
        crate::modules::governance::get_guardians(&e)
    }

    pub fn initiate_upgrade(e: Env, wasm_hash: soroban_sdk::BytesN<32>) -> Result<(), ErrorCode> {
        crate::modules::governance::initiate_upgrade(&e, wasm_hash)
    }

    pub fn vote_for_upgrade(e: Env, voter: Address, vote_for: bool) -> Result<bool, ErrorCode> {
        crate::modules::governance::vote_for_upgrade(&e, voter, vote_for)
    }

    pub fn execute_upgrade(e: Env) -> Result<soroban_sdk::BytesN<32>, ErrorCode> {
        crate::modules::governance::execute_upgrade(&e)
    }

    pub fn get_pending_upgrade(e: Env) -> Option<crate::types::PendingUpgrade> {
        crate::modules::governance::get_pending_upgrade(&e)
    }

    pub fn get_upgrade_votes(e: Env) -> Result<crate::types::UpgradeVoteStats, ErrorCode> {
        crate::modules::governance::get_upgrade_votes(&e)
    }

    pub fn is_timelock_satisfied(e: Env) -> Result<bool, ErrorCode> {
        crate::modules::governance::is_timelock_satisfied(&e)
    }

    pub fn set_timelock_duration(e: Env, seconds: u64) -> Result<(), ErrorCode> {
        crate::modules::governance::set_timelock_duration(&e, seconds)
    }

    pub fn get_timelock_duration(e: Env) -> u64 {
        crate::modules::governance::get_timelock_duration(&e)
    }

    pub fn emergency_pause(e: Env, voter: Address) -> Result<(), ErrorCode> {
        crate::modules::governance::emergency_pause(&e, voter)
    }

    /// Prune (archive) a resolved market after 30 days grace period
    pub fn prune_market(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::markets::prune_market(&e, market_id)
    }

    pub fn cancel_market_admin(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::cancellation::cancel_market_admin(&e, market_id)
    }

    pub fn cancel_market_vote(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::cancellation::cancel_market_vote(&e, market_id)
    }
}
