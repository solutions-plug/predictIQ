#![no_main]

use libfuzzer_sys::fuzz_target;
use predict_iq::{PredictIQContract, PredictIQContractClient};
use soroban_sdk::{testutils::Address as _, token, Address, Env, Vec};

/// Fuzz `bets::claim_winnings` parimutuel payout math.
///
/// Exercises varied bet amounts and winning/losing outcome stakes and asserts
/// that claiming never panics and that the total payouts never exceed the
/// market's `total_staked`.
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

    // Derive a small number of outcomes and bets from the fuzz input.
    let outcome_count = 2 + (data[0] as u32 % 3);
    let mut outcomes: Vec<u32> = Vec::new(&env);
    for i in 0..outcome_count {
        outcomes.push_back(i);
    }

    let market_id = client.create_market(&admin, &outcomes, &1_000_000u64);

    // Place a handful of bets with amounts derived from the fuzz bytes.
    let bet_count = 1 + (data[1] as u32 % 6);
    let mut bettors: Vec<Address> = Vec::new(&env);
    let mut total_staked: i128 = 0;
    for i in 0..bet_count {
        let bettor = Address::generate(&env);
        let amount = 1 + (data[(2 + i as usize) % data.len()] as i128) * 1_000;
        let outcome = (data[(3 + i as usize) % data.len()] as u32) % outcome_count;
        client.place_bet(&bettor, &market_id, &outcome, &amount);
        bettors.push_back(bettor);
        total_staked += amount;
    }

    // Resolve to one of the outcomes.
    let winning_outcome = (data[data.len() - 1] as u32) % outcome_count;
    client.resolve_market(&admin, &market_id, &winning_outcome);

    // Claim for every bettor; assert no panic and payout invariants.
    let mut total_paid: i128 = 0;
    for bettor in bettors.iter() {
        let payout = client.claim_winnings(&bettor, &market_id);
        assert!(payout >= 0, "payout must be non-negative");
        total_paid += payout;
    }

    // Total payouts must never exceed the market's total staked amount.
    assert!(
        total_paid <= total_staked,
        "total payouts exceed total_staked"
    );
});
