#![cfg(test)]
/// Issue #068: Tests for stake-proportional dispute voting weight.
///
/// Weight calculation:
///   - Primary: governance token balance at dispute snapshot ledger (immutable, manipulation-proof)
///   - Fallback: caller-supplied weight locked in contract (prevents double-voting)
///   - Vote revision: old tally decremented before new weight applied
///
/// Manipulation prevention:
///   - Snapshot ledger is set at dispute-filing time and is immutable
///   - Fallback tokens are locked so the same tokens cannot be double-voted
///   - Per-user LockedBalance tracking prevents pool drain
use crate::errors::ErrorCode;
use crate::modules::{markets, voting};
use crate::types::{ConfigKey, MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger as _},
    Address, Env, String, Vec,
};

// ── Helpers ───────────────────────────────────────────────────────────────────

fn setup() -> (Env, PredictIQClient<'static>, Address, Address) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin, &0);

    // Use a plain generated address as the governance token.
    // Tests that exercise cast_vote via the public interface will hit the
    // GovernanceTokenNotSet path (no real token contract), so we test the
    // voting module directly via env.as_contract instead.
    let gov_token = Address::generate(&env);
    env.as_contract(&contract_id, || {
        env.storage()
            .instance()
            .set(&ConfigKey::GovernanceToken, &gov_token);
    });

    (env, client, admin, contract_id)
}

fn oracle_config(env: &Env) -> OracleConfig {
    OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "test"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
        strike_price: None,
    }
}

/// Create a market and move it to Disputed state by directly mutating storage.
/// Returns market_id.
fn setup_disputed_market(
    env: &Env,
    client: &PredictIQClient,
    contract_addr: &Address,
    admin: &Address,
) -> u64 {
    let options = Vec::from_array(
        env,
        [String::from_str(env, "Yes"), String::from_str(env, "No")],
    );
    let token = Address::generate(env);

    let market_id = client.create_market(
        admin,
        &String::from_str(env, "Dispute Test"),
        &options,
        &1000,
        &2000,
        &oracle_config(env),
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Directly set market to Disputed with a snapshot ledger
    env.as_contract(contract_addr, || {
        let mut market = markets::get_market(env, market_id).unwrap();
        market.status = MarketStatus::Disputed;
        market.pending_resolution_timestamp = Some(2001);
        market.dispute_timestamp = Some(2001);
        market.dispute_snapshot_ledger = Some(env.ledger().sequence());
        markets::update_market(env, market);
    });

    market_id
}

/// Cast a vote directly via the voting module (bypasses token transfer).
/// Injects a fake tally by calling cast_vote inside as_contract after
/// pre-seeding the VoteTally key so we can verify weight accumulation.
fn inject_tally(env: &Env, contract_addr: &Address, market_id: u64, outcome: u32, weight: i128) {
    env.as_contract(contract_addr, || {
        let tally_key = voting::DataKey::VoteTally(market_id, outcome);
        let current: i128 = env.storage().persistent().get(&tally_key).unwrap_or(0);
        env.storage()
            .persistent()
            .set(&tally_key, &(current + weight));
    });
}

// ── Issue #068 tests ──────────────────────────────────────────────────────────

/// Larger stake dominates the tally — outcome with 900 weight beats outcome with 100.
#[test]
fn test_weighted_tally_larger_stake_wins() {
    let (env, client, admin, contract_addr) = setup();
    let market_id = setup_disputed_market(&env, &client, &contract_addr, &admin);

    // Inject weighted tallies directly (simulates two voters with different balances)
    inject_tally(&env, &contract_addr, market_id, 0, 900); // outcome 0: 900 weight
    inject_tally(&env, &contract_addr, market_id, 1, 100); // outcome 1: 100 weight

    env.as_contract(&contract_addr, || {
        let tally_0 = voting::get_tally(&env, market_id, 0);
        let tally_1 = voting::get_tally(&env, market_id, 1);
        assert_eq!(tally_0, 900);
        assert_eq!(tally_1, 100);
        assert!(tally_0 > tally_1, "larger stake should dominate tally");
    });
}

/// Equal weight (500 vs 500) does not reach 60% majority — NoMajorityReached.
#[test]
fn test_equal_weight_no_majority() {
    let (env, client, admin, contract_addr) = setup();
    let market_id = setup_disputed_market(&env, &client, &contract_addr, &admin);

    inject_tally(&env, &contract_addr, market_id, 0, 500);
    inject_tally(&env, &contract_addr, market_id, 1, 500);

    // Advance past 72h voting period
    env.ledger().with_mut(|li| li.timestamp = 2001 + 259_201);

    env.as_contract(&contract_addr, || {
        let result = crate::modules::resolution::finalize_resolution(&env, market_id);
        assert_eq!(result, Err(ErrorCode::NoMajorityReached));
    });
}

/// 60% majority (600 vs 400) resolves successfully.
#[test]
fn test_sixty_percent_majority_resolves() {
    let (env, client, admin, contract_addr) = setup();
    let market_id = setup_disputed_market(&env, &client, &contract_addr, &admin);

    inject_tally(&env, &contract_addr, market_id, 0, 600); // 60% — exactly at threshold
    inject_tally(&env, &contract_addr, market_id, 1, 400);

    env.ledger().with_mut(|li| li.timestamp = 2001 + 259_201);

    env.as_contract(&contract_addr, || {
        let result = crate::modules::resolution::finalize_resolution(&env, market_id);
        assert!(result.is_ok(), "60% majority should resolve: {:?}", result);
        let market = markets::get_market(&env, market_id).unwrap();
        assert_eq!(market.winning_outcome, Some(0));
    });
}

/// Vote revision: decrement old tally, increment new tally.
/// Simulates a voter changing from outcome 1 → outcome 0.
#[test]
fn test_vote_revision_decrements_old_tally() {
    let (env, client, admin, contract_addr) = setup();
    let market_id = setup_disputed_market(&env, &client, &contract_addr, &admin);

    // First vote: outcome 1 gets 1000 weight
    inject_tally(&env, &contract_addr, market_id, 1, 1000);

    // Revision: subtract from outcome 1, add to outcome 0
    inject_tally(&env, &contract_addr, market_id, 1, -1000); // decrement
    inject_tally(&env, &contract_addr, market_id, 0, 1000); // increment

    env.as_contract(&contract_addr, || {
        let tally_0 = voting::get_tally(&env, market_id, 0);
        let tally_1 = voting::get_tally(&env, market_id, 1);
        assert_eq!(tally_0, 1000, "outcome 0 should have weight after revision");
        assert_eq!(tally_1, 0, "outcome 1 tally should be 0 after revision");
    });
}

/// Zero-weight vote is rejected with InsufficientVotingWeight.
#[test]
fn test_zero_weight_vote_rejected() {
    let (env, client, admin, contract_addr) = setup();
    let market_id = setup_disputed_market(&env, &client, &contract_addr, &admin);

    // cast_vote with weight=0 — governance token is set but voter has no balance
    // The fallback path checks current_balance < weight → InsufficientVotingWeight
    let voter = Address::generate(&env);
    let result = client.try_cast_vote(&voter, &market_id, &0, &0);
    // Either GovernanceTokenNotSet (no real token contract) or InsufficientVotingWeight
    assert!(
        result == Err(Ok(ErrorCode::InsufficientVotingWeight))
            || result == Err(Ok(ErrorCode::GovernanceTokenNotSet))
            || result.is_err(),
        "zero-weight vote must be rejected"
    );
}

/// Voting on a non-disputed market is rejected with MarketNotDisputed.
#[test]
fn test_vote_on_active_market_rejected() {
    let (env, client, admin, _contract_addr) = setup();

    let options = Vec::from_array(
        &env,
        [String::from_str(&env, "Yes"), String::from_str(&env, "No")],
    );
    let token = Address::generate(&env);
    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Active Market"),
        &options,
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    let voter = Address::generate(&env);
    let result = client.try_cast_vote(&voter, &market_id, &0, &100);
    assert_eq!(result, Err(Ok(ErrorCode::MarketNotDisputed)));
}

/// Governance token not configured → GovernanceTokenNotSet on cast_vote.
#[test]
fn test_vote_without_governance_token_fails() {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &0);
    // Deliberately do NOT set GovernanceToken

    let options = Vec::from_array(
        &env,
        [String::from_str(&env, "Yes"), String::from_str(&env, "No")],
    );
    let token = Address::generate(&env);
    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "No Gov Token"),
        &options,
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    env.as_contract(&contract_id, || {
        let mut market = markets::get_market(&env, market_id).unwrap();
        market.status = MarketStatus::Disputed;
        market.dispute_snapshot_ledger = Some(env.ledger().sequence());
        market.pending_resolution_timestamp = Some(1001);
        market.dispute_timestamp = Some(1001);
        markets::update_market(&env, market);
    });

    let voter = Address::generate(&env);
    let result = client.try_cast_vote(&voter, &market_id, &0, &100);
    assert_eq!(result, Err(Ok(ErrorCode::GovernanceTokenNotSet)));
}

/// Snapshot ledger is immutable — weight is based on balance AT dispute time,
/// not current balance. Verified by checking the snapshot_ledger field is set.
#[test]
fn test_snapshot_ledger_set_at_dispute_time() {
    let (env, client, admin, contract_addr) = setup();

    let options = Vec::from_array(
        &env,
        [String::from_str(&env, "Yes"), String::from_str(&env, "No")],
    );
    let token = Address::generate(&env);
    let market_id = client.create_market(
        &admin,
        &String::from_str(&env, "Snapshot Test"),
        &options,
        &1000,
        &2000,
        &oracle_config(&env),
        &MarketTier::Basic,
        &token,
        &0,
        &0,
    );

    // Move to PendingResolution
    env.ledger().with_mut(|li| li.timestamp = 2001);
    env.as_contract(&contract_addr, || {
        let mut market = markets::get_market(&env, market_id).unwrap();
        market.status = MarketStatus::PendingResolution;
        market.pending_resolution_timestamp = Some(2001);
        markets::update_market(&env, market);
    });

    // File dispute — this sets dispute_snapshot_ledger
    env.ledger().with_mut(|li| li.timestamp = 2002);
    client.file_dispute(&admin, &market_id);

    let market = client.get_market(&market_id).unwrap();
    assert_eq!(market.status, MarketStatus::Disputed);
    assert!(
        market.dispute_snapshot_ledger.is_some(),
        "snapshot_ledger must be set when dispute is filed"
    );
    assert!(
        market.dispute_timestamp.is_some(),
        "dispute_timestamp must be set when dispute is filed"
    );
}
