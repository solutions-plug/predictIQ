#![cfg(test)]
use crate::errors::ErrorCode;
use crate::types::{MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
use crate::types::{Market, MarketStatus, MarketTier, OracleConfig};
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
        max_confidence_bps: 100,
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
    let balance_before = token::Client::new(&env, &token).balance(&user1);
    let winnings = client.claim_winnings(&user1, &market_id, &token);
    let balance_after = token::Client::new(&env, &token).balance(&user1);

    // User1 should get their 1000 back plus share of losing pool (minus fees)
    assert!(winnings > 1000);
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
    assert!(winnings1 > 1000); // More than original bet
    assert!(winnings2 > 2000); // More than original bet
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
    let refund = client.withdraw_refund(&user, &market_id2, &0, &token);
    assert_eq!(refund, 2000);

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

    // Refund outcome 0
    let refund0 = client.withdraw_refund(&user, &market_id, &0, &token);
    assert_eq!(refund0, 1000);

    // Refund outcome 1 — must still be present (not orphaned, not double-removed)
    let refund1 = client.withdraw_refund(&user, &market_id, &1, &token);
    assert_eq!(refund1, 2000);

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

    // Outcome 0 accumulated to 1000
    let refund0 = client.withdraw_refund(&user, &market_id, &0, &token);
    assert_eq!(refund0, 1000);

    // Outcome 1 is independent at 300
    let refund1 = client.withdraw_refund(&user, &market_id, &1, &token);
    assert_eq!(refund1, 300);
}
