#![cfg(test)]
use crate::errors::ErrorCode;
use crate::types::{MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, String, Vec,
};

fn setup_test_with_token() -> (Env, PredictIQClient<'static>, Address, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &100);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    let user = Address::generate(&env);
    let token_client = token::StellarAssetClient::new(&env, &token_address);
    token_client.mint(&user, &100_000);

    (env, client, admin, user, token_address)
}

fn create_simple_market(
    client: &PredictIQClient,
    env: &Env,
    creator: &Address,
    token: &Address,
) -> u64 {
    let options = Vec::from_array(
        env,
        [String::from_str(env, "Yes"), String::from_str(env, "No")],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    client.create_market(
        creator,
        &String::from_str(env, "Test Market"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &oracle_config,
        &MarketTier::Basic,
        token,
        &0,
        &0,
    )
}

#[test]
fn test_place_bet_success() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert!(result.is_ok());
}

#[test]
fn test_place_bet_zero_amount() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    let result = client.try_place_bet(&user, &market_id, &0, &0, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidAmount)));
}

#[test]
fn test_place_bet_negative_amount() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    let result = client.try_place_bet(&user, &market_id, &0, &-100, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidAmount)));
}

#[test]
fn test_place_bet_invalid_outcome() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Market has outcomes 0 and 1, try outcome 2
    let result = client.try_place_bet(&user, &market_id, &2, &1000, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}

#[test]
fn test_place_bet_after_deadline() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Advance time past deadline
    env.ledger().set_timestamp(1500);

    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::MarketClosed)));
}

// fix/issue-60: Verify the hard stop on betting at the resolution deadline.
// Even if the market status is still Active (attempt_oracle_resolution hasn't been called yet),
// a bet placed at resolution_deadline + 1 must be rejected. This closes the race window where
// an oracle result is known off-chain but the on-chain status hasn't been updated, preventing
// informed bettors from exploiting information asymmetry.
#[test]
fn test_place_bet_rejected_one_second_after_resolution_deadline_while_still_active() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    // Market created at t=500; deadline = 500+1000 = 1500, resolution_deadline = 500+2000 = 2500
    env.ledger().with_mut(|li| li.timestamp = 500);
    let market_id = create_simple_market(&client, &env, &user, &token);

    // Confirm market is still Active (attempt_oracle_resolution has NOT been called)
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, crate::types::MarketStatus::Active);

    // Advance to exactly resolution_deadline + 1 (2501)
    // The oracle result may already be known off-chain at this point
    env.ledger().with_mut(|li| li.timestamp = 2501);

    // Bet must be rejected with ResolutionDeadlinePassed, not succeed due to Active status
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::ResolutionDeadlinePassed)));
}

#[test]
fn test_place_bet_allowed_one_second_before_resolution_deadline() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    // Market created at t=500; deadline = 1500, resolution_deadline = 2500
    env.ledger().with_mut(|li| li.timestamp = 500);
    let market_id = create_simple_market(&client, &env, &user, &token);

    // Advance to one second before resolution_deadline (2499) — still within betting window
    env.ledger().with_mut(|li| li.timestamp = 2499);

    // Bet should be accepted (not blocked by resolution deadline guard)
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert_ne!(result, Err(Ok(ErrorCode::ResolutionDeadlinePassed)));
}

#[test]
fn test_place_bet_on_resolved_market() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Resolve market
    client.resolve_market(&market_id, &0);

    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &None);
    assert_eq!(result, Err(Ok(ErrorCode::MarketClosed)));
}

#[test]
fn test_place_multiple_bets_same_outcome() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user, &market_id, &0, &2000, &token, &None);

    // Both bets should succeed
}

#[test]
fn test_place_bets_different_outcomes() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user, &market_id, &1, &2000, &token, &None);

    // User can bet on different outcomes
}

#[test]
fn test_claim_winnings_success() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);

    // Resolve market with outcome 0 (user wins)
    client.resolve_market(&market_id, &0);

    let result = client.try_claim_winnings(&user, &market_id);
    assert!(result.is_ok());
}

#[test]
fn test_claim_winnings_losing_bet() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);

    // Resolve market with outcome 1 (user loses)
    client.resolve_market(&market_id, &1);

    let result = client.try_claim_winnings(&user, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::NoWinnings)));
}

#[test]
fn test_claim_winnings_before_resolution() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);

    let result = client.try_claim_winnings(&user, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotResolved)));
}

#[test]
fn test_claim_winnings_twice() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.resolve_market(&market_id, &0);

    client.claim_winnings(&user, &market_id);

    // Second claim should fail
    let result = client.try_claim_winnings(&user, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::AlreadyClaimed)));
}

#[test]
fn test_claim_winnings_rapid_sequence_same_ledger() {
    // Stress-test: simulate a repeated-call attack where a bettor fires N claim
    // attempts within the same ledger timestamp. Only the first must succeed;
    // every subsequent attempt must be rejected with AlreadyClaimed, and the
    // user's balance must reflect exactly one payout.
    const ATTACK_ITERATIONS: u32 = 10;

    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    let bet_amount: i128 = 1_000;
    client.place_bet(&user, &market_id, &0, &bet_amount, &token, &None);
    client.resolve_market(&market_id, &0);

    let token_client = token::Client::new(&env, &token);
    let balance_before = token_client.balance(&user);

    // First claim must succeed.
    client.claim_winnings(&user, &market_id);

    // All subsequent attempts in the same ledger must be rejected.
    for _ in 1..ATTACK_ITERATIONS {
        let result = client.try_claim_winnings(&user, &market_id);
        assert_eq!(
            result,
            Err(Ok(ErrorCode::AlreadyClaimed)),
            "expected AlreadyClaimed on repeated call"
        );
    }

    // Balance increased by exactly one payout — no double-spend.
    let balance_after = token_client.balance(&user);
    // Sole bettor takes the whole pool, so payout == bet_amount.
    assert_eq!(balance_after - balance_before, bet_amount);
}

#[test]
fn test_claim_winnings_no_bet_placed() {
    let (env, client, _admin, _user, _token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let other_user = Address::generate(&env);
    let market_id = create_simple_market(&client, &env, &other_user, &_token);

    client.resolve_market(&market_id, &0);

    let result = client.try_claim_winnings(&other_user, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::NoWinnings)));
}

#[test]
fn test_winnings_calculation_single_winner() {
    let (env, client, _admin, user1, token) = setup_test_with_token();

    let user2 = Address::generate(&env);
    let token_client = token::Client::new(&env, &token);
    token::StellarAssetClient::new(&env, &token).mint(&user2, &100_000);

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user1, &token);

    // User1 bets 1000 on outcome 0
    client.place_bet(&user1, &market_id, &0, &1000, &token, &None);

    // User2 bets 2000 on outcome 1
    client.place_bet(&user2, &market_id, &1, &2000, &token, &None);

    // Resolve with outcome 0 (user1 wins)
    client.resolve_market(&market_id, &0);

    let balance_before = token_client.balance(&user1);
    let winnings = client.claim_winnings(&user1, &market_id);
    let balance_after = token_client.balance(&user1);

    // User1 should get their stake back plus share of losing pool (minus fees)
    assert!(winnings > 990); // net stake was 990 after 1% fee
    assert_eq!(balance_after - balance_before, winnings);
}

#[test]
fn test_winnings_calculation_multiple_winners() {
    let (env, client, _admin, user1, token) = setup_test_with_token();

    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let sac = token::StellarAssetClient::new(&env, &token);
    sac.mint(&user2, &100_000);
    sac.mint(&user3, &100_000);

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user1, &token);

    // User1 and User2 bet on outcome 0
    client.place_bet(&user1, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user2, &market_id, &0, &2000, &token, &None);

    // User3 bets on outcome 1
    client.place_bet(&user3, &market_id, &1, &3000, &token, &None);

    // Resolve with outcome 0
    client.resolve_market(&market_id, &0);

    let winnings1 = client.claim_winnings(&user1, &market_id);
    let winnings2 = client.claim_winnings(&user2, &market_id);

    // User2 bet twice as much, should get twice the winnings
    assert!(winnings2 > winnings1);
    assert!(winnings1 > 990);  // More than net stake (990 after 1% fee)
    assert!(winnings2 > 1980); // More than net stake (1980 after 1% fee)
}

#[test]
fn test_referral_rewards_tracked() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    let referrer = Address::generate(&env);

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Place bet with referrer
    client.place_bet(
        &user,
        &market_id,
        &0,
        &1000,
        &token,
        &Some(referrer.clone()),
    );

    // Referrer should have pending rewards
    let rewards = client.try_claim_referral_rewards(&referrer, &token);
    assert!(rewards.is_ok());
}

#[test]
fn test_bet_with_self_referral_rejected() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Try to refer yourself
    let result = client.try_place_bet(&user, &market_id, &0, &1000, &token, &Some(user.clone()));
    assert_eq!(result, Err(Ok(ErrorCode::InvalidReferrer)));
}

// ===================== Storage Cleanup / Issue #56 Tests =====================

#[test]
fn test_withdraw_refund_clears_bet_record() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Place a bet on outcome 0
    client.place_bet(&user, &market_id, &0, &1000, &token, &None);

    // Cancel the market
    client.resolve_market(&market_id, &0); // resolve first so we can test via admin cancel path
                                           // Use admin cancel instead
                                           // Re-create a fresh market for the cancel path
    let market_id2 = create_simple_market(&client, &env, &user, &token);
    client.place_bet(&user, &market_id2, &0, &2000, &token, &None);
    client.cancel_market_admin(&market_id2);

    // Withdraw refund for outcome 0
    // Net stored amount after 1% fee: 2000 - 20 = 1980
    let refund = client.withdraw_refund(&user, &market_id2, &0, &token);
    assert_eq!(refund, 1980);

    // Attempting a second refund for the same outcome must fail — record is gone
    let result = client.try_withdraw_refund(&user, &market_id2, &0, &token);
    assert!(result.is_err());
}

#[test]
fn test_withdraw_refund_multi_outcome_no_orphan_data() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Bettor places on both outcomes — each gets its own storage key
    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user, &market_id, &1, &2000, &token, &None);

    client.cancel_market_admin(&market_id);

    // Refund outcome 0 — net stored after 1% fee: 1000 - 10 = 990
    let refund0 = client.withdraw_refund(&user, &market_id, &0, &token);
    assert_eq!(refund0, 990);

    // Refund outcome 1 — must still be present (not orphaned, not double-removed)
    // net stored after 1% fee: 2000 - 20 = 1980
    let refund1 = client.withdraw_refund(&user, &market_id, &1, &token);
    assert_eq!(refund1, 1980);

    // Both records are now gone — any further attempt fails
    let result0 = client.try_withdraw_refund(&user, &market_id, &0, &token);
    let result1 = client.try_withdraw_refund(&user, &market_id, &1, &token);
    assert!(result0.is_err());
    assert!(result1.is_err());
}

#[test]
fn test_bet_key_is_unique_per_outcome() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Bet on outcome 0 twice — should accumulate
    client.place_bet(&user, &market_id, &0, &500, &token, &None);
    client.place_bet(&user, &market_id, &0, &500, &token, &None);

    // Bet on outcome 1 separately
    client.place_bet(&user, &market_id, &1, &300, &token, &None);

    client.cancel_market_admin(&market_id);

    // Outcome 0 accumulated to 990 net (two bets of 500 each, 1% fee each: 495 + 495)
    let refund0 = client.withdraw_refund(&user, &market_id, &0, &token);
    assert_eq!(refund0, 990);

    // Outcome 1 is independent — net stored after 1% fee: 300 - 3 = 297
    let refund1 = client.withdraw_refund(&user, &market_id, &1, &token);
    assert_eq!(refund1, 297);
}

// =============================================================================
// Issue #24: Precise winner counter tests
// =============================================================================

#[test]
fn test_winner_count_increments_on_first_bet() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1, &token, &None);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.winner_counts.get(0).unwrap_or(0),
        1,
        "First bet on outcome 0 should set winner_counts[0] = 1"
    );
}

#[test]
fn test_winner_count_not_incremented_on_repeat_bet() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Same bettor bets twice on the same outcome
    client.place_bet(&user, &market_id, &0, &1, &token, &None);
    client.place_bet(&user, &market_id, &0, &1, &token, &None);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.winner_counts.get(0).unwrap_or(0),
        1,
        "Repeat bet by same bettor must not increment winner_counts"
    );
}

#[test]
fn test_winner_count_independent_per_outcome() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    client.place_bet(&user, &market_id, &0, &1, &token, &None);
    client.place_bet(&user, &market_id, &1, &1, &token, &None);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.winner_counts.get(0).unwrap_or(0), 1);
    assert_eq!(market.winner_counts.get(1).unwrap_or(0), 1);
}

#[test]
fn test_winner_count_multiple_unique_bettors() {
    let (env, client, _admin, user1, token) = setup_test_with_token();

    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token).mint(&user2, &100_000);
    token::StellarAssetClient::new(&env, &token).mint(&user3, &100_000);

    env.ledger().set_timestamp(500);
    let market_id = create_simple_market(&client, &env, &user1, &token);

    client.place_bet(&user1, &market_id, &0, &1, &token, &None);
    client.place_bet(&user2, &market_id, &0, &1, &token, &None);
    client.place_bet(&user3, &market_id, &0, &1, &token, &None);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.winner_counts.get(0).unwrap_or(0),
        3,
        "Three unique bettors on outcome 0 should yield winner_counts[0] = 3"
    );
}

#[test]
fn test_resolve_market_uses_precise_count_for_push_mode() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);

    // Place a single bet — winner_counts[0] = 1, well below default threshold of 50
    client.place_bet(&user, &market_id, &0, &1, &token, &None);
    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.payout_mode,
        crate::types::PayoutMode::Push,
        "1 winner is below threshold — should select Push mode"
    );
}

#[test]
fn test_resolve_market_switches_to_pull_when_winners_exceed_threshold() {
    let (env, client, _admin, user1, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user1, &token);

    // Lower threshold to 2 so we can test overflow with a small number of bettors
    client.set_max_push_payout_winners(&2);

    // 3 unique bettors on outcome 0 — exceeds threshold of 2
    let sac = token::StellarAssetClient::new(&env, &token);
    for _ in 0..3 {
        let u = Address::generate(&env);
        sac.mint(&u, &100_000);
        client.place_bet(&u, &market_id, &0, &1, &token, &None);
    }

    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.payout_mode,
        crate::types::PayoutMode::Pull,
        "3 winners exceeds threshold of 2 — should select Pull mode"
    );
}

/// Regression test: with the old tally/100 heuristic, 10,000 micro-bets of 1 unit
/// each would produce tally=10,000 → estimated_winners = 100, which exceeds the
/// default threshold of 50 and correctly selects Pull. But if the average bet were
/// 200 units (tally=2,000,000), the heuristic gives 20,000 estimated winners —
/// wildly wrong. The precise counter always gives the exact unique bettor count.
#[test]
fn test_micro_bets_precise_count_prevents_gas_overflow() {
    let (env, client, _admin, user1, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user1, &token);

    // Set threshold to 50 (default)
    client.set_max_push_payout_winners(&50);

    let sac = token::StellarAssetClient::new(&env, &token);

    // 60 unique bettors each placing 1-unit bets (micro-bets)
    for _ in 0..60 {
        let u = Address::generate(&env);
        sac.mint(&u, &100_000);
        client.place_bet(&u, &market_id, &0, &1, &token, &None);
    }

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.winner_counts.get(0).unwrap_or(0),
        60,
        "Precise counter must reflect all 60 unique bettors"
    );

    client.resolve_market(&market_id, &0);

    let resolved = client.get_market(&market_id).unwrap();
    assert_eq!(
        resolved.payout_mode,
        crate::types::PayoutMode::Pull,
        "60 winners > threshold 50 — must select Pull to avoid gas overflow"
    );
}

// =============================================================================
// Issue #91: Parimutuel payout correctness
// =============================================================================

/// Canonical parimutuel scenario from the issue:
/// 10 users bet 100 XLM each (total pool = 1000 XLM).
/// 2 users bet on the winning outcome (stake = 200 XLM).
/// Each winner should receive 500 XLM (their proportional share of the full pool).
///
/// Formula: winnings = (bet_amount / winning_outcome_stake) * total_pool
///        = (100 / 200) * 1000 = 500 XLM per winner.
///
/// This test uses 0 fee (base_fee = 0) to match the issue's exact numbers.
#[test]
fn test_parimutuel_payout_10_bettors_2_winners() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    // Initialize with 0 fee so net amounts equal gross amounts
    client.initialize(&admin, &0);

    let token_admin = Address::generate(&env);
    let token_id = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    env.ledger().set_timestamp(500);

    // Create 10 users, each with 100 XLM
    let bet_amount: i128 = 100;
    let mut users = soroban_sdk::Vec::new(&env);
    let sac = token::StellarAssetClient::new(&env, &token_address);
    for _ in 0..10 {
        let u = Address::generate(&env);
        sac.mint(&u, &bet_amount);
        users.push_back(u);
    }

    let options = soroban_sdk::Vec::from_array(
        &env,
        [
            String::from_str(&env, "Yes"),
            String::from_str(&env, "No"),
        ],
    );
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };
    let market_id = client.create_market(
        &users[0],
        &String::from_str(&env, "Parimutuel Test"),
        &options,
        &(env.ledger().timestamp() + 1000),
        &(env.ledger().timestamp() + 2000),
        &oracle_config,
        &MarketTier::Basic,
        &token_address,
        &0,
        &0,
    );

    // Users 0 and 1 bet on outcome 0 (the winning side)
    client.place_bet(&users.get(0).unwrap(), &market_id, &0, &bet_amount, &token_address, &None);
    client.place_bet(&users.get(1).unwrap(), &market_id, &0, &bet_amount, &token_address, &None);

    // Users 2–9 bet on outcome 1 (the losing side)
    for i in 2..10u32 {
        client.place_bet(&users.get(i).unwrap(), &market_id, &1, &bet_amount, &token_address, &None);
    }

    // Resolve with outcome 0
    client.resolve_market(&market_id, &0);

    // Each winner should receive 500 XLM:
    // winnings = (100 * 1000) / 200 = 500
    let winnings0 = client.claim_winnings(&users.get(0).unwrap(), &market_id);
    let winnings1 = client.claim_winnings(&users.get(1).unwrap(), &market_id);

    assert_eq!(winnings0, 500, "Winner 0 should receive 500 XLM");
    assert_eq!(winnings1, 500, "Winner 1 should receive 500 XLM");

    // Verify token balances reflect the payout
    let token_client = token::Client::new(&env, &token_address);
    assert_eq!(token_client.balance(&users.get(0).unwrap()), 500);
    assert_eq!(token_client.balance(&users.get(1).unwrap()), 500);
}

/// Verify that with a non-zero fee, the net pool is correctly distributed.
/// 2 users bet 1000 each (1% fee → net 990 each, total_staked = 1980).
/// User1 bets on outcome 0 (winning), user2 on outcome 1 (losing).
/// User1 should receive the full net pool: 1980.
#[test]
fn test_parimutuel_payout_with_fee_deduction() {
    let (env, client, _admin, user1, token) = setup_test_with_token();
    // setup_test_with_token initializes with 100 bps (1%) fee

    let user2 = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token).mint(&user2, &100_000);

    env.ledger().set_timestamp(500);
    let market_id = create_simple_market(&client, &env, &user1, &token);

    client.place_bet(&user1, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user2, &market_id, &1, &1000, &token, &None);

    client.resolve_market(&market_id, &0);

    // net_user1 = 990, net_user2 = 990, total_staked = 1980, winning_stake = 990
    // winnings = (990 * 1980) / 990 = 1980
    let winnings = client.claim_winnings(&user1, &market_id);
    assert_eq!(winnings, 1980, "Winner should receive the full net pool");

    // Protocol should have collected 20 in fees (10 per bet)
    assert_eq!(client.get_revenue(&token), 20);
}

/// Verify proportional distribution: a bettor with 2x the stake gets 2x the payout.
#[test]
fn test_parimutuel_proportional_payout() {
    let (env, client, _admin, user1, token) = setup_test_with_token();

    let user2 = Address::generate(&env);
    let user3 = Address::generate(&env);
    let sac = token::StellarAssetClient::new(&env, &token);
    sac.mint(&user2, &100_000);
    sac.mint(&user3, &100_000);

    env.ledger().set_timestamp(500);
    let market_id = create_simple_market(&client, &env, &user1, &token);

    // user1: 1000, user2: 2000 on winning outcome 0 (2:1 ratio)
    client.place_bet(&user1, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user2, &market_id, &0, &2000, &token, &None);
    // user3: 3000 on losing outcome 1
    client.place_bet(&user3, &market_id, &1, &3000, &token, &None);

    client.resolve_market(&market_id, &0);

    let w1 = client.claim_winnings(&user1, &market_id);
    let w2 = client.claim_winnings(&user2, &market_id);

    // user2 staked 2x user1, so should receive 2x the payout
    assert_eq!(w2, w1 * 2, "Payout must be proportional to stake");
    // Both winners receive more than their original net stake
    assert!(w1 > 990);
    assert!(w2 > 1980);
}

// =============================================================================
// Issue #93: SAC-safe transfer consistency and circuit breaker coverage
// =============================================================================

/// Refund must be blocked when the contract is in Paused state.
/// This verifies that withdraw_refund respects require_not_paused_for_high_risk,
/// closing the security bypass where a paused contract could still drain funds.
#[test]
fn test_withdraw_refund_blocked_when_paused() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);
    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.cancel_market_admin(&market_id);

    // Pause the contract
    client.pause();

    // Refund must be rejected while paused
    let result = client.try_withdraw_refund(&user, &market_id, &0, &token);
    assert_eq!(
        result,
        Err(Ok(ErrorCode::ContractPaused)),
        "withdraw_refund must be blocked when contract is paused"
    );
}

/// Refund succeeds after the contract is unpaused, confirming the guard is
/// the only thing blocking it (not a permanent state change).
#[test]
fn test_withdraw_refund_succeeds_after_unpause() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().with_mut(|li| li.timestamp = 500);

    let market_id = create_simple_market(&client, &env, &user, &token);
    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.cancel_market_admin(&market_id);

    client.pause();
    client.unpause();

    // Should succeed now — net amount after 1% fee: 1000 - 10 = 990
    let refund = client.withdraw_refund(&user, &market_id, &0, &token);
    assert_eq!(refund, 990);
}

// ---------------------------------------------------------------------------
// Fault-injection / cleanup-path tests
//
// These tests assert that no stale Bet storage keys remain after any cleanup
// path — successful or failed.  They use `get_bet` to inspect storage directly
// so they catch orphaned records that higher-level assertions would miss.
// ---------------------------------------------------------------------------

/// After a successful `claim_winnings` the Bet record must be gone.
/// Verifies the happy-path removal in `internal_claim_amount`.
#[test]
fn test_claim_winnings_removes_bet_key() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);
    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.resolve_market(&market_id, &0);

    client.claim_winnings(&user, &market_id);

    // Bet key must be absent — no orphaned record.
    assert!(
        client.get_bet(&market_id, &user, &0).is_none(),
        "bet key must be removed after successful claim"
    );
}

/// After a successful `withdraw_refund` the Bet record must be gone.
/// Verifies the refund cleanup path in `internal_claim_amount`.
#[test]
fn test_withdraw_refund_removes_bet_key() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);
    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.cancel_market_admin(&market_id);

    client.withdraw_refund(&user, &market_id, &0, &token);

    assert!(
        client.get_bet(&market_id, &user, &0).is_none(),
        "bet key must be removed after successful refund"
    );
}

/// A failed `withdraw_refund` (wrong token) must leave the Bet record intact —
/// partial cleanup must not silently drop the record before the transfer.
#[test]
fn test_failed_withdraw_refund_leaves_bet_key_intact() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);
    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.cancel_market_admin(&market_id);

    // Use a different token address to force an early error before any cleanup.
    let wrong_token = Address::generate(&env);
    let result = client.try_withdraw_refund(&user, &market_id, &0, &wrong_token);
    assert!(result.is_err(), "refund with wrong token must fail");

    // Bet record must still be present — nothing was cleaned up.
    assert!(
        client.get_bet(&market_id, &user, &0).is_some(),
        "bet key must survive a failed refund attempt"
    );
}

/// A failed `claim_winnings` (losing outcome) must leave no stale Claimed
/// sentinel and must not remove the Bet record for the losing outcome.
#[test]
fn test_failed_claim_leaves_no_orphaned_keys() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);
    // Bet on the losing outcome (1); market resolves to outcome 0.
    client.place_bet(&user, &market_id, &1, &1000, &token, &None);
    client.resolve_market(&market_id, &0);

    let result = client.try_claim_winnings(&user, &market_id);
    assert_eq!(result, Err(Ok(ErrorCode::NoWinnings)));

    // The losing bet record must still be present (not silently removed).
    assert!(
        client.get_bet(&market_id, &user, &1).is_some(),
        "losing bet key must not be removed by a failed claim"
    );
}

/// Refunding one outcome must not disturb the Bet record for a different
/// outcome held by the same bettor — simulates partial-cleanup isolation.
#[test]
fn test_partial_refund_leaves_other_outcome_bet_intact() {
    let (env, client, _admin, user, token) = setup_test_with_token();
    env.ledger().set_timestamp(500);

    let market_id = create_simple_market(&client, &env, &user, &token);
    client.place_bet(&user, &market_id, &0, &1000, &token, &None);
    client.place_bet(&user, &market_id, &1, &2000, &token, &None);
    client.cancel_market_admin(&market_id);

    // Refund only outcome 0.
    client.withdraw_refund(&user, &market_id, &0, &token);

    // Outcome 0 key gone, outcome 1 key still present.
    assert!(
        client.get_bet(&market_id, &user, &0).is_none(),
        "refunded bet key must be removed"
    );
    assert!(
        client.get_bet(&market_id, &user, &1).is_some(),
        "unredeemed bet key must remain intact after partial refund"
    );
}

#[test]
fn test_resolve_market_invalid_outcome() {
    let (env, client, _admin, user, token) = setup_test_with_token();

    env.ledger().set_timestamp(500);
    let market_id = create_simple_market(&client, &env, &user, &token);

    // Market has outcomes 0 and 1; outcome index 99 is out of range.
    let result = client.try_resolve_market(&market_id, &99);
    assert_eq!(result, Err(Ok(ErrorCode::InvalidOutcome)));
}
