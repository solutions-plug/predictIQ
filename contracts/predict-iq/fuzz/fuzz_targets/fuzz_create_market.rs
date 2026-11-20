#![no_main]

use libfuzzer_sys::fuzz_target;
use predict_iq::{OracleConfig, PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token::{StellarAssetClient, TokenClient},
    Address, Env, String, Vec,
};

// Keep the target fast: cap the number of outcomes and label bytes so each
// run is cheap. These are fuzzing-side caps, not contract constraints.
const MAX_FUZZ_OUTCOMES: u32 = 16;
const MAX_FUZZ_OPTION_BYTES: u32 = 64;
const MAX_FUZZ_DESCRIPTION_BYTES: u32 = 512;

fn make_string(env: &Env, len: u32) -> String {
    let mut s = String::from_str(env, "");
    let mut i = 0u32;
    while i < len {
        s.push_str(&String::from_str(env, "a"));
        i += 1;
    }
    s
}

fuzz_target!(|data: &[u8]| {
    // Need at least a few bytes to derive inputs.
    if data.len() < 4 {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    // Register the contract.
    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    // Initialize with an admin.
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Register a Stellar asset token.
    let token_admin = Address::generate(&env);
    let token_contract = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_contract.address();
    let _token_client = TokenClient::new(&env, &token_address);
    let _token_admin_client = StellarAssetClient::new(&env, &token_address);

    // Derive an outcome count. Use a modulo that exercises both valid and
    // out-of-range counts (MAX_OUTCOMES_PER_MARKET + 2 range).
    // We don't know the exact constant here, so use a generous bound that
    // covers valid and invalid values.
    let max_outcomes_bound: u32 = MAX_FUZZ_OUTCOMES + 2;
    let outcome_count = (data[0] as u32) % max_outcomes_bound;

    // Derive a label length per outcome (bounded).
    let label_len = (data[1] as u32) % (MAX_FUZZ_OPTION_BYTES + 8);

    // Derive a description length (bounded around a reasonable max).
    let description_len = (data[2] as u32) % (MAX_FUZZ_DESCRIPTION_BYTES + 8);

    // Build the options Vec<String> with labels of the derived lengths.
    let mut options: Vec<String> = Vec::new(&env);
    let mut i = 0u32;
    while i < outcome_count {
        options.push_back(make_string(&env, label_len));
        i += 1;
    }

    // Build a valid OracleConfig.
    // 64-char hex feed_id.
    let feed_id = String::from_str(
        &env,
        "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
    );
    // Non-zero max_staleness_seconds.
    let max_staleness_seconds: u64 = 3600;
    // max_confidence_bps <= 10_000.
    let max_confidence_bps: u32 = 500;
    // oracle_address not equal to the contract address.
    let oracle_address = Address::generate(&env);
    let oracle_config = OracleConfig {
        feed_id,
        max_staleness_seconds,
        max_confidence_bps,
        oracle_address,
    };

    // Valid deadlines: current timestamp + 1000 and + 2000, respecting the
    // 86400s minimum gap.
    let now = env.ledger().timestamp();
    let start_time = now + 1000;
    let end_time = now + 1000 + 86400 + 1000;

    let description = make_string(&env, description_len);

    // Call create_market. Treat any typed Err as acceptable; assert the call
    // never panics.
    let result = client.try_create_market(
        &admin,
        &description,
        &options,
        &token_address,
        &start_time,
        &end_time,
        &oracle_config,
    );

    // Any Ok or typed Err is acceptable; a panic would abort the fuzzer.
    let _ = result;
});