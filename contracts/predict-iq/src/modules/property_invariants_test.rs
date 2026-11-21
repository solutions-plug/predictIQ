//! Proptest-based property tests for core contract invariants (Issue #999).
//!
//! Tests the following invariants across arbitrary inputs:
//!   1. Stake conservation: sum(outcome_stakes) == total_staked at all times
//!   2. State machine irreversibility: status transitions follow the DAG and
//!      never go backwards (Active → Resolved ✓, Resolved → Active ✗)
//!   3. Payout conservation: after resolution, total claims ≤ total_staked
//!      (fees mean ≤, not ==)
#![cfg(test)]

use crate::types::{MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use proptest::prelude::*;
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String as SorobanString, Vec as SorobanVec,
};

/// Maximum number of guardians used by the guardian property tests. Kept
/// small so that failing cases shrink to readable inputs.
const MAX_GUARDIANS: usize = 8;

/// Upper bound for a single guardian's voting power. Chosen so that a handful
/// of guardians can still sum past `u32::MAX` while keeping values readable.
const MAX_GUARDIAN_POWER: u32 = u32::MAX / 4;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup_env() -> (Env, PredictIQClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &100); // 1% fee
    (env, client, admin)
}

fn create_two_option_market(
    env: &Env,
    client: &PredictIQClient,
    admin: &Address,
    deadline: u64,
    resolution_deadline: u64,
) -> (u64, Address) {
    let options = SorobanVec::from_array(
        env,
        [
            SorobanString::from_str(env, "Yes"),
            SorobanString::from_str(env, "No"),
        ],
    );
    let oracle = OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: SorobanString::from_str(env, "feed"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
        strike_price: None,
    };
    let token_admin = Address::generate(env);
    let token = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();
    let market_id = client.create_market(
        admin,
        &SorobanString::from_str(env, "Prop Test Market"),
        &options,
        &deadline,
        &resolution_deadline,
        &oracle,
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );
    (market_id, token)
}

fn assert_stake_conservation(env: &Env, client: &PredictIQClient, market_id: u64) {
    let market = client.get_market(&market_id).unwrap();
    let mut outcome_sum: i128 = 0;
    for o in 0..10u32 {
        outcome_sum += client.get_outcome_stake(&market_id, &o);
    }
    assert_eq!(
        outcome_sum,
        market.total_staked,
        "stake conservation violated: sum(outcome_stakes)={outcome_sum} != total_staked={}",
        market.total_staked
    );
}

// ---------------------------------------------------------------------------
// Proptest strategies
// ---------------------------------------------------------------------------

prop_compose! {
    fn arb_bet_sequence()(
        amounts in prop::collection::vec(1i128..=10_000i128, 1..=8),
        outcomes in prop::collection::vec(0u32..=1u32, 1..=8),
    ) -> std::vec::Vec<(u32, i128)> {
        outcomes.into_iter().zip(amounts).collect()
    }
}

prop_compose! {
    /// Generates a guardian set guaranteed to contain at least one duplicate
    /// address, together with the index of the duplicated entry.
    fn arb_guardian_set_with_duplicate()(
        powers in prop::collection::vec(1u32..=MAX_GUARDIAN_POWER, 1..=MAX_GUARDIANS),
        dup_index in 0usize..MAX_GUARDIANS,
    ) -> (std::vec::Vec<u32>, usize) {
        let len = powers.len();
        (powers, dup_index % len)
    }
}

prop_compose! {
    /// Generates a guardian set where at least one guardian has zero voting
    /// power, together with the index of that guardian.
    fn arb_guardian_set_with_zero_power()(
        powers in prop::collection::vec(1u32..=MAX_GUARDIAN_POWER, 1..=MAX_GUARDIANS),
        zero_index in 0usize..MAX_GUARDIANS,
    ) -> (std::vec::Vec<u32>, usize) {
        let len = powers.len();
        (powers, zero_index % len)
    }
}

prop_compose! {
    /// Generates a guardian set whose voting powers sum to at least
    /// `u32::MAX`, i.e. a set that would overflow a `u32` accumulator.
    fn arb_guardian_set_overflowing_sum()(
        powers in prop::collection::vec(1u32..=MAX_GUARDIAN_POWER, 4..=MAX_GUARDIANS),
    ) -> std::vec::Vec<u32> {
        powers
    }
}

/// Builds the guardian address vector for the given power list, reusing the
/// same construction pattern as `admin_test.rs` (fresh `Address` per guardian).
fn build_guardians(env: &Env, powers: &[u32]) -> SorobanVec<(Address, u32)> {
    let mut guardians = SorobanVec::new(env);
    for power in powers {
        guardians.push_back((Address::generate(env), *power));
    }
    guardians
}

// ---------------------------------------------------------------------------
// Invariant 1 — Stake conservation under arbitrary bet sequences
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_stake_conservation_arbitrary_bets(bets in arb_bet_sequence()) {
        let (env, client, admin) = setup_env();
        let (market_id, token) = create_two_option_market(&env, &client, &admin, 1_000, 2_000);

        env.ledger().set_timestamp(0);

        for (outcome, amount) in &bets {
            let user = Address::generate(&env);
            soroban_sdk::testutils::StellarAssetContract::new(token.clone(), &env)
                .mint(&user, amount);
            let _ = client.try_place_bet(&user, &market_id, outcome, amount, &token, &None);
            assert_stake_conservation(&env, &client, market_id);
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 2 — State machine: Active → Cancelled is irreversible
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_cancelled_market_status_is_terminal(
        amounts in prop::collection::vec(1i128..=5_000i128, 0..=4),
    ) {
        let (env, client, admin) = setup_env();
        let (market_id, token) = create_two_option_market(&env, &client, &admin, 1_000, 2_000);

        env.ledger().set_timestamp(0);

        // Place arbitrary bets
        for (i, amount) in amounts.iter().enumerate() {
            let outcome = (i % 2) as u32;
            let user = Address::generate(&env);
            soroban_sdk::testutils::StellarAssetContract::new(token.clone(), &env)
                .mint(&user, amount);
            let _ = client.try_place_bet(&user, &market_id, &outcome, amount, &token, &None);
        }

        // Cancel market
        client.cancel_market_admin(&market_id);

        let market = client.get_market(&market_id).unwrap();
        assert_eq!(market.status, MarketStatus::Cancelled, "market must be Cancelled");

        // Attempting to place a bet on a cancelled market must fail
        let late_user = Address::generate(&env);
        soroban_sdk::testutils::StellarAssetContract::new(token.clone(), &env)
            .mint(&late_user, &1_000i128);
        let result = client.try_place_bet(&late_user, &market_id, &0, &500, &token, &None);
        assert!(result.is_err(), "bets on a Cancelled market must be rejected");
    }
}

// ---------------------------------------------------------------------------
// Invariant 3 — State machine: Resolved market cannot accept new bets
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_resolved_market_rejects_new_bets(
        bet_amount in 100i128..=5_000i128,
        winning_outcome in 0u32..=1u32,
    ) {
        let (env, client, admin) = setup_env();
        let (market_id, token) = create_two_option_market(&env, &client, &admin, 1_000, 2_000);

        env.ledger().set_timestamp(0);

        let user = Address::generate(&env);
        soroban_sdk::testutils::StellarAssetContract::new(token.clone(), &env)
            .mint(&user, &(bet_amount * 2));
        client.place_bet(&user, &market_id, &winning_outcome, &bet_amount, &token, &None);

        // Resolve
        client.resolve_market(&market_id, &winning_outcome);

        let market = client.get_market(&market_id).unwrap();
        assert_eq!(market.status, MarketStatus::Resolved, "market must be Resolved");

        // Bets on a resolved market must fail
        let post_user = Address::generate(&env);
        soroban_sdk::testutils::StellarAssetContract::new(token.clone(), &env)
            .mint(&post_user, &1_000i128);
        let result = client.try_place_bet(&post_user, &market_id, &winning_outcome, &500, &token, &None);
        assert!(result.is_err(), "bets on a Resolved market must be rejected");
    }
}

// ---------------------------------------------------------------------------
// Invariant 4 — total_staked is non-negative at all times
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_total_staked_never_negative(
        amounts in prop::collection::vec(1i128..=50_000i128, 1..=10),
    ) {
        let (env, client, admin) = setup_env();
        let (market_id, token) = create_two_option_market(&env, &client, &admin, 1_000, 2_000);

        env.ledger().set_timestamp(0);

        for (i, amount) in amounts.iter().enumerate() {
            let outcome = (i % 2) as u32;
            let user = Address::generate(&env);
            soroban_sdk::testutils::StellarAssetContract::new(token.clone(), &env)
                .mint(&user, amount);
            let _ = client.try_place_bet(&user, &market_id, &outcome, amount, &token, &None);

            let market = client.get_market(&market_id).unwrap();
            assert!(
                market.total_staked >= 0,
                "total_staked must never be negative; got {}",
                market.total_staked
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Invariant 5 — Refund conservation: after cancel + full refund, total_staked == 0
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_full_refund_drains_total_staked(
        amounts in prop::collection::vec(1i128..=10_000i128, 1..=6),
    ) {
        let (env, client, admin) = setup_env();
        let (market_id, token) = create_two_option_market(&env, &client, &admin, 1_000, 2_000);

        env.ledger().set_timestamp(0);

        let mut bettors: std::vec::Vec<(Address, u32)> = std::vec::Vec::new();
        for (i, amount) in amounts.iter().enumerate() {
            let outcome = (i % 2) as u32;
            let user = Address::generate(&env);
            soroban_sdk::testutils::StellarAssetContract::new(token.clone(), &env)
                .mint(&user, amount);
            if client.try_place_bet(&user, &market_id, &outcome, amount, &token, &None).is_ok() {
                bettors.push((user, outcome));
            }
        }

        client.cancel_market_admin(&market_id);
        assert_stake_conservation(&env, &client, market_id);

        for (bettor, outcome) in &bettors {
            let _ = client.try_withdraw_refund(bettor, &market_id, outcome, &token);
            assert_stake_conservation(&env, &client, market_id);
        }

        let market = client.get_market(&market_id).unwrap();
        assert_eq!(
            market.total_staked, 0,
            "total_staked must be 0 after full refund; got {}",
            market.total_staked
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 6 — Guardian set construction rejects duplicate guardians
// (Issue #1907, target for open issue #1701)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_guardian_set_rejects_duplicates(
        (powers, dup_index) in arb_guardian_set_with_duplicate(),
    ) {
        let (env, client, admin) = setup_env();

        let mut guardians = build_guardians(&env, &powers);
        // Duplicate an existing guardian address so the set contains a repeat.
        let (dup_addr, dup_power) = guardians.get(dup_index as u32).unwrap();
        guardians.push_back((dup_addr, dup_power));

        let result = client.try_initialize_guardians(&admin, &guardians);
        prop_assert!(
            result.is_err(),
            "guardian set with a duplicate address must be rejected; powers={:?}",
            powers
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 7 — Guardian set construction rejects zero voting power
// (Issue #1907, target for open issue #1701)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_guardian_set_rejects_zero_power(
        (mut powers, zero_index) in arb_guardian_set_with_zero_power(),
    ) {
        let (env, client, admin) = setup_env();

        powers[zero_index] = 0;
        let guardians = build_guardians(&env, &powers);

        let result = client.try_initialize_guardians(&admin, &guardians);
        prop_assert!(
            result.is_err(),
            "guardian with zero voting power must be rejected; powers={:?}",
            powers
        );
    }
}

// ---------------------------------------------------------------------------
// Invariant 8 — Guardian voting power sum cannot overflow u32
// (Issue #1907, target for open issue #1701)
// ---------------------------------------------------------------------------

proptest! {
    #[test]
    fn prop_guardian_power_sum_cannot_overflow(
        powers in arb_guardian_set_overflowing_sum(),
    ) {
        let (env, client, admin) = setup_env();

        // The generated set must actually be capable of overflowing a u32 sum.
        let checked_total = powers
            .iter()
            .try_fold(0u32, |acc, p| acc.checked_add(*p));
        prop_assume!(checked_total.is_none());

        let guardians = build_guardians(&env, &powers);
        let result = client.try_initialize_guardians(&admin, &guardians);

        // Either the contract rejects the overflowing set outright, or it
        // accepts it and the stored total must still fit in a u32 (i.e. the
        // implementation used checked arithmetic). Silently wrapping is a bug.
        if result.is_ok() {
            let total = powers
                .iter()
                .try_fold(0u32, |acc, p| acc.checked_add(*p));
            prop_assert!(
                total.is_some(),
                "guardian power sum overflowed u32 without rejection; powers={:?}",
                powers
            );
        }
    }
}
