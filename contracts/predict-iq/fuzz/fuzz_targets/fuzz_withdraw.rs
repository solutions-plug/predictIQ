//! cargo-fuzz target for `withdraw_refund` entry point (Issue #1000).
//!
//! Places bets on a cancelled market then fuzzes the withdraw_refund call
//! with arbitrary market IDs, ensuring no panics occur regardless of input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use predict_iq::{PredictIQ, PredictIQClient};
use predict_iq::types::{MarketTier, OracleConfig};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, String as SStr, Vec as SVec,
};

fuzz_target!(|data: &[u8]| {
    if data.len() < 9 {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &0);

    let options = SVec::from_array(
        &env,
        [SStr::from_str(&env, "P"), SStr::from_str(&env, "Q")],
    );
    let oracle = OracleConfig {
        oracle_address: Address::generate(&env),
        feed_id: SStr::from_str(&env, "f"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
        strike_price: None,
    };
    let token_admin = Address::generate(&env);
    let token_addr = env
        .register_stellar_asset_contract_v2(token_admin)
        .address();

    let real_market_id = client.create_market(
        &admin,
        &SStr::from_str(&env, "FuzzW"),
        &options,
        &1_000u64,
        &2_000u64,
        &oracle,
        &MarketTier::Basic,
        &token_addr,
        &0,
        &0,
    );

    env.ledger().set_timestamp(0);
    let bettor = Address::generate(&env);
    token::StellarAssetClient::new(&env, &token_addr).mint(&bettor, &5_000i128);
    let _ = client.try_place_bet(&bettor, &real_market_id, &0, &1_000, &token_addr, &None);

    // Cancel the market so withdraw_refund is valid.
    client.cancel_market_admin(&real_market_id);

    // Fuzzed withdrawal inputs.
    let market_id_fuzz = u64::from_le_bytes(data[..8].try_into().unwrap_or([0u8; 8]));
    let market_id = if data[8] & 1 == 0 { real_market_id } else { market_id_fuzz };

    // Must not panic.
    let _ = client.try_withdraw_refund(&bettor, &market_id, &token_addr);
});
