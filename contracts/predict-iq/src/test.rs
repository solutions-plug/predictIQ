#![cfg(test)]
use super::*;
use soroban_sdk::testutils::{Address as _};
use soroban_sdk::{Address, Env, Vec, String, token};

#[test]
fn test_market_lifecycle() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);

    client.initialize(&admin, &100); // 1% fee

    let creator = Address::generate(&e);
    let description = String::from_str(&e, "Will BTC reach $100k?");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let deadline = 1000;
    let resolution_deadline = 2000;
    
    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "btc_price"),
        min_responses: 1,
        max_staleness_seconds: 300, // 5 minutes
        max_confidence_bps: 200, // 2%
    };

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    let market_id = client.create_market(&creator, &description, &options, &deadline, &resolution_deadline, &oracle_config, &token_address);
    assert_eq!(market_id, 1);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.id, 1);
    assert_eq!(market.status, types::MarketStatus::Active);
    assert_eq!(market.token_address, token_address);
}

#[test]
fn test_claim_winnings_three_winners() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);
    
    client.initialize(&admin, &100); // 1% fee

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);
    let token_std_client = token::Client::new(&e, &token_address);

    let creator = Address::generate(&e);
    let bettor1 = Address::generate(&e);
    let bettor2 = Address::generate(&e);
    let bettor3 = Address::generate(&e);
    let loser = Address::generate(&e);

    token_client.mint(&bettor1, &1000);
    token_client.mint(&bettor2, &2000);
    token_client.mint(&bettor3, &3000);
    token_client.mint(&loser, &4000);

    let description = String::from_str(&e, "Test Market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test"),
        min_responses: 1,
    };

    let market_id = client.create_market(&creator, &description, &options, &100, &200, &oracle_config, &token_address);

    client.place_bet(&bettor1, &market_id, &0, &1000, &token_address);
    client.place_bet(&bettor2, &market_id, &0, &2000, &token_address);
    client.place_bet(&bettor3, &market_id, &0, &3000, &token_address);
    client.place_bet(&loser, &market_id, &1, &4000, &token_address);

    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, types::MarketStatus::Resolved);
    assert_eq!(market.winning_outcome, Some(0));

    let total_staked = 10000_i128;
    let fee = (total_staked * 100) / 10000;
    let net_pool = total_staked - fee;
    let winning_stake = 6000_i128;

    let payout1 = client.claim_winnings(&bettor1, &market_id);
    let expected1 = (1000 * net_pool) / winning_stake;
    assert_eq!(payout1, expected1);
    assert_eq!(token_std_client.balance(&bettor1), expected1);

    let payout2 = client.claim_winnings(&bettor2, &market_id);
    let expected2 = (2000 * net_pool) / winning_stake;
    assert_eq!(payout2, expected2);
    assert_eq!(token_std_client.balance(&bettor2), expected2);

    let payout3 = client.claim_winnings(&bettor3, &market_id);
    let expected3 = (3000 * net_pool) / winning_stake;
    assert_eq!(payout3, expected3);
    assert_eq!(token_std_client.balance(&bettor3), expected3);
}

#[test]
#[should_panic(expected = "Error(Contract, #121)")]
fn test_claim_winnings_double_claim() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);
    
    client.initialize(&admin, &100);

    let token_admin = Address::generate(&e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let token_client = token::StellarAssetClient::new(&e, &token_address);

    let creator = Address::generate(&e);
    let bettor = Address::generate(&e);

    token_client.mint(&bettor, &1000);

    let description = String::from_str(&e, "Test Market");
    let mut options = Vec::new(&e);
    options.push_back(String::from_str(&e, "Yes"));
    options.push_back(String::from_str(&e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(&e),
        feed_id: String::from_str(&e, "test"),
        min_responses: 1,
    };

    let market_id = client.create_market(&creator, &description, &options, &100, &200, &oracle_config, &token_address);
    client.place_bet(&bettor, &market_id, &0, &1000, &token_address);
    client.resolve_market(&market_id, &0);

    client.claim_winnings(&bettor, &market_id);
    client.claim_winnings(&bettor, &market_id);
}
