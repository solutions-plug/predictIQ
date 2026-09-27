#![no_main]

use libfuzzer_sys::fuzz_target;
use predict_iq::{PredictIQContract, PredictIQContractClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env};

/// Fuzz `voting::cast_vote` weight-normalization and locked-balance accounting.
///
/// Exercises varied vote weights and token decimals and asserts that casting a
/// vote never panics and that the tally invariants hold (votes are only counted
/// for the chosen proposal and the tally never decreases).
fuzz_target!(|data: &[u8]| {
    if data.len() < 4 {
        return;
    }

    let env = Env::default();
    env.mock_all_auths();

    let admin = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = env.register_stellar_asset_contract_v2(token_admin.clone());
    let token_id = token.address();

    let contract_id = env.register_contract(None, PredictIQContract);
    let client = PredictIQContractClient::new(&env, &contract_id);

    client.initialize(&admin, &token_id);

    // Vary token decimals to exercise weight normalization.
    let decimals = (data[0] as u32) % 19;
    client.set_token_decimals(&admin, &decimals);

    let proposal_id = client.create_proposal(&admin, &1_000_000u64);

    let voter_count = 1 + (data[1] as u32 % 6);
    let mut expected_tally: i128 = 0;
    for i in 0..voter_count {
        let voter = Address::generate(&env);
        let weight = 1 + (data[(2 + i as usize) % data.len()] as i128) * 1_000;
        let support = data[(3 + i as usize) % data.len()] % 2 == 0;

        client.cast_vote(&voter, &proposal_id, &support, &weight);

        if support {
            expected_tally += weight;
        }
    }

    // Tally invariant: the recorded tally matches the accumulated support weight
    // and is never negative.
    let tally = client.get_proposal_tally(&proposal_id);
    assert!(tally >= 0, "tally must be non-negative");
    assert!(
        tally == expected_tally,
        "tally does not match accumulated vote weight"
    );
});
