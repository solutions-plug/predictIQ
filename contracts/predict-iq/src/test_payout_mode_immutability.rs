//! Issue #252 / #182: Tests for payout mode immutability after market creation.
//!
//! `payout_mode` is fixed at creation time and must not be mutated by any
//! resolution path (oracle, dispute, admin-fallback) or while the market is
//! in PendingResolution or Disputed state (issue #182).

#![cfg(test)]

use crate::modules::markets::DataKey as MarketDataKey;
use crate::types::{Market, MarketStatus, OracleConfig, PayoutMode};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Env, String, Vec,
};

// ── helpers ──────────────────────────────────────────────────────────────────

fn setup(e: &Env) -> (PredictIQClient, Address, Address, Address) {
    e.mock_all_auths();
    let admin = Address::generate(e);
    let contract_id = e.register(PredictIQ, ());
    let client = PredictIQClient::new(e, &contract_id);
    client.initialize(&admin, &1000);

    let token_admin = Address::generate(e);
    let token_id = e.register_stellar_asset_contract_v2(token_admin.clone());
    let token_address = token_id.address();
    let stellar_token = token::StellarAssetClient::new(e, &token_address);

    client.set_governance_token(&token_address);

    (client, contract_id, admin, token_address)
}

fn create_two_outcome_market(
    e: &Env,
    client: &PredictIQClient,
    token_address: &Address,
) -> u64 {
    let creator = Address::generate(e);
    let oracle_config = OracleConfig {
        oracle_address: Address::generate(e),
        feed_id: String::from_str(e, "BTC/USD"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 200,
    };
    client.create_market(
        &creator,
        &String::from_str(e, "Test market"),
        &Vec::from_array(e, [String::from_str(e, "Yes"), String::from_str(e, "No")]),
        &1000,
        &2000,
        &oracle_config,
        &crate::types::MarketTier::Basic,
        token_address,
        &0u64,
        &0u32,
    )
}

/// Read payout_mode directly from storage.
fn get_payout_mode(e: &Env, contract_id: &Address, market_id: u64) -> PayoutMode {
    e.as_contract(contract_id, || {
        let market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.payout_mode
    })
}

/// Force market into PendingResolution so oracle/dispute flows can proceed.
fn set_pending_resolution(e: &Env, contract_id: &Address, market_id: u64) {
    e.as_contract(contract_id, || {
        let mut market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.status = MarketStatus::PendingResolution;
        market.pending_resolution_timestamp = Some(e.ledger().timestamp());
        market.winning_outcome = Some(0);
        e.storage()
            .persistent()
            .set(&MarketDataKey::Market(market_id), &market);
    });
}

/// Force market into Disputed so voting/admin-fallback flows can proceed.
fn set_disputed(e: &Env, contract_id: &Address, market_id: u64) {
    e.as_contract(contract_id, || {
        let mut market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.status = MarketStatus::Disputed;
        market.dispute_timestamp = Some(e.ledger().timestamp());
        e.storage()
            .persistent()
            .set(&MarketDataKey::Market(market_id), &market);
    });
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// Payout mode set at creation must survive the oracle resolution path.
#[test]
fn test_payout_mode_unchanged_after_oracle_resolution() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let mode_at_creation = get_payout_mode(&e, &contract_id, market_id);

    // Move through oracle resolution → finalize
    set_pending_resolution(&e, &contract_id, market_id);

    // Advance past 72h dispute window
    e.ledger().with_mut(|li| li.timestamp += 259_200 + 1);

    client.finalize_resolution(&market_id);

    let mode_after_resolution = get_payout_mode(&e, &contract_id, market_id);
    assert_eq!(
        mode_at_creation, mode_after_resolution,
        "payout_mode must not change through oracle resolution path"
    );
}

/// Payout mode must survive filing a dispute.
#[test]
fn test_payout_mode_unchanged_after_dispute_filed() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let mode_at_creation = get_payout_mode(&e, &contract_id, market_id);

    set_pending_resolution(&e, &contract_id, market_id);

    let disciplinarian = Address::generate(&e);
    client.file_dispute(&disciplinarian, &market_id);

    let mode_after_dispute = get_payout_mode(&e, &contract_id, market_id);
    assert_eq!(
        mode_at_creation, mode_after_dispute,
        "payout_mode must not change when a dispute is filed"
    );
}

/// Payout mode must survive the full dispute → voting → finalize path.
#[test]
fn test_payout_mode_unchanged_after_dispute_resolution() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let mode_at_creation = get_payout_mode(&e, &contract_id, market_id);

    // Inject a clear majority vote directly into storage
    set_disputed(&e, &contract_id, market_id);
    e.as_contract(&contract_id, || {
        use crate::modules::voting::DataKey as VotingDataKey;
        e.storage()
            .persistent()
            .set(&VotingDataKey::VoteTally(market_id, 0u32), &6000_i128);
        e.storage()
            .persistent()
            .set(&VotingDataKey::VoteTally(market_id, 1u32), &4000_i128);
    });

    // Advance past 72h voting period
    e.ledger().with_mut(|li| li.timestamp += 259_200 + 1);

    client.finalize_resolution(&market_id);

    let mode_after = get_payout_mode(&e, &contract_id, market_id);
    assert_eq!(
        mode_at_creation, mode_after,
        "payout_mode must not change through dispute resolution path"
    );
}

/// Payout mode must survive admin fallback resolution.
#[test]
fn test_payout_mode_unchanged_after_admin_fallback() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let mode_at_creation = get_payout_mode(&e, &contract_id, market_id);

    // Tie → NoMajorityReached → admin fallback required
    set_disputed(&e, &contract_id, market_id);
    e.as_contract(&contract_id, || {
        use crate::modules::voting::DataKey as VotingDataKey;
        e.storage()
            .persistent()
            .set(&VotingDataKey::VoteTally(market_id, 0u32), &5000_i128);
        e.storage()
            .persistent()
            .set(&VotingDataKey::VoteTally(market_id, 1u32), &5000_i128);
    });

    e.ledger().with_mut(|li| li.timestamp += 259_200 + 1);

    client.admin_fallback_resolution(&market_id, &0u32);

    let mode_after = get_payout_mode(&e, &contract_id, market_id);
    assert_eq!(
        mode_at_creation, mode_after,
        "payout_mode must not change through admin fallback resolution"
    );
}

/// Explicitly verify resolve_market (disputes.rs) does NOT mutate payout_mode.
/// This directly tests the bug described in issue #252.
#[test]
fn test_resolve_market_does_not_mutate_payout_mode() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let mode_at_creation = get_payout_mode(&e, &contract_id, market_id);

    // Call resolve_market directly via the contract entrypoint
    set_disputed(&e, &contract_id, market_id);
    e.as_contract(&contract_id, || {
        crate::modules::disputes::resolve_market(&e, market_id, 0).unwrap();
    });

    let mode_after = get_payout_mode(&e, &contract_id, market_id);
    assert_eq!(
        mode_at_creation, mode_after,
        "resolve_market must not mutate payout_mode (issue #252)"
    );
}


// ── issue #182: payout_mode locked once status leaves Active ─────────────────

/// Helper: force market into a given status directly in storage.
fn force_status(e: &Env, contract_id: &Address, market_id: u64, status: MarketStatus) {
    e.as_contract(contract_id, || {
        let mut market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.status = status.clone();
        if status == MarketStatus::PendingResolution {
            market.pending_resolution_timestamp = Some(e.ledger().timestamp());
        }
        if status == MarketStatus::Disputed {
            market.dispute_timestamp = Some(e.ledger().timestamp());
        }
        e.storage()
            .persistent()
            .set(&MarketDataKey::Market(market_id), &market);
    });
}

/// resolve_market must NOT change payout_mode when market is PendingResolution.
#[test]
fn test_resolve_market_does_not_change_payout_mode_during_pending_resolution() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let mode_before = get_payout_mode(&e, &contract_id, market_id);

    force_status(&e, &contract_id, market_id, MarketStatus::PendingResolution);

    e.as_contract(&contract_id, || {
        crate::modules::disputes::resolve_market(&e, market_id, 0).unwrap();
    });

    assert_eq!(
        mode_before,
        get_payout_mode(&e, &contract_id, market_id),
        "payout_mode must not change during PendingResolution (issue #182)"
    );
}

/// resolve_market must NOT change payout_mode when market is Disputed.
#[test]
fn test_resolve_market_does_not_change_payout_mode_during_disputed() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let mode_before = get_payout_mode(&e, &contract_id, market_id);

    force_status(&e, &contract_id, market_id, MarketStatus::Disputed);

    e.as_contract(&contract_id, || {
        crate::modules::disputes::resolve_market(&e, market_id, 0).unwrap();
    });

    assert_eq!(
        mode_before,
        get_payout_mode(&e, &contract_id, market_id),
        "payout_mode must not change during Disputed phase (issue #182)"
    );
}

/// payout_mode set at creation survives the full lifecycle:
/// Active → PendingResolution → Disputed → Resolved.
#[test]
fn test_payout_mode_stable_across_full_lifecycle() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let mode_at_creation = get_payout_mode(&e, &contract_id, market_id);

    force_status(&e, &contract_id, market_id, MarketStatus::PendingResolution);
    assert_eq!(mode_at_creation, get_payout_mode(&e, &contract_id, market_id));

    force_status(&e, &contract_id, market_id, MarketStatus::Disputed);
    assert_eq!(mode_at_creation, get_payout_mode(&e, &contract_id, market_id));

    e.as_contract(&contract_id, || {
        crate::modules::disputes::resolve_market(&e, market_id, 0).unwrap();
    });
    assert_eq!(
        mode_at_creation,
        get_payout_mode(&e, &contract_id, market_id),
        "payout_mode must be stable across the full market lifecycle (issue #182)"
    );
}

// ── issue #182: set_payout_mode API enforcement ───────────────────────────────

/// Creator can change payout_mode while the market is Active.
#[test]
fn test_set_payout_mode_allowed_when_active() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let creator = e.as_contract(&contract_id, || {
        let market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.creator
    });

    // Default is Pull — switch to Push while Active
    client.set_payout_mode(&creator, &market_id, &PayoutMode::Push);
    assert_eq!(get_payout_mode(&e, &contract_id, market_id), PayoutMode::Push);

    // Switch back to Pull
    client.set_payout_mode(&creator, &market_id, &PayoutMode::Pull);
    assert_eq!(get_payout_mode(&e, &contract_id, market_id), PayoutMode::Pull);
}

/// set_payout_mode must be rejected once the market enters PendingResolution.
#[test]
fn test_set_payout_mode_rejected_when_pending_resolution() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let creator = e.as_contract(&contract_id, || {
        let market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.creator
    });

    force_status(&e, &contract_id, market_id, MarketStatus::PendingResolution);

    let result = client.try_set_payout_mode(&creator, &market_id, &PayoutMode::Push);
    assert_eq!(
        result,
        Err(Ok(crate::errors::ErrorCode::PayoutModeLocked)),
        "set_payout_mode must return PayoutModeLocked during PendingResolution (issue #182)"
    );
}

/// set_payout_mode must be rejected once the market enters Disputed.
#[test]
fn test_set_payout_mode_rejected_when_disputed() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let creator = e.as_contract(&contract_id, || {
        let market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.creator
    });

    force_status(&e, &contract_id, market_id, MarketStatus::Disputed);

    let result = client.try_set_payout_mode(&creator, &market_id, &PayoutMode::Push);
    assert_eq!(
        result,
        Err(Ok(crate::errors::ErrorCode::PayoutModeLocked)),
        "set_payout_mode must return PayoutModeLocked during Disputed phase (issue #182)"
    );
}

/// set_payout_mode must be rejected once the market is Resolved.
#[test]
fn test_set_payout_mode_rejected_when_resolved() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let creator = e.as_contract(&contract_id, || {
        let market: Market = e
            .storage()
            .persistent()
            .get(&MarketDataKey::Market(market_id))
            .unwrap();
        market.creator
    });

    force_status(&e, &contract_id, market_id, MarketStatus::Disputed);
    e.as_contract(&contract_id, || {
        crate::modules::disputes::resolve_market(&e, market_id, 0).unwrap();
    });

    let result = client.try_set_payout_mode(&creator, &market_id, &PayoutMode::Push);
    assert_eq!(
        result,
        Err(Ok(crate::errors::ErrorCode::PayoutModeLocked)),
        "set_payout_mode must return PayoutModeLocked when market is Resolved (issue #182)"
    );
}

/// Non-creator must be rejected regardless of market status.
#[test]
fn test_set_payout_mode_rejected_for_non_creator() {
    let e = Env::default();
    let (client, contract_id, _, token_address) = setup(&e);
    let market_id = create_two_outcome_market(&e, &client, &token_address);

    let stranger = Address::generate(&e);
    let result = client.try_set_payout_mode(&stranger, &market_id, &PayoutMode::Push);
    assert_eq!(
        result,
        Err(Ok(crate::errors::ErrorCode::NotAuthorized)),
        "set_payout_mode must reject non-creator callers"
    );
}
