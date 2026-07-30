//! cargo-fuzz target for `place_bet` entry point (Issue #1000).
//!
//! libFuzzer drives the byte corpus; each run parses the raw bytes into the
//! function's parameters and calls the contract.  The harness treats any typed
//! `ErrorCode` return as acceptable — we are hunting for *panics*, not
//! business-logic failures.
#![no_main]

use libfuzzer_sys::fuzz_target;
use predict_iq::{PredictIQ, PredictIQClient};
use predict_iq::types::{MarketTier, OracleConfig};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    token, Address, Env, String as SStr, Vec as SVec,
};

fuzz_target!(|data: &[u8]| {
    // Need at least 13 bytes to derive inputs.
    if data.len() < 13 {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &100); // 1% fee

    // Build a 2-option market with fixed deadlines.
    let options = SVec::from_array(
        &env,
        [SStr::from_str(&env, "A"), SStr::from_str(&env, "B")],
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

    let market_id = client.create_market(
        &admin,
        &SStr::from_str(&env, "Fuzz"),
        &options,
        &1_000u64,
        &2_000u64,
        &oracle,
        &MarketTier::Basic,
        &token_addr,
        &0,
        &0,
    );

    // Derive fuzzed parameters from raw bytes.
    let outcome = (data[0] as u32) % 8; // occasionally out-of-range
    let amount = i128::from_le_bytes({
        let mut b = [0u8; 16];
        b.copy_from_slice(&data[1..17].get(..16).unwrap_or(&[0u8; 16][..]));
        if data.len() >= 17 { b.copy_from_slice(&data[1..17]); }
        b
    });
    let ts_raw = u64::from_le_bytes({
        let mut b = [0u8; 8];
        let slice = if data.len() >= 21 { &data[17..25] } else { &data[data.len()-8..] };
        b.copy_from_slice(&slice[..8.min(slice.len())]);
        b
    });

    env.ledger().set_timestamp(ts_raw % 3_000);

    let bettor = Address::generate(&env);
    if amount > 0 {
        token::StellarAssetClient::new(&env, &token_addr).mint(&bettor, &amount.abs());
    }

    // Must not panic — any typed error is acceptable.
    let _ = client.try_place_bet(&bettor, &market_id, &outcome, &amount, &token_addr, &None);
});
