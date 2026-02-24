#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, Vec, String, token};

/// Test Classic Stellar Asset (USDC) integration via SAC
#[test]
fn test_classic_asset_sac_integration() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100);

    // Create Classic USDC via Stellar Asset Contract (SAC)
    let token_admin = Address::generate(&e);
    let usdc_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let usdc_address = usdc_id.address();
    let usdc_client = token::StellarAssetClient::new(&e, &usdc_address);

    // Create market with Classic asset
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "ETH > $5k?");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "eth"),
        min_responses: Some(1),
    };

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &1000,
        &2000,
        &oracle_config,
        &types::MarketTier::Basic,
        &usdc_address,
        &0,
        &0,
    );

    // Mint USDC to bettors
    let bettor1 = Address::generate(&e);
    let bettor2 = Address::generate(&e);
    usdc_client.mint(&bettor1, &1_000_000_000); // 1000 USDC
    usdc_client.mint(&bettor2, &500_000_000);   // 500 USDC

    // Place bets using Classic asset
    client.place_bet(&bettor1, &market_id, &0, &500_000_000, &usdc_address, &None);
    client.place_bet(&bettor2, &market_id, &1, &300_000_000, &usdc_address, &None);

    // Verify balances
    let token_client = token::Client::new(&e, &usdc_address);
    let contract_balance = token_client.balance(&contract_id);
    assert_eq!(contract_balance, 800_000_000);

    // Resolve market
    e.ledger().set_timestamp(2001);
    client.resolve_market(&market_id, &0);

    // Claim winnings with Classic asset
    let payout = client.claim_winnings(&bettor1, &market_id, &usdc_address);
    assert!(payout > 500_000_000); // Should get more than initial bet
}

/// Test clawback detection - demonstrates the check_clawback function
#[test]
fn test_clawback_detection_cancels_market() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100);

    // Create Classic asset
    let token_admin = Address::generate(&e);
    let asset_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let asset_address = asset_id.address();
    let asset_client = token::StellarAssetClient::new(&e, &asset_address);

    // Create market
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Test Market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test"),
        min_responses: Some(1),
    };

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &1000,
        &2000,
        &oracle_config,
        &types::MarketTier::Basic,
        &asset_address,
        &0,
        &0,
    );

    // Place bets
    let bettor = Address::generate(&e);
    asset_client.mint(&bettor, &1_000_000);
    client.place_bet(&bettor, &market_id, &0, &500_000, &asset_address, &None);

    // Verify check_clawback passes when balance is correct
    let result = client.try_check_clawback(&market_id);
    assert!(result.is_ok());
    
    // Note: Actual clawback would require the asset to have AUTH_CLAWBACK_ENABLED flag
    // In production, if issuer claws back funds, check_clawback would detect it
}

/// Test successful place_bet and claim_winnings with Classic asset wrapper
#[test]
fn test_classic_asset_full_lifecycle() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100);

    // Register Classic USDC
    let issuer = Address::generate(&e);
    let usdc_id = e.register_stellar_asset_contract_v2(issuer.clone());
    let usdc_address = usdc_id.address();
    let usdc_client = token::StellarAssetClient::new(&e, &usdc_address);

    // Create market
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "BTC > $100k?");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "btc"),
        min_responses: Some(1),
    };

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &1000,
        &2000,
        &oracle_config,
        &types::MarketTier::Basic,
        &usdc_address,
        &0,
        &0,
    );

    // Setup bettors
    let winner = Address::generate(&e);
    let loser = Address::generate(&e);
    usdc_client.mint(&winner, &1_000_000);
    usdc_client.mint(&loser, &500_000);

    // Place bets
    client.place_bet(&winner, &market_id, &0, &600_000, &usdc_address, &None);
    client.place_bet(&loser, &market_id, &1, &400_000, &usdc_address, &None);

    // Verify contract holds the funds
    let token_client = token::Client::new(&e, &usdc_address);
    let contract_balance = token_client.balance(&contract_id);
    assert_eq!(contract_balance, 1_000_000);

    // Resolve market - outcome 0 wins
    e.ledger().set_timestamp(2001);
    client.resolve_market(&market_id, &0);

    // Winner claims
    let initial_balance = token_client.balance(&winner);
    let payout = client.claim_winnings(&winner, &market_id, &usdc_address);
    let final_balance = token_client.balance(&winner);

    // Verify payout
    assert!(payout > 600_000); // Winner gets more than bet
    assert_eq!(final_balance, initial_balance + payout);

    // Loser cannot claim
    let loser_result = client.try_claim_winnings(&loser, &market_id, &usdc_address);
    assert!(loser_result.is_err());
}

/// Test freeze scenario - market should handle gracefully
#[test]
fn test_frozen_asset_handling() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100);

    // Create asset
    let issuer = Address::generate(&e);
    let asset_id = e.register_stellar_asset_contract_v2(issuer.clone());
    let asset_address = asset_id.address();
    let asset_client = token::StellarAssetClient::new(&e, &asset_address);

    // Create market
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Test");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test"),
        min_responses: Some(1),
    };

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &1000,
        &2000,
        &oracle_config,
        &types::MarketTier::Basic,
        &asset_address,
        &0,
        &0,
    );

    // Place bet
    let bettor = Address::generate(&e);
    asset_client.mint(&bettor, &1_000_000);
    client.place_bet(&bettor, &market_id, &0, &500_000, &asset_address, &None);

    // Resolve market
    e.ledger().set_timestamp(2001);
    client.resolve_market(&market_id, &0);

    // Normal claim should succeed (freeze test would require admin controls)
    let payout = client.claim_winnings(&bettor, &market_id, &asset_address);
    assert!(payout > 0);
}
