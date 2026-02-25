use soroban_sdk::{symbol_short, Address, Env};

/// Standardized Event Emission Module
///
/// Event Topic Layout:
/// - Topic 0: Event Name (short symbol, max 9 chars)
/// - Topic 1: market_id (u64) - primary identifier for indexers
/// - Topic 2: Triggering Address - who initiated the action
///
/// This standardization ensures external indexers can perfectly reconstruct
/// market states by following a consistent event schema.

/// Emit MarketCreated event
/// Topics: [mkt_creat, market_id, creator]
/// Data: (description, num_outcomes, deadline)
pub fn emit_market_created(
    e: &Env,
    market_id: u64,
    creator: Address,
    description: soroban_sdk::String,
    num_outcomes: u32,
    deadline: u64,
) {
    e.events().publish(
        (symbol_short!("mkt_creat"), market_id, creator),
        (description, num_outcomes, deadline),
    );
}

/// Emit BetPlaced event
/// Topics: [bet_place, market_id, bettor]
/// Data: (outcome, amount)
pub fn emit_bet_placed(e: &Env, market_id: u64, bettor: Address, outcome: u32, amount: i128) {
    e.events().publish(
        (symbol_short!("bet_place"), market_id, bettor),
        (outcome, amount),
    );
}

/// Emit DisputeFiled event
/// Topics: [disp_file, market_id, disciplinarian]
/// Data: (new_deadline)
pub fn emit_dispute_filed(e: &Env, market_id: u64, disciplinarian: Address, new_deadline: u64) {
    e.events().publish(
        (symbol_short!("disp_file"), market_id, disciplinarian),
        new_deadline,
    );
}

/// Emit ResolutionFinalized event
/// Topics: [resolv_fx, market_id, resolver]
/// Data: (winning_outcome, total_payout)
pub fn emit_resolution_finalized(
    e: &Env,
    market_id: u64,
    resolver: Address,
    winning_outcome: u32,
    total_payout: i128,
) {
    e.events().publish(
        (symbol_short!("resolv_fx"), market_id, resolver),
        (winning_outcome, total_payout),
    );
}

/// Emit RewardsClaimed event
/// Topics: [reward_fx, market_id, claimer]
/// Data: (amount, is_refund)
pub fn emit_rewards_claimed(
    e: &Env,
    market_id: u64,
    claimer: Address,
    amount: i128,
    is_refund: bool,
) {
    e.events().publish(
        (symbol_short!("reward_fx"), market_id, claimer),
        (amount, is_refund),
    );
}

/// Emit VoteCast event for governance
/// Topics: [vote_cast, market_id, voter]
/// Data: (outcome, weight)
pub fn emit_vote_cast(e: &Env, market_id: u64, voter: Address, outcome: u32, weight: i128) {
    e.events().publish(
        (symbol_short!("vote_cast"), market_id, voter),
        (outcome, weight),
    );
}

/// Emit CircuitBreakerTriggered event for system state changes
/// Topics: [cb_state, 0 (no market), contract_address]
/// Data: (state)
pub fn emit_circuit_breaker_triggered(
    e: &Env,
    contract_address: Address,
    state: soroban_sdk::String,
) {
    e.events()
        .publish((symbol_short!("cb_state"), 0u64, contract_address), state);
}

/// Emit OracleResultSet event
/// Topics: [oracle_ok, market_id, oracle_address]
/// Data: (outcome)
pub fn emit_oracle_result_set(e: &Env, market_id: u64, oracle_address: Address, outcome: u32) {
    e.events().publish(
        (symbol_short!("oracle_ok"), market_id, oracle_address),
        outcome,
    );
}

/// Emit MarketCancelled event
/// Topics: [mkt_cancl, market_id, contract_address]
/// Data: (is_clawback)
pub fn emit_market_cancelled(e: &Env, market_id: u64, is_clawback: bool) {
    e.events().publish(
        (symbol_short!("mkt_cancl"), market_id, e.current_contract_address()),
        is_clawback,
    );
}
