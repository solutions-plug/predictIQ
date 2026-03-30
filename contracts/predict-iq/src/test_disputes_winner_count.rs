#![cfg(test)]
use crate::*;
use soroban_sdk::testutils::Address as _;
use soroban_sdk::{token, Address, Env, String, Vec};

// ── Shared helpers ────────────────────────────────────────────────────────────

fn setup() -> (Env, Address, Address, PredictIQClient<'static>) {
    let e = Env::default();
    e.mock_all_auths();
    e.budget().reset_unlimited();

    let admin = Address::generate(&e);
    let contract_id = e.register_contract(None, PredictIQ);
    let client = PredictIQClient::new(&e, &contract_id);
    let mut guardians = Vec::new(&e);
    guardians.push_back(types::Guardian {
        address: Address::generate(&e),
        voting_power: 1,
    });
    client.initialize(&admin, &0, &guardians);

    (e, admin, contract_id, client)
}

/// Create a 2-outcome market and return (market_id, token_address).
fn create_market(
    client: &PredictIQClient,
    e: &Env,
    deadline: u64,
    resolution_deadline: u64,
) -> (u64, Address) {
    let creator = Address::generate(e);
    let token_admin = Address::generate(e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();

    let mut options = Vec::new(e);
    options.push_back(String::from_str(e, "Yes"));
    options.push_back(String::from_str(e, "No"));

    let oracle_config = types::OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "feed"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };

    let market_id = client.create_market(
        &creator,
        &String::from_str(e, "Test"),
        &options,
        &deadline,
        &resolution_deadline,
        &oracle_config,
        &types::MarketTier::Basic,
        &token_address,
        &0u64,
        &0u32,
    );

    (market_id, token_address)
}

/// Place a bet and return the bettor address.
fn place_bet(
    client: &PredictIQClient,
    e: &Env,
    market_id: u64,
    outcome: u32,
    amount: i128,
    token_address: &Address,
) -> Address {
    let token_admin = Address::generate(e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    // Mint on the market token, not a new one — we need the market's token.
    // Use StellarAssetClient on the market token.
    let bettor = Address::generate(e);
    let stellar = token::StellarAssetClient::new(e, token_address);
    stellar.mint(&bettor, &amount);
    client.place_bet(&bettor, &market_id, &outcome, &amount, token_address, &None);
    bettor
}

// ── Tests ─────────────────────────────────────────────────────────────────────

/// Heuristic divergence proof: the old `tally/100` formula returns 0 for a
/// market with 3 bettors each staking 10 tokens (total tally = 0 because
/// voting tallies are separate from bet stakes).  The exact counter must
/// return 3.
///
/// This test would have PASSED with the heuristic (returning 0 ≠ 3) and
/// now FAILS if the exact counter is broken — proving the migration is live.
#[test]
fn test_exact_count_matches_actual_bettor_count_not_heuristic() {
    let (e, _admin, _, client) = setup();

    let deadline = 1000u64;
    let resolution_deadline = 2000u64;
    let (market_id, token_address) = create_market(&client, &e, deadline, resolution_deadline);

    // 3 distinct bettors on outcome 0, each with a small stake (10 units).
    // Old heuristic: voting_tally(outcome 0) == 0 → estimate = 0.
    // Exact counter: winner_counts[0] == 3.
    for _ in 0..3 {
        place_bet(&client, &e, market_id, 0, 10, &token_address);
    }

    let metrics = client.get_resolution_metrics(&market_id, &0);
    assert_eq!(
        metrics.winner_count, 3,
        "exact counter must equal the number of distinct bettors, not tally/100"
    );
}

/// A single bettor placing multiple bets on the same outcome is counted once.
#[test]
fn test_repeated_bets_by_same_bettor_counted_once() {
    let (e, _admin, _, client) = setup();

    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    let bettor = Address::generate(&e);
    let stellar = token::StellarAssetClient::new(&e, &token_address);
    stellar.mint(&bettor, &300);

    // Three separate place_bet calls from the same address.
    client.place_bet(&bettor, &market_id, &0, &100, &token_address, &None);
    client.place_bet(&bettor, &market_id, &0, &100, &token_address, &None);
    client.place_bet(&bettor, &market_id, &0, &100, &token_address, &None);

    let metrics = client.get_resolution_metrics(&market_id, &0);
    assert_eq!(
        metrics.winner_count, 1,
        "same bettor placing multiple bets must still count as 1 winner"
    );
}

/// Bets on different outcomes are tracked independently.
#[test]
fn test_winner_counts_are_per_outcome() {
    let (e, _admin, _, client) = setup();

    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    // 2 bettors on outcome 0, 1 bettor on outcome 1.
    place_bet(&client, &e, market_id, 0, 50, &token_address);
    place_bet(&client, &e, market_id, 0, 50, &token_address);
    place_bet(&client, &e, market_id, 1, 50, &token_address);

    assert_eq!(client.get_resolution_metrics(&market_id, &0).winner_count, 2);
    assert_eq!(client.get_resolution_metrics(&market_id, &1).winner_count, 1);
}

/// resolve_market selects Push mode when winner_count ≤ max_push_payout_winners.
/// With the heuristic this was unreliable; with exact counting it is deterministic.
#[test]
fn test_resolve_market_selects_push_mode_when_winners_within_threshold() {
    let (e, _admin, _, client) = setup();

    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    // 3 bettors on outcome 0 — well within the default threshold of 50.
    place_bet(&client, &e, market_id, 0, 100, &token_address);
    place_bet(&client, &e, market_id, 0, 100, &token_address);
    place_bet(&client, &e, market_id, 0, 100, &token_address);

    // Resolve as admin.
    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.payout_mode,
        types::PayoutMode::Push,
        "3 winners ≤ threshold(50) must select Push mode"
    );
}

/// resolve_market selects Pull mode when winner_count > max_push_payout_winners.
#[test]
fn test_resolve_market_selects_pull_mode_when_winners_exceed_threshold() {
    let (e, _admin, _, client) = setup();

    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    // Default threshold is 50, so place 51 distinct bettors.
    for _ in 0..51 {
        place_bet(&client, &e, market_id, 0, 100, &token_address);
    }

    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.payout_mode,
        types::PayoutMode::Pull,
        "51 winners > default threshold(50) must select Pull mode"
    );
}

/// Zero bettors on the winning outcome → winner_count is 0, Push mode selected.
#[test]
fn test_zero_bettors_on_winning_outcome_gives_push_mode() {
    let (e, _admin, _, client) = setup();

    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    // Bets only on outcome 1; outcome 0 has no bettors.
    place_bet(&client, &e, market_id, 1, 100, &token_address);

    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    // 0 ≤ threshold → Push
    assert_eq!(market.payout_mode, types::PayoutMode::Push);
    assert_eq!(
        client.get_resolution_metrics(&market_id, &0).winner_count,
        0
    );
}

// ── Issue 1: Heuristic vs exact count divergence proof ────────────────────────

/// Demonstrates that the old tally/100 heuristic would return 0 for a market
/// where 3 bettors each stake 10 tokens (total stake = 30, 30/100 = 0).
/// The exact counter must return 3, proving the migration is live and correct.
/// If this test fails, the exact counting path is broken.
#[test]
fn test_heuristic_diverges_from_exact_count_micro_bets() {
    let (e, _admin, _, client) = setup();
    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    // 3 bettors, each staking 10 tokens.
    // Old heuristic: total_stake(outcome 0) = 30 → 30 / 100 = 0 (integer division).
    // Exact counter: winner_counts[0] = 3.
    for _ in 0..3 {
        place_bet(&client, &e, market_id, 0, 10, &token_address);
    }

    let exact = client.count_bets_for_outcome(&market_id, &0);
    // Simulate what the heuristic would have returned: stake / 100
    let stake = client.get_outcome_stake(&market_id, &0);
    let heuristic = (stake / 100) as u32;

    assert_ne!(
        exact, heuristic,
        "heuristic ({heuristic}) must diverge from exact count ({exact}) for micro-bet markets"
    );
    assert_eq!(exact, 3, "exact counter must equal the number of distinct bettors");
    assert_eq!(heuristic, 0, "heuristic must undercount to 0 for 10-token micro-bets");
}

/// Demonstrates heuristic divergence at the boundary where stake is just below 100.
/// 5 bettors each staking 19 tokens → total stake = 95 → heuristic = 0, exact = 5.
#[test]
fn test_heuristic_diverges_below_100_stake_boundary() {
    let (e, _admin, _, client) = setup();
    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    for _ in 0..5 {
        place_bet(&client, &e, market_id, 0, 19, &token_address);
    }

    let exact = client.count_bets_for_outcome(&market_id, &0);
    let stake = client.get_outcome_stake(&market_id, &0);
    let heuristic = (stake / 100) as u32;

    assert_ne!(exact, heuristic, "heuristic diverges: exact={exact}, heuristic={heuristic}");
    assert_eq!(exact, 5);
    assert_eq!(heuristic, 0);
}

// ── Issue 2: Boundary coverage for max_push_winners threshold ─────────────────

/// winner_count == max_push_winners (exactly at threshold) → Push mode.
/// The condition is `actual_winners > max_push_winners`, so equals must be Push.
#[test]
fn test_resolve_push_mode_when_winners_equal_threshold() {
    let (e, _admin, _, client) = setup();
    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    // Set threshold to 5 so we can test equality cheaply.
    client.set_max_push_payout_winners(&5);
    assert_eq!(client.get_max_push_payout_winners(), 5);

    // Place exactly 5 bets (== threshold).
    for _ in 0..5 {
        place_bet(&client, &e, market_id, 0, 100, &token_address);
    }

    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.payout_mode,
        types::PayoutMode::Push,
        "winner_count == threshold must select Push (condition is strictly greater-than)"
    );
}

/// winner_count == max_push_winners - 1 (one below threshold) → Push mode.
#[test]
fn test_resolve_push_mode_when_winners_one_below_threshold() {
    let (e, _admin, _, client) = setup();
    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    client.set_max_push_payout_winners(&5);

    // Place 4 bets (threshold - 1).
    for _ in 0..4 {
        place_bet(&client, &e, market_id, 0, 100, &token_address);
    }

    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.payout_mode,
        types::PayoutMode::Push,
        "winner_count < threshold must select Push"
    );
}

/// winner_count == max_push_winners + 1 (one above threshold) → Pull mode.
#[test]
fn test_resolve_pull_mode_when_winners_one_above_threshold() {
    let (e, _admin, _, client) = setup();
    let (market_id, token_address) = create_market(&client, &e, 1000, 2000);

    client.set_max_push_payout_winners(&5);

    // Place 6 bets (threshold + 1).
    for _ in 0..6 {
        place_bet(&client, &e, market_id, 0, 100, &token_address);
    }

    client.resolve_market(&market_id, &0);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(
        market.payout_mode,
        types::PayoutMode::Pull,
        "winner_count > threshold must select Pull"
    );
}

/// Boundary triple: verify all three cases (below, equal, above) in one sweep
/// using a custom threshold, ensuring no off-by-one in the branch condition.
#[test]
fn test_threshold_boundary_all_three_cases() {
    let threshold: u32 = 3;

    // --- below threshold ---
    {
        let (e, _admin, _, client) = setup();
        let (market_id, token_address) = create_market(&client, &e, 1000, 2000);
        client.set_max_push_payout_winners(&threshold);
        for _ in 0..(threshold - 1) {
            place_bet(&client, &e, market_id, 0, 100, &token_address);
        }
        client.resolve_market(&market_id, &0);
        assert_eq!(
            client.get_market(&market_id).unwrap().payout_mode,
            types::PayoutMode::Push,
            "below threshold ({}) must be Push", threshold - 1
        );
    }

    // --- equal to threshold ---
    {
        let (e, _admin, _, client) = setup();
        let (market_id, token_address) = create_market(&client, &e, 1000, 2000);
        client.set_max_push_payout_winners(&threshold);
        for _ in 0..threshold {
            place_bet(&client, &e, market_id, 0, 100, &token_address);
        }
        client.resolve_market(&market_id, &0);
        assert_eq!(
            client.get_market(&market_id).unwrap().payout_mode,
            types::PayoutMode::Push,
            "equal to threshold ({}) must be Push", threshold
        );
    }

    // --- above threshold ---
    {
        let (e, _admin, _, client) = setup();
        let (market_id, token_address) = create_market(&client, &e, 1000, 2000);
        client.set_max_push_payout_winners(&threshold);
        for _ in 0..(threshold + 1) {
            place_bet(&client, &e, market_id, 0, 100, &token_address);
        }
        client.resolve_market(&market_id, &0);
        assert_eq!(
            client.get_market(&market_id).unwrap().payout_mode,
            types::PayoutMode::Pull,
            "above threshold ({}) must be Pull", threshold + 1
        );
    }
}
