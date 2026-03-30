// End-to-end integration test for the full disputed market lifecycle
// Active -> PendingResolution -> Disputed -> Resolved -> Claim

use predict_iq::types::{MarketStatus, OracleConfig, MarketTier, Guardian};
use predict_iq::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec, Symbol,
};

mod common;
use common::*;

#[test]
fn test_full_disputed_lifecycle() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    let gov_token_admin = Address::generate(&env);
    let gov_token_id = env.register_stellar_asset_contract_v2(gov_token_admin.clone());
    let gov_token = gov_token_id.address();
    
    let native_token_admin = Address::generate(&env);
    let native_token_id = env.register_stellar_asset_contract_v2(native_token_admin.clone());
    let native_token = native_token_id.address();

    // 1. Initialize contract with guardians
    let guardian_addr = Address::generate(&env);
    let mut guardians = Vec::new(&env);
    guardians.push_back(Guardian {
        address: guardian_addr.clone(),
        voting_power: 100, // 100 power
    });

    client.initialize(&admin, &100);
    client.initialize_guardians(&guardians);
    client.set_governance_token(&gov_token);

    // 2. Create market
    let creator = Address::generate(&env);
    let options = Vec::from_array(
        &env,
        [String::from_str(&env, "Outcome 0"), String::from_str(&env, "Outcome 1")],
    );

    let oracle_config = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: String::from_str(&env, "test_feed"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    };

    env.ledger().with_mut(|li| li.timestamp = 1000);
    let deadline = 2000;
    let resolution_deadline = 3000;

    let market_id = client.create_market(
        &creator,
        &String::from_str(&env, "Full Lifecycle Market"),
        &options,
        &deadline,
        &resolution_deadline,
        &oracle_config,
        &MarketTier::Basic,
        &native_token,
        &0,
        &0,
    );

    // 3. Place bets
    let user_a = Address::generate(&env); // Predicts 0
    let user_b = Address::generate(&env); // Predicts 1
    let token_client = token::StellarAssetClient::new(&env, &native_token);
    token_client.mint(&user_a, &10_000);
    token_client.mint(&user_b, &10_000);

    client.place_bet(&user_a, &market_id, &0, &1_000, &native_token, &None);
    client.place_bet(&user_b, &market_id, &1, &1_000, &native_token, &None);

    assert_market_status(&client, market_id, MarketStatus::Active);

    // 4. Resolve via Oracle (Proposed Outcome: 1)
    env.ledger().with_mut(|li| li.timestamp = 3001); // Past resolution deadline
    
    // Set oracle result as admin
    client.set_oracle_result(&market_id, &1);
    client.attempt_oracle_resolution(&market_id);
    
    assert_market_status(&client, market_id, MarketStatus::PendingResolution);
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.winning_outcome, Some(1));

    // 5. File Dispute (User A disagrees)
    env.ledger().with_mut(|li| li.timestamp = 3100); // Within 48h dispute window
    client.file_dispute(&user_a, &market_id);
    
    assert_market_status(&client, market_id, MarketStatus::Disputed);

    // 6. Community Voting (Majority decides Outcome 0)
    // Setup governance tokens for voters
    let voter_1 = Address::generate(&env);
    let voter_2 = Address::generate(&env);
    let gov_token_client = token::StellarAssetClient::new(&env, &gov_token);
    gov_token_client.mint(&voter_1, &600); // 60% majority
    gov_token_client.mint(&voter_2, &400); // 40%

    // Vote casting
    // Note: weight is deduced from balance in current implementation if balance_at fails
    client.cast_vote(&voter_1, &market_id, &0, &600);
    client.cast_vote(&voter_2, &market_id, &1, &400);

    // 7. Finalize Resolution after voting period (72h after dispute)
    env.ledger().with_mut(|li| li.timestamp = 3100 + (72 * 3601)); // Past 72h
    client.finalize_resolution(&market_id);
    
    assert_market_status(&client, market_id, MarketStatus::Resolved);
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.winning_outcome, Some(0)); // Majority won

    // 8. Claim Winnings (User A was right after all)
    let balance_before = token::Client::new(&env, &native_token).balance(&user_a);
    let claimed = client.claim_winnings(&user_a, &market_id);
    assert!(claimed > 1000); // Original 1000 + share of user B's bet - fees
    
    let balance_after = token::Client::new(&env, &native_token).balance(&user_a);
    assert_eq!(balance_after, balance_before + claimed);

    // 9. Loser (User B) cannot claim
    let b_result = client.try_claim_winnings(&user_b, &market_id);
    assert!(b_result.is_err());
}
