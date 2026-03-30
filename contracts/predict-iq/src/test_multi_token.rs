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
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
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
    client.place_bet(&bettor1, &market_id, &0, &1_000_000_000, &usdc_address, &None);
    client.place_bet(&bettor2, &market_id, &1, &500_000_000, &usdc_address, &None);

    // Verify total_staked — net amounts after 1% fee
    // 1_000_000_000 - 10_000_000 = 990_000_000
    // 500_000_000 - 5_000_000 = 495_000_000
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 1_485_000_000);
    assert_eq!(client.get_outcome_stake(&market_id, &0), 990_000_000);
    assert_eq!(client.get_outcome_stake(&market_id, &1), 495_000_000);
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
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
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
    client.place_bet(&bettor1, &market_id, &0, &20_000_000_000, &xlm_address, &None);
    client.place_bet(&bettor2, &market_id, &1, &10_000_000_000, &xlm_address, &None);

    // Verify total_staked — net amounts after 1% fee
    // 20_000_000_000 - 200_000_000 = 19_800_000_000
    // 10_000_000_000 - 100_000_000 = 9_900_000_000
    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.total_staked, 29_700_000_000);
    assert_eq!(client.get_outcome_stake(&market_id, &0), 19_800_000_000);
    assert_eq!(client.get_outcome_stake(&market_id, &1), 9_900_000_000);
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
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
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
    client.place_bet(&winner1, &market_id, &0, &1_000_000_000, &usdc_address, &None);
    client.place_bet(&winner2, &market_id, &0, &500_000_000, &usdc_address, &None);
    client.place_bet(&loser, &market_id, &1, &500_000_000, &usdc_address, &None);

    // Resolve market
    e.ledger().with_mut(|li| li.timestamp = 2500);
    let _ = client.resolve_market(&market_id, &0);

    // Claim winnings
    let payout1 = client.claim_winnings(&winner1, &market_id);
    let payout2 = client.claim_winnings(&winner2, &market_id);

    // Verify precision with net amounts after 1% fee:
    // winner1 net: 1_000_000_000 - 10_000_000 = 990_000_000
    // winner2 net: 500_000_000 - 5_000_000 = 495_000_000
    // loser net:   500_000_000 - 5_000_000 = 495_000_000
    // total_staked = 1_980_000_000, winning_stake = 1_485_000_000
    // payout1 = (990_000_000 * 1_980_000_000) / 1_485_000_000 = 1_320_000_000
    // payout2 = (495_000_000 * 1_980_000_000) / 1_485_000_000 = 660_000_000
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
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
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
    client.place_bet(&winner1, &market_id, &0, &33_333_333_3, &xlm_address, &None);
    client.place_bet(&winner2, &market_id, &0, &16_666_666_7, &xlm_address, &None);
    client.place_bet(&loser, &market_id, &1, &50_000_000_0, &xlm_address, &None);

    // Resolve market
    e.ledger().with_mut(|li| li.timestamp = 2500);
    let _ = client.resolve_market(&market_id, &0);

    // Claim winnings
    let payout1 = client.claim_winnings(&winner1, &market_id);
    let payout2 = client.claim_winnings(&winner2, &market_id);

    // Verify precision with net amounts after 1% fee:
    // winner1 net: 333333333 - 3333333 = 330000000
    // winner2 net: 166666667 - 1666666 = 165000001
    // loser net:   500000000 - 5000000 = 495000000
    // total_staked = 990000001, winning_stake = 495000001
    // payout1 = (330000000 * 990000001) / 495000001 = 659999999
    // payout2 = (165000001 * 990000001) / 495000001 = 330000001
    assert_eq!(payout1, 659_999_999);
    assert_eq!(payout2, 330_000_001);
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
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
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
    let result = client.try_place_bet(&bettor, &market_id, &0, &1_000_000, &xlm_address, &None);
    
    // Should fail with InvalidBetAmount error
    assert_eq!(result, Err(Ok(ErrorCode::InvalidBetAmount)));
}
