// Full end-to-end integration test for bet-place → resolve → claim payout flow
// Tests the complete happy path with fee deduction and referral rewards

use predict_iq::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};

mod common;
use common::*;

#[test]
fn test_full_bet_place_resolve_claim_flow() {
    let (env, client, admin, token) = setup_with_token();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    // 1. Create market
    let market_id = create_market(&client, &env, &admin, &token);
    assert_eq!(market_id, 1);

    // 2. Setup users with balances
    let bettor1 = setup_user_with_balance(&env, &token, 50_000);
    let bettor2 = setup_user_with_balance(&env, &token, 50_000);
    let bettor3 = setup_user_with_balance(&env, &token, 50_000);
    let token_client = token::StellarAssetClient::new(&env, &token);

    // 3. Place bets on different outcomes
    let bet1_amount = 10_000i128;
    let bet2_amount = 15_000i128;
    let bet3_amount = 20_000i128;

    client.place_bet(&bettor1, &market_id, &0, &bet1_amount, &token, &None);
    client.place_bet(&bettor2, &market_id, &0, &bet2_amount, &token, &None);
    client.place_bet(&bettor3, &market_id, &1, &bet3_amount, &token, &None);

    // Verify balances after betting
    let balance1_after_bet = token_client.balance(&bettor1);
    let balance2_after_bet = token_client.balance(&bettor2);
    let balance3_after_bet = token_client.balance(&bettor3);

    assert_eq!(balance1_after_bet, 50_000 - bet1_amount);
    assert_eq!(balance2_after_bet, 50_000 - bet2_amount);
    assert_eq!(balance3_after_bet, 50_000 - bet3_amount);

    // 4. Verify market state
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, predict_iq::types::MarketStatus::Active);

    // 5. Advance time to resolution deadline
    advance_time(&env, 2100);

    // 6. Resolve market with outcome 0 (bettor1 and bettor2 win)
    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, predict_iq::types::MarketStatus::Resolved);

    // 7. Winners claim payouts
    let winnings1 = client.claim_winnings(&bettor1, &market_id, &token);
    let winnings2 = client.claim_winnings(&bettor2, &market_id, &token);

    // Verify winnings are greater than original bets (includes pool share)
    assert!(
        winnings1 > bet1_amount,
        "Winnings should exceed original bet"
    );
    assert!(
        winnings2 > bet2_amount,
        "Winnings should exceed original bet"
    );

    // 8. Verify balances after claiming
    let balance1_after_claim = token_client.balance(&bettor1);
    let balance2_after_claim = token_client.balance(&bettor2);

    assert_eq!(balance1_after_claim, 50_000 - bet1_amount + winnings1);
    assert_eq!(balance2_after_claim, 50_000 - bet2_amount + winnings2);

    // 9. Loser cannot claim
    let result = client.try_claim_winnings(&bettor3, &market_id, &token);
    assert_eq!(result, Err(Ok(predict_iq::errors::ErrorCode::NoWinnings)));

    // Verify loser's balance unchanged
    let balance3_after_claim = token_client.balance(&bettor3);
    assert_eq!(balance3_after_claim, 50_000 - bet3_amount);

    // 10. Verify fee collection
    let revenue = client.get_revenue(&token);
    assert!(revenue > 0, "Revenue should be collected from fees");
}

#[test]
fn test_full_flow_with_referral_rewards() {
    let (env, client, admin, token) = setup_with_token();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = create_market(&client, &env, &admin, &token);

    // Setup users
    let bettor = setup_user_with_balance(&env, &token, 50_000);
    let referrer = setup_user_with_balance(&env, &token, 0);
    let token_client = token::StellarAssetClient::new(&env, &token);

    // Place bet with referrer
    let bet_amount = 10_000i128;
    client.place_bet(
        &bettor,
        &market_id,
        &0,
        &bet_amount,
        &token,
        &Some(referrer.clone()),
    );

    // Resolve market with bettor winning
    advance_time(&env, 2100);
    client.resolve_market(&market_id, &0);

    // Bettor claims winnings
    let winnings = client.claim_winnings(&bettor, &market_id, &token);
    assert!(winnings > bet_amount);

    // Referrer should have earned rewards
    let referrer_balance = token_client.balance(&referrer);
    assert!(referrer_balance > 0, "Referrer should have earned rewards");
}

#[test]
fn test_full_flow_fee_deduction_verification() {
    let (env, client, admin, token) = setup_with_token();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    // Set base fee to 5%
    client.set_base_fee(&500);

    let market_id = create_market(&client, &env, &admin, &token);

    let bettor1 = setup_user_with_balance(&env, &token, 100_000);
    let bettor2 = setup_user_with_balance(&env, &token, 100_000);
    let token_client = token::StellarAssetClient::new(&env, &token);

    // Place equal bets on opposite outcomes
    let bet_amount = 10_000i128;
    client.place_bet(&bettor1, &market_id, &0, &bet_amount, &token, &None);
    client.place_bet(&bettor2, &market_id, &1, &bet_amount, &token, &None);

    // Resolve market
    advance_time(&env, 2100);
    client.resolve_market(&market_id, &0);

    // Bettor1 claims winnings
    let winnings = client.claim_winnings(&bettor1, &market_id, &token);

    // Verify fee was deducted: winnings should be less than total pool
    // Total pool = 20_000, minus 5% fee = 19_000
    // Bettor1 gets share of remaining pool
    assert!(
        winnings < 20_000,
        "Winnings should be less than total pool due to fees"
    );
    assert!(
        winnings > bet_amount,
        "Winnings should still exceed original bet"
    );

    // Verify revenue was collected
    let revenue = client.get_revenue(&token);
    assert!(revenue > 0, "Revenue should be collected");
}

#[test]
fn test_full_flow_multiple_markets_sequential() {
    let (env, client, admin, token) = setup_with_token();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let user = setup_user_with_balance(&env, &token, 200_000);
    let token_client = token::StellarAssetClient::new(&env, &token);

    // Create and resolve multiple markets sequentially
    for i in 0..3 {
        let market_id = create_market(&client, &env, &admin, &token);
        assert_eq!(market_id, (i + 1) as u64);

        // Place bet
        let bet_amount = 10_000i128;
        client.place_bet(&user, &market_id, &0, &bet_amount, &token, &None);

        // Resolve market
        advance_time(&env, 2100);
        client.resolve_market(&market_id, &0);

        // Claim winnings
        let winnings = client.claim_winnings(&user, &market_id, &token);
        assert!(winnings > bet_amount);
    }

    // Verify final balance is greater than initial minus all bets
    let final_balance = token_client.balance(&user);
    assert!(final_balance > 200_000 - (3 * 10_000));
}

#[test]
fn test_full_flow_with_multiple_winners() {
    let (env, client, admin, token) = setup_with_token();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = create_market(&client, &env, &admin, &token);

    // Create 5 winners and 5 losers
    let winners = create_users(&env, &token, 5, 50_000);
    let losers = create_users(&env, &token, 5, 50_000);
    let token_client = token::StellarAssetClient::new(&env, &token);

    // Winners bet on outcome 0
    for winner in winners.iter() {
        client.place_bet(&winner, &market_id, &0, &5_000, &token, &None);
    }

    // Losers bet on outcome 1
    for loser in losers.iter() {
        client.place_bet(&loser, &market_id, &1, &5_000, &token, &None);
    }

    // Resolve market with outcome 0
    advance_time(&env, 2100);
    client.resolve_market(&market_id, &0);

    // All winners claim
    let mut total_winnings = 0i128;
    for winner in winners.iter() {
        let winnings = client.claim_winnings(&winner, &market_id, &token);
        assert!(winnings > 5_000);
        total_winnings += winnings;
    }

    // Verify total winnings is less than total pool (due to fees)
    let total_pool = 50_000i128; // 5 winners * 5_000 + 5 losers * 5_000
    assert!(total_winnings < total_pool);

    // Verify losers cannot claim
    for loser in losers.iter() {
        let result = client.try_claim_winnings(&loser, &market_id, &token);
        assert_eq!(result, Err(Ok(predict_iq::errors::ErrorCode::NoWinnings)));
    }
}

#[test]
fn test_full_flow_payout_distribution_accuracy() {
    let (env, client, admin, token) = setup_with_token();
    env.ledger().with_mut(|li| li.timestamp = 1000);

    let market_id = create_market(&client, &env, &admin, &token);

    // Create users with specific bet amounts
    let bettor1 = setup_user_with_balance(&env, &token, 100_000);
    let bettor2 = setup_user_with_balance(&env, &token, 100_000);
    let bettor3 = setup_user_with_balance(&env, &token, 100_000);

    // Place bets with different amounts
    let bet1 = 10_000i128;
    let bet2 = 20_000i128;
    let bet3 = 30_000i128;

    client.place_bet(&bettor1, &market_id, &0, &bet1, &token, &None);
    client.place_bet(&bettor2, &market_id, &0, &bet2, &token, &None);
    client.place_bet(&bettor3, &market_id, &1, &bet3, &token, &None);

    // Resolve market
    advance_time(&env, 2100);
    client.resolve_market(&market_id, &0);

    // Claim winnings
    let winnings1 = client.claim_winnings(&bettor1, &market_id, &token);
    let winnings2 = client.claim_winnings(&bettor2, &market_id, &token);

    // Verify proportional distribution
    // Bettor1 bet 10_000 out of 30_000 total winning bets (1/3)
    // Bettor2 bet 20_000 out of 30_000 total winning bets (2/3)
    // So winnings2 should be approximately 2x winnings1
    let ratio = winnings2 as f64 / winnings1 as f64;
    assert!(
        ratio > 1.8 && ratio < 2.2,
        "Payout ratio should be approximately 2:1"
    );
}
