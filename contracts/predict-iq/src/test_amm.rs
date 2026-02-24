#![cfg(test)]
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{testutils::Address as _, Address, Env};

fn create_test_token(e: &Env, admin: &Address) -> Address {
    e.register_stellar_asset_contract_v2(admin.clone()).address()
}

#[test]
fn test_amm_buy_increases_price() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    let token = create_test_token(&e, &admin);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&e, &token);
    let token_client = soroban_sdk::token::Client::new(&e, &token);

    // Initialize contract
    client.initialize(&admin, &100);

    // Initialize AMM pools for market with 2 outcomes
    let market_id = 1u64;
    let initial_liquidity = 10_000_0000000i128; // 10k USDC
    client.initialize_amm_pools(&market_id, &2, &initial_liquidity);

    // Get initial price
    let price_before = client.get_buy_price(&market_id, &0);

    // User buys shares
    let user = Address::generate(&e);
    token_admin.mint(&user, &1_000_0000000); // Mint 1000 USDC

    let buy_amount = 100_0000000i128; // 100 USDC
    let (_, _) = client.buy_shares(&user, &market_id, &0, &buy_amount, &token);

    // Get price after buy
    let price_after = client.get_buy_price(&market_id, &0);

    // Verify price increased
    assert!(price_after > price_before, "Price should increase after buying");
}

#[test]
fn test_amm_buy_sell_roundtrip_with_slippage() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    let token = create_test_token(&e, &admin);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&e, &token);

    client.initialize(&admin, &100);

    let market_id = 1u64;
    let initial_liquidity = 10_000_0000000i128;
    client.initialize_amm_pools(&market_id, &2, &initial_liquidity);

    let user = Address::generate(&e);
    let initial_balance = 1_000_0000000i128;
    token_admin.mint(&user, &initial_balance);

    // User buys 100 USDC worth of shares
    let buy_amount = 100_0000000i128;
    let (shares_received, _) = client.buy_shares(&user, &market_id, &0, &buy_amount, &token);
    
    assert!(shares_received > 0, "Should receive shares");

    // Verify user shares
    let user_shares = client.get_user_shares(&market_id, &user, &0);
    assert_eq!(user_shares, shares_received);

    // User sells all shares back
    let (usdc_received, _) = client.sell_shares(&user, &market_id, &0, &shares_received, &token);

    // Verify slippage: should receive less than initial due to fees and slippage
    assert!(usdc_received < buy_amount, "Should receive less due to slippage and fees");
    
    // Verify received at least 95% back (5% max slippage + fees)
    let min_expected = (buy_amount * 95) / 100;
    assert!(usdc_received >= min_expected, "Slippage too high");

    // Verify user shares are zero
    let final_shares = client.get_user_shares(&market_id, &user, &0);
    assert_eq!(final_shares, 0);
}

#[test]
fn test_amm_pool_invariant_maintained() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    let token = create_test_token(&e, &admin);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&e, &token);

    client.initialize(&admin, &100);

    let market_id = 1u64;
    let initial_liquidity = 10_000_0000000i128;
    client.initialize_amm_pools(&market_id, &2, &initial_liquidity);

    // Verify initial invariant
    assert!(client.verify_pool_invariant(&market_id, &0), "Initial invariant should hold");

    // Perform multiple trades
    let user1 = Address::generate(&e);
    token_admin.mint(&user1, &1_000_0000000);
    client.buy_shares(&user1, &market_id, &0, &100_0000000, &token);

    // Verify invariant after buy
    assert!(client.verify_pool_invariant(&market_id, &0));

    let user2 = Address::generate(&e);
    token_admin.mint(&user2, &1_000_0000000);
    client.buy_shares(&user2, &market_id, &0, &50_0000000, &token);

    // Verify invariant after another buy
    assert!(client.verify_pool_invariant(&market_id, &0));

    // Sell some shares
    let shares = client.get_user_shares(&market_id, &user1, &0);
    client.sell_shares(&user1, &market_id, &0, &(shares / 2), &token);

    // Verify invariant after sell
    assert!(client.verify_pool_invariant(&market_id, &0));
}

#[test]
fn test_amm_usdc_backing_matches_shares() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    let token = create_test_token(&e, &admin);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&e, &token);

    client.initialize(&admin, &100);

    let market_id = 1u64;
    let initial_liquidity = 10_000_0000000i128;
    client.initialize_amm_pools(&market_id, &2, &initial_liquidity);

    // Get initial pool state
    let pool_before = client.get_amm_pool(&market_id, &0).unwrap();
    let initial_usdc = pool_before.usdc_reserve;

    // Multiple users buy shares
    let user1 = Address::generate(&e);
    token_admin.mint(&user1, &1_000_0000000);
    client.buy_shares(&user1, &market_id, &0, &100_0000000, &token);

    let user2 = Address::generate(&e);
    token_admin.mint(&user2, &1_000_0000000);
    client.buy_shares(&user2, &market_id, &0, &200_0000000, &token);

    // Get pool state after buys
    let pool_after = client.get_amm_pool(&market_id, &0).unwrap();

    // Verify USDC increased by buy amounts (minus fees)
    let expected_min_usdc = initial_usdc + 100_0000000 + 200_0000000 - 1_0000000; // Allow for fees
    assert!(pool_after.usdc_reserve >= expected_min_usdc, "USDC reserve should increase");

    // Verify total shares issued matches user holdings
    let user1_shares = client.get_user_shares(&market_id, &user1, &0);
    let user2_shares = client.get_user_shares(&market_id, &user2, &0);
    assert_eq!(pool_after.total_shares_issued, user1_shares + user2_shares);
}

#[test]
fn test_amm_quote_accuracy() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    let token = create_test_token(&e, &admin);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&e, &token);

    client.initialize(&admin, &100);

    let market_id = 1u64;
    client.initialize_amm_pools(&market_id, &2, &10_000_0000000);

    let user = Address::generate(&e);
    token_admin.mint(&user, &1_000_0000000);

    // Get quote for buying
    let buy_amount = 100_0000000i128;
    let quoted_shares = client.quote_buy(&market_id, &0, &buy_amount);

    // Execute actual buy
    let (actual_shares, _) = client.buy_shares(&user, &market_id, &0, &buy_amount, &token);

    // Quote should match actual (within rounding)
    assert_eq!(quoted_shares, actual_shares, "Quote should match actual buy");

    // Get quote for selling
    let quoted_usdc = client.quote_sell(&market_id, &0, &actual_shares);

    // Execute actual sell
    let (actual_usdc, _) = client.sell_shares(&user, &market_id, &0, &actual_shares, &token);

    // Quote should match actual
    assert_eq!(quoted_usdc, actual_usdc, "Quote should match actual sell");
}

#[test]
#[should_panic(expected = "#107")]
fn test_amm_insufficient_shares_error() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    let token = create_test_token(&e, &admin);

    client.initialize(&admin, &100);

    let market_id = 1u64;
    client.initialize_amm_pools(&market_id, &2, &10_000_0000000);

    let user = Address::generate(&e);

    // Try to sell shares without owning any - should panic with error code 107 (InsufficientBalance)
    client.sell_shares(&user, &market_id, &0, &100_0000000, &token);
}

#[test]
fn test_amm_multiple_outcomes_independent() {
    let e = Env::default();
    e.mock_all_auths();

    let admin = Address::generate(&e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(&e, &contract_id);

    let token = create_test_token(&e, &admin);
    let token_admin = soroban_sdk::token::StellarAssetClient::new(&e, &token);

    client.initialize(&admin, &100);

    let market_id = 1u64;
    client.initialize_amm_pools(&market_id, &3, &15_000_0000000);

    // Get initial prices for all outcomes
    let price0_before = client.get_buy_price(&market_id, &0);
    let price1_before = client.get_buy_price(&market_id, &1);
    let price2_before = client.get_buy_price(&market_id, &2);

    // Buy shares for outcome 0
    let user = Address::generate(&e);
    token_admin.mint(&user, &1_000_0000000);
    client.buy_shares(&user, &market_id, &0, &100_0000000, &token);

    // Check prices after
    let price0_after = client.get_buy_price(&market_id, &0);
    let price1_after = client.get_buy_price(&market_id, &1);
    let price2_after = client.get_buy_price(&market_id, &2);

    // Outcome 0 price should increase
    assert!(price0_after > price0_before, "Outcome 0 price should increase");

    // Other outcomes should remain unchanged
    assert_eq!(price1_after, price1_before, "Outcome 1 price should not change");
    assert_eq!(price2_after, price2_before, "Outcome 2 price should not change");
}
