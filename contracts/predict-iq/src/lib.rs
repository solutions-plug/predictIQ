#![cfg_attr(not(test), no_std)]
use soroban_sdk::{contract, contractimpl, Address, BytesN, Env, String, Vec};

pub mod errors;
mod modules;
mod test;
#[cfg(test)]
mod query_tests;
#[cfg(test)]
mod test_tie_handling;
#[cfg(test)]
mod test_payout_mode_immutability;
pub mod types;

pub use errors::ErrorCode;

use crate::modules::admin;
use crate::types::{CircuitBreakerState, ConfigKey, Guardian, UpgradeStats};

#[contract]
pub struct PredictIQ;

#[contractimpl]
impl PredictIQ {
    pub fn initialize(
        e: Env,
        admin: Address,
        base_fee: i128,
        guardians: Vec<crate::types::Guardian>,
    ) -> Result<(), ErrorCode> {
        // Require the deployer's authorization to prevent front-running attacks.
        // Only the account that deployed this contract can call initialize.
        e.deployer().require_auth();

        if e.storage().persistent().has(&ConfigKey::Admin) {
            return Err(ErrorCode::AlreadyInitialized);
        }

        if guardians.is_empty() {
            return Err(ErrorCode::NotAuthorized);
        }

        admin::set_admin(&e, admin);
        e.storage().persistent().set(&ConfigKey::BaseFee, &base_fee);
        e.storage().instance().set(
            &ConfigKey::CircuitBreakerState,
            &CircuitBreakerState::Closed,
        );
        crate::modules::governance::initialize_guardians(&e, guardians)?;
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

    pub fn claim_winnings(e: Env, bettor: Address, market_id: u64) -> Result<i128, ErrorCode> {
        crate::modules::bets::claim_winnings(&e, bettor, market_id)
    }

    pub fn withdraw_refund(e: Env, bettor: Address, market_id: u64) -> Result<i128, ErrorCode> {
        crate::modules::cancellation::withdraw_refund(&e, bettor, market_id)
    }

    pub fn cancel_market_admin(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::cancellation::cancel_market_admin(&e, market_id)
    }

    pub fn get_market(e: Env, id: u64) -> Option<crate::types::Market> {
        crate::modules::markets::get_market(&e, id)
    }

    pub fn get_outcome_stake(e: Env, market_id: u64, outcome: u32) -> i128 {
        crate::modules::markets::get_outcome_stake(&e, market_id, outcome)
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

    pub fn unlock_tokens(e: Env, voter: Address, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::voting::unlock_tokens(&e, voter, market_id)
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

    pub fn set_fee_admin(e: Env, fee_admin: Address) -> Result<(), ErrorCode> {
        crate::modules::admin::set_fee_admin(&e, fee_admin)
    }

    pub fn get_fee_admin(e: Env) -> Option<Address> {
        crate::modules::admin::get_fee_admin(&e)
    }

    pub fn get_revenue(e: Env, token: Address) -> i128 {
        crate::modules::fees::get_revenue(&e, token)
    }

    /// Issue #26: Withdraw accumulated protocol fees to a recipient.
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

    pub fn set_oracle_result(e: Env, market_id: u64, outcome: u32) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::oracles::set_oracle_result(&e, market_id, outcome)
    }

    pub fn resolve_market(e: Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::disputes::resolve_market(&e, market_id, winning_outcome)
    }

    pub fn attempt_oracle_resolution(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::resolution::attempt_oracle_resolution(&e, market_id)
    }

    pub fn finalize_resolution(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::resolution::finalize_resolution(&e, market_id)
    }

    /// Issue #63: Administrative fallback for disputed markets that failed to
    /// reach the 60% community majority threshold after the full voting period.
    ///
    /// Only callable by the master admin. Enforces that:
    ///   - The market is still Disputed (not already resolved).
    ///   - The 72-hour voting window has fully elapsed.
    ///   - Community voting genuinely deadlocked (NoMajorityReached).
    ///
    /// This ensures user capital is never permanently orphaned while keeping
    /// the community-first resolution path intact.
    pub fn admin_fallback_resolution(
        e: Env,
        market_id: u64,
        winning_outcome: u32,
    ) -> Result<(), ErrorCode> {
        crate::modules::resolution::admin_fallback_resolution(&e, market_id, winning_outcome)
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

    pub fn set_governance_token(e: Env, token: Address) -> Result<(), ErrorCode> {
        crate::modules::admin::set_governance_token(&e, token)
    }

    pub fn get_resolution_metrics(
        e: Env,
        market_id: u64,
        outcome: u32,
    ) -> crate::modules::disputes::ResolutionMetrics {
        crate::modules::disputes::get_resolution_metrics(&e, market_id, outcome)
    }

    pub fn set_max_push_payout_winners(e: Env, threshold: u32) -> Result<(), ErrorCode> {
        crate::modules::disputes::set_max_push_payout_winners(&e, threshold)
    }

    pub fn get_max_push_payout_winners(e: Env) -> u32 {
        crate::modules::disputes::get_max_push_payout_winners(&e)
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
    pub fn add_guardian(e: Env, guardian: crate::types::Guardian) -> Result<(), ErrorCode> {
        crate::modules::governance::add_guardian(&e, guardian)
    }

    pub fn remove_guardian(e: Env, address: Address) -> Result<(), ErrorCode> {
        crate::modules::governance::remove_guardian(&e, address)
    }

    pub fn vote_on_guardian_removal(e: Env, voter: Address, approve: bool) -> Result<(), ErrorCode> {
        crate::modules::governance::vote_on_guardian_removal(&e, voter, approve)
    }

    pub fn get_guardians(e: Env) -> Vec<crate::types::Guardian> {
        crate::modules::governance::get_guardians(&e)
    }

    pub fn initiate_upgrade(e: Env, wasm_hash: BytesN<32>) -> Result<(), ErrorCode> {
        crate::modules::governance::initiate_upgrade(&e, wasm_hash)
    }

    pub fn vote_for_upgrade(e: Env, voter: Address, vote_for: bool) -> Result<bool, ErrorCode> {
        crate::modules::governance::vote_for_upgrade(&e, voter, vote_for)
    }

    pub fn execute_upgrade(e: Env) -> Result<(), ErrorCode> {
        crate::modules::governance::execute_upgrade(&e)
    }

    pub fn get_pending_upgrade(e: Env) -> Option<crate::types::PendingUpgrade> {
        crate::modules::governance::get_pending_upgrade(&e)
    }

    /// Issue #33: Returns named UpgradeStats struct.
    pub fn get_upgrade_votes(e: Env) -> Result<UpgradeStats, ErrorCode> {
        crate::modules::governance::get_upgrade_votes(&e)
    }

    pub fn is_timelock_satisfied(e: Env) -> Result<bool, ErrorCode> {
        crate::modules::governance::is_timelock_satisfied(&e)
    }

    /// Issue #13: Configurable timelock duration.
    pub fn set_timelock_duration(e: Env, seconds: u64) -> Result<(), ErrorCode> {
        crate::modules::governance::set_timelock_duration(&e, seconds)
    }

    /// Issue #13: Returns the currently active timelock duration in seconds.
    pub fn get_timelock_duration(e: Env) -> u64 {
        crate::modules::governance::get_timelock_duration(&e)
    }

    /// Issue #8: Set the dispute window duration (admin-only, minimum 24h).
    pub fn set_dispute_window(e: Env, seconds: u64) -> Result<(), ErrorCode> {
        crate::modules::resolution::set_dispute_window(&e, seconds)
    }

    /// Issue #8: Get the active dispute window duration in seconds (default 72h).
    pub fn get_dispute_window(e: Env) -> u64 {
        crate::modules::resolution::get_dispute_window(&e)
    }

    /// Issue #47: Permissionless prune after grace period.
    pub fn prune_market(e: Env, market_id: u64) -> Result<(), ErrorCode> {
        crate::modules::markets::prune_market(&e, market_id)
    }

    pub fn get_minimum_bet_amount(e: Env) -> i128 {
        crate::modules::bets::get_minimum_bet_amount(&e)
    }

    pub fn set_minimum_bet_amount(e: Env, amount: i128) -> Result<(), ErrorCode> {
        crate::modules::bets::set_minimum_bet_amount(&e, amount)
    }

    /// Emergency pause triggered by 2/3 Guardian majority (community panic override)
    pub fn emergency_pause(e: Env, voter: Address) -> Result<(), ErrorCode> {
        crate::modules::governance::emergency_pause(&e, voter)
    }

    /// Get the accurate bet count for a specific outcome (analytics)
    pub fn count_bets_for_outcome(e: Env, market_id: u64, outcome: u32) -> u32 {
        crate::modules::markets::count_bets_for_outcome(&e, market_id, outcome)
    }
}
