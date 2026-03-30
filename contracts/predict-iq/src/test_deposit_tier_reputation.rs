//! Deterministic table-driven tests for creation deposit + reputation + tier interaction.
//!
//! Acceptance criteria (issue: markets tiering/deposit logic):
//!   - Every (tier, reputation) combination has explicit expected-deposit coverage.
//!   - Pro and Institutional reputation waive the deposit; None and Basic do not.
//!   - The stored `market.creation_deposit` always reflects the *configured* deposit,
//!     regardless of whether the creator was charged (it is the refundable amount).
//!   - The market tier is stored exactly as supplied.

#![cfg(test)]

use crate::types::{CreatorReputation, MarketStatus, MarketTier, OracleConfig};
use crate::{PredictIQ, PredictIQClient};
use soroban_sdk::{
    testutils::Address as _,
    token,
    Address, Env, String, Vec,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn setup() -> (Env, PredictIQClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PredictIQ, ());
    let client = PredictIQClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin, &100);
    (env, client, admin)
}

fn oracle(env: &Env) -> OracleConfig {
    OracleConfig {
        oracle_address: Address::generate(env),
        feed_id: String::from_str(env, "feed"),
        min_responses: Some(1),
        max_staleness_seconds: 3600,
        max_confidence_bps: 100,
    }
}

fn two_options(env: &Env) -> Vec<String> {
    Vec::from_array(env, [String::from_str(env, "Yes"), String::from_str(env, "No")])
}

/// Register a minimal SAC token, mint `amount` to `recipient`, and return the token address.
fn mint_token(env: &Env, recipient: &Address, amount: i128) -> Address {
    let token_id = env.register_stellar_asset_contract_v2(recipient.clone());
    let token_address = token_id.address();
    let admin_client = token::StellarAssetClient::new(env, &token_address);
    admin_client.mint(recipient, &amount);
    token_address
}

// ---------------------------------------------------------------------------
// Table-driven: deposit waiver by reputation
//
// Logic under test (markets.rs):
//   deposit_required = !matches!(reputation, Pro | Institutional)
//   adjusted_deposit = if deposit_required { creation_deposit } else { 0 }
// ---------------------------------------------------------------------------

struct DepositCase {
    reputation: CreatorReputation,
    deposit_waived: bool, // true → creator is NOT charged; false → creator IS charged
}

fn deposit_cases() -> [DepositCase; 4] {
    [
        DepositCase { reputation: CreatorReputation::None,          deposit_waived: false },
        DepositCase { reputation: CreatorReputation::Basic,         deposit_waived: false },
        DepositCase { reputation: CreatorReputation::Pro,           deposit_waived: true  },
        DepositCase { reputation: CreatorReputation::Institutional, deposit_waived: true  },
    ]
}

/// For each reputation level: verify whether the deposit is charged or waived.
#[test]
fn test_deposit_waiver_by_reputation() {
    const DEPOSIT: i128 = 10_000_000;

    for case in deposit_cases() {
        let (env, client, _admin) = setup();
        client.set_creation_deposit(&DEPOSIT);

        let creator = Address::generate(&env);
        client.set_creator_reputation(&creator, &case.reputation);

        let token = mint_token(&env, &creator, DEPOSIT * 2);

        let balance_before = token::Client::new(&env, &token).balance(&creator);

        let market_id = client.create_market(
            &creator,
            &String::from_str(&env, "M"),
            &two_options(&env),
            &1000,
            &2000,
            &oracle(&env),
            &MarketTier::Basic,
            &token,
            &0,
            &0,
        );

        let balance_after = token::Client::new(&env, &token).balance(&creator);
        let charged = balance_before - balance_after;

        if case.deposit_waived {
            assert_eq!(
                charged, 0,
                "reputation {:?} should waive deposit but was charged {}",
                case.reputation, charged
            );
        } else {
            assert_eq!(
                charged, DEPOSIT,
                "reputation {:?} should be charged {} but was charged {}",
                case.reputation, DEPOSIT, charged
            );
        }

        // market.creation_deposit always stores the configured amount (for refund tracking)
        let market = client.get_market(&market_id).unwrap();
        assert_eq!(
            market.creation_deposit, DEPOSIT,
            "market.creation_deposit should always store the configured deposit amount"
        );
    }
}

// ---------------------------------------------------------------------------
// Table-driven: tier is stored exactly as supplied, across all reputations
// ---------------------------------------------------------------------------

struct TierCase {
    tier: MarketTier,
    reputation: CreatorReputation,
}

fn tier_cases() -> [TierCase; 9] {
    [
        TierCase { tier: MarketTier::Basic,         reputation: CreatorReputation::None          },
        TierCase { tier: MarketTier::Basic,         reputation: CreatorReputation::Pro           },
        TierCase { tier: MarketTier::Pro,           reputation: CreatorReputation::None          },
        TierCase { tier: MarketTier::Pro,           reputation: CreatorReputation::Basic         },
        TierCase { tier: MarketTier::Pro,           reputation: CreatorReputation::Pro           },
        TierCase { tier: MarketTier::Pro,           reputation: CreatorReputation::Institutional },
        TierCase { tier: MarketTier::Institutional, reputation: CreatorReputation::None          },
        TierCase { tier: MarketTier::Institutional, reputation: CreatorReputation::Pro           },
        TierCase { tier: MarketTier::Institutional, reputation: CreatorReputation::Institutional },
    ]
}

#[test]
fn test_tier_stored_correctly_across_reputations() {
    let (env, client, _admin) = setup();
    // No deposit so we don't need token minting
    client.set_creation_deposit(&0);

    for case in tier_cases() {
        let creator = Address::generate(&env);
        client.set_creator_reputation(&creator, &case.reputation);
        let token = Address::generate(&env);

        let market_id = client.create_market(
            &creator,
            &String::from_str(&env, "M"),
            &two_options(&env),
            &1000,
            &2000,
            &oracle(&env),
            &case.tier,
            &token,
            &0,
            &0,
        );

        let market = client.get_market(&market_id).unwrap();
        assert_eq!(
            market.tier, case.tier,
            "tier mismatch for reputation {:?}",
            case.reputation
        );
        assert_eq!(market.status, MarketStatus::Active);
    }
}

// ---------------------------------------------------------------------------
// Deposit=0 edge case: no transfer occurs for any reputation
// ---------------------------------------------------------------------------

#[test]
fn test_zero_deposit_no_charge_for_any_reputation() {
    let reputations = [
        CreatorReputation::None,
        CreatorReputation::Basic,
        CreatorReputation::Pro,
        CreatorReputation::Institutional,
    ];

    for reputation in reputations {
        let (env, client, _admin) = setup();
        // deposit stays at default 0

        let creator = Address::generate(&env);
        client.set_creator_reputation(&creator, &reputation);
        // No token minting needed — transfer of 0 is a no-op
        let token = Address::generate(&env);

        let result = client.try_create_market(
            &creator,
            &String::from_str(&env, "M"),
            &two_options(&env),
            &1000,
            &2000,
            &oracle(&env),
            &MarketTier::Basic,
            &token,
            &0,
            &0,
        );

        assert!(
            result.is_ok(),
            "zero deposit should never block creation (reputation {:?})",
            reputation
        );
    }
}

// ---------------------------------------------------------------------------
// Insufficient balance: None/Basic reputation with deposit set must fail
// ---------------------------------------------------------------------------

#[test]
fn test_insufficient_balance_blocks_creation_for_non_waived_reputations() {
    const DEPOSIT: i128 = 10_000_000;

    let non_waived = [CreatorReputation::None, CreatorReputation::Basic];

    for reputation in non_waived {
        let (env, client, _admin) = setup();
        client.set_creation_deposit(&DEPOSIT);

        let creator = Address::generate(&env);
        client.set_creator_reputation(&creator, &reputation);

        // Mint less than required
        let token = mint_token(&env, &creator, DEPOSIT - 1);

        let result = client.try_create_market(
            &creator,
            &String::from_str(&env, "M"),
            &two_options(&env),
            &1000,
            &2000,
            &oracle(&env),
            &MarketTier::Basic,
            &token,
            &0,
            &0,
        );

        assert!(
            result.is_err(),
            "reputation {:?} with insufficient balance should fail",
            reputation
        );
    }
}

/// Pro/Institutional with deposit set but zero balance: creation succeeds (waived).
#[test]
fn test_waived_reputations_succeed_with_zero_balance() {
    const DEPOSIT: i128 = 10_000_000;

    let waived = [CreatorReputation::Pro, CreatorReputation::Institutional];

    for reputation in waived {
        let (env, client, _admin) = setup();
        client.set_creation_deposit(&DEPOSIT);

        let creator = Address::generate(&env);
        client.set_creator_reputation(&creator, &reputation);

        // No tokens minted — balance is 0
        let token = Address::generate(&env);

        let result = client.try_create_market(
            &creator,
            &String::from_str(&env, "M"),
            &two_options(&env),
            &1000,
            &2000,
            &oracle(&env),
            &MarketTier::Basic,
            &token,
            &0,
            &0,
        );

        assert!(
            result.is_ok(),
            "reputation {:?} should be waived and succeed with zero balance",
            reputation
        );
    }
}
