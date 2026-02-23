#![no_std]
use soroban_sdk::{contract, contractimpl, Env, Address, String, Vec};

pub mod types;
pub mod errors;
mod modules;
mod test;
mod test_amm;
mod test_snapshot_voting;
mod test_resolution_state_machine;
mod test_multi_token;
mod test_cancellation;
mod test_referral;
mod test_optimization;
mod mock_identity;
mod test_identity;

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

    // Guardian Governance Functions
    pub fn set_guardians(e: Env, guardians: Vec<Address>) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::guardians::set_guardians(&e, guardians)
    }

    pub fn sign_reset_admin(e: Env, guardian: Address, new_admin: Address) -> Result<(), ErrorCode> {
        crate::modules::guardians::sign_reset_admin(&e, guardian, new_admin)
    }

    pub fn get_recovery_state(e: Env) -> Option<crate::modules::guardians::RecoveryState> {
        crate::modules::guardians::get_recovery_state(&e)
    }

    pub fn is_recovery_active(e: Env) -> bool {
        crate::modules::guardians::is_recovery_active(&e)
    }

    pub fn finalize_recovery(e: Env) -> Result<Address, ErrorCode> {
        crate::modules::guardians::finalize_recovery(&e)
    }

    // AMM Functions
    pub fn initialize_amm_pools(e: Env, market_id: u64, num_outcomes: u32, initial_usdc: i128) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::amm::initialize_pools(&e, market_id, num_outcomes, initial_usdc);
        Ok(())
    }

    pub fn buy_shares(
        e: Env,
        buyer: Address,
        market_id: u64,
        outcome: u32,
        usdc_in: i128,
        token_address: Address,
    ) -> Result<(i128, i128), ErrorCode> {
        buyer.require_auth();
        crate::modules::circuit_breaker::require_closed(&e)?;
        
        let client = soroban_sdk::token::Client::new(&e, &token_address);
        client.transfer(&buyer, &e.current_contract_address(), &usdc_in);
        
        crate::modules::amm::buy_shares(&e, market_id, buyer, outcome, usdc_in)
    }

    pub fn sell_shares(
        e: Env,
        seller: Address,
        market_id: u64,
        outcome: u32,
        shares_in: i128,
        token_address: Address,
    ) -> Result<(i128, i128), ErrorCode> {
        seller.require_auth();
        crate::modules::circuit_breaker::require_closed(&e)?;
        
        let (usdc_out, price) = crate::modules::amm::sell_shares(&e, market_id, seller.clone(), outcome, shares_in)?;
        
        let client = soroban_sdk::token::Client::new(&e, &token_address);
        client.transfer(&e.current_contract_address(), &seller, &usdc_out);
        
        Ok((usdc_out, price))
    }

    pub fn get_buy_price(e: Env, market_id: u64, outcome: u32) -> i128 {
        crate::modules::amm::get_buy_price(&e, market_id, outcome).unwrap_or(0)
    }

    pub fn get_user_shares(e: Env, market_id: u64, user: Address, outcome: u32) -> i128 {
        crate::modules::amm::get_user_shares(&e, market_id, user, outcome)
    }

    pub fn get_amm_pool(e: Env, market_id: u64, outcome: u32) -> Option<crate::modules::amm::AMMPool> {
        crate::modules::amm::get_pool(&e, market_id, outcome)
    }

    pub fn quote_buy(e: Env, market_id: u64, outcome: u32, usdc_in: i128) -> i128 {
        crate::modules::amm::quote_buy(&e, market_id, outcome, usdc_in).unwrap_or(0)
    }

    pub fn quote_sell(e: Env, market_id: u64, outcome: u32, shares_in: i128) -> i128 {
        crate::modules::amm::quote_sell(&e, market_id, outcome, shares_in).unwrap_or(0)
    }

    pub fn verify_pool_invariant(e: Env, market_id: u64, outcome: u32) -> bool {
        crate::modules::amm::verify_invariant(&e, market_id, outcome).unwrap_or(false)
    }

    // Storage Optimization Functions
    pub fn garbage_collect_bet(e: Env, caller: Address, market_id: u64, bettor: Address) -> Result<i128, ErrorCode> {
        crate::modules::gc::garbage_collect_bet(&e, caller, market_id, bettor)
    }

    pub fn get_market_description(e: Env, market_id: u64) -> String {
        if let Some(market) = crate::modules::markets::get_market(&e, market_id) {
            crate::modules::compression::decompress_description(&e, &market.metadata)
        } else {
            String::from_str(&e, "")
        }
    }

    pub fn get_market_options(e: Env, market_id: u64) -> Vec<String> {
        if let Some(market) = crate::modules::markets::get_market(&e, market_id) {
            crate::modules::compression::decompress_options(&e, &market.metadata)
        } else {
            Vec::new(&e)
        }
    }

    pub fn set_identity_contract(e: Env, contract: Address) -> Result<(), ErrorCode> {
        crate::modules::admin::require_admin(&e)?;
        crate::modules::identity::set_identity_contract(&e, contract);
        Ok(())
    }
}
