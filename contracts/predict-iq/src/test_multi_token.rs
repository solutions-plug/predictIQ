#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _, Ledger};
use soroban_sdk::{Address, Env, Vec, String, token};

/// Test market creation and betting with USDC (6 decimals)
#[test]
fn test_usdc_market_6_decimals() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100);

    // Create USDC token (6 decimals)
    let token_admin = Address::generate(&e);
    let usdc_id = e.register_stellar_asset_contract_v2(token_admin.clone());
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
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &1000,
        &2000,
        &oracle_config,
        &usdc_address,
    );

    // Mint USDC: 1000.000000 (1000 * 10^6)
    let bettor1 = Address::generate(&e);
    let bettor2 = Address::generate(&e);
    usdc_client.mint(&bettor1, &1_000_000_000);
    usdc_client.mint(&bettor2, &500_000_000);

    // Place bets
    client.place_bet(&bettor1, &market_id, &0, &1_000_000_000, &usdc_address);
    client.place_bet(&bettor2, &market_id, &1, &500_000_000, &usdc_address);

    // Verify total_staked
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 1_500_000_000);
    assert_eq!(market.outcome_stakes.get(0).unwrap(), 1_000_000_000);
    assert_eq!(market.outcome_stakes.get(1).unwrap(), 500_000_000);
}

/// Test market creation and betting with XLM (7 decimals)
#[test]
fn test_xlm_market_7_decimals() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100);

    // Create XLM token (7 decimals)
    let token_admin = Address::generate(&e);
    let xlm_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let xlm_address = xlm_id.address();
    let xlm_client = token::StellarAssetClient::new(&e, &xlm_address);

    // Create market
    let creator = Address::generate(&e);
    let description = String::from_str(&e, "ETH > $5k?");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "eth"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &description,
        &options,
        &1000,
        &2000,
        &oracle_config,
        &xlm_address,
    );

    // Mint XLM: 2000.0000000 (2000 * 10^7)
    let bettor1 = Address::generate(&e);
    let bettor2 = Address::generate(&e);
    xlm_client.mint(&bettor1, &20_000_000_000);
    xlm_client.mint(&bettor2, &10_000_000_000);

    // Place bets
    client.place_bet(&bettor1, &market_id, &0, &20_000_000_000, &xlm_address);
    client.place_bet(&bettor2, &market_id, &1, &10_000_000_000, &xlm_address);

    // Verify total_staked
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 30_000_000_000);
    assert_eq!(market.outcome_stakes.get(0).unwrap(), 20_000_000_000);
    assert_eq!(market.outcome_stakes.get(1).unwrap(), 10_000_000_000);
}

/// Test resolution and payout precision with USDC (6 decimals)
#[test]
fn test_usdc_payout_precision() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|li| li.timestamp = 500);

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100); // 1% fee

    // Create USDC token
    let token_admin = Address::generate(&e);
    let usdc_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let usdc_address = usdc_id.address();
    let usdc_client = token::StellarAssetClient::new(&e, &usdc_address);

    // Create market
    let creator = Address::generate(&e);
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(&e, "Test"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &usdc_address,
    );

    // Setup bettors
    let winner1 = Address::generate(&e);
    let winner2 = Address::generate(&e);
    let loser = Address::generate(&e);

    usdc_client.mint(&winner1, &1_000_000_000); // 1000 USDC
    usdc_client.mint(&winner2, &500_000_000);   // 500 USDC
    usdc_client.mint(&loser, &500_000_000);     // 500 USDC

    // Place bets
    client.place_bet(&winner1, &market_id, &0, &1_000_000_000, &usdc_address);
    client.place_bet(&winner2, &market_id, &0, &500_000_000, &usdc_address);
    client.place_bet(&loser, &market_id, &1, &500_000_000, &usdc_address);

    // Resolve market
    e.ledger().with_mut(|li| li.timestamp = 2500);
    let _ = client.resolve_market(&market_id, &0);

    // Claim winnings
    let payout1 = client.claim_winnings(&winner1, &market_id);
    let payout2 = client.claim_winnings(&winner2, &market_id);

    // Verify precision: total pool = 2_000_000_000, fee = 20_000_000 (1%)
    // net_pool = 1_980_000_000
    // winner1: (1_000_000_000 * 1_980_000_000) / 1_500_000_000 = 1_320_000_000
    // winner2: (500_000_000 * 1_980_000_000) / 1_500_000_000 = 660_000_000
    assert_eq!(payout1, 1_320_000_000);
    assert_eq!(payout2, 660_000_000);
}

/// Test resolution and payout precision with XLM (7 decimals)
#[test]
fn test_xlm_payout_precision() {
    let e = Env::default();
    e.mock_all_auths();
    e.ledger().with_mut(|li| li.timestamp = 500);

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100); // 1% fee

    // Create XLM token
    let token_admin = Address::generate(&e);
    let xlm_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let xlm_address = xlm_id.address();
    let xlm_client = token::StellarAssetClient::new(&e, &xlm_address);

    // Create market
    let creator = Address::generate(&e);
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(&e, "Test"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &xlm_address,
    );

    // Setup bettors with precise amounts
    let winner1 = Address::generate(&e);
    let winner2 = Address::generate(&e);
    let loser = Address::generate(&e);

    xlm_client.mint(&winner1, &33_333_333_3); // 3.33333333 XLM
    xlm_client.mint(&winner2, &16_666_666_7); // 1.66666667 XLM
    xlm_client.mint(&loser, &50_000_000_0);   // 5.0 XLM

    // Place bets
    client.place_bet(&winner1, &market_id, &0, &33_333_333_3, &xlm_address);
    client.place_bet(&winner2, &market_id, &0, &16_666_666_7, &xlm_address);
    client.place_bet(&loser, &market_id, &1, &50_000_000_0, &xlm_address);

    // Resolve market
    e.ledger().with_mut(|li| li.timestamp = 2500);
    let _ = client.resolve_market(&market_id, &0);

    // Claim winnings
    let payout1 = client.claim_winnings(&winner1, &market_id);
    let payout2 = client.claim_winnings(&winner2, &market_id);

    // Verify precision to 0.0000001 XLM
    // total = 100_000_000_0, fee = 1_000_000_0 (1%), net = 99_000_000_0
    // winner1: (33_333_333_3 * 99_000_000_0) / 50_000_000_0 = 65_999_999_9
    // winner2: (16_666_666_7 * 99_000_000_0) / 50_000_000_0 = 33_000_000_0
    assert_eq!(payout1, 65_999_999_9);
    assert_eq!(payout2, 33_000_000_0);
}

/// Test that wrong token address is rejected
#[test]
fn test_wrong_token_rejected() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);
    client.initialize(&admin, &100);

    // Create USDC market
    let token_admin = Address::generate(&e);
    let usdc_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let usdc_address = usdc_id.address();

    let creator = Address::generate(&e);
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test"),
        min_responses: 1,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(&e, "Test"),
        &options,
        &1000,
        &2000,
        &oracle_config,
        &usdc_address,
    );

    // Try to bet with wrong token (XLM instead of USDC)
    let xlm_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let xlm_address = xlm_id.address();
    
    let bettor = Address::generate(&e);
    let result = client.try_place_bet(&bettor, &market_id, &0, &1_000_000, &xlm_address);
    
    // Should fail with InvalidBetAmount error
    assert_eq!(result, Err(Ok(ErrorCode::InvalidBetAmount)));
}
