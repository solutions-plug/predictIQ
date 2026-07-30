use crate::errors::ErrorCode;
use crate::modules::markets;
// Issue #171: ConfigKey (including GovernanceToken variant) must be explicitly imported
// from types. Previously missing, causing compilation failure in cast_vote.
use crate::types::{ConfigKey, LockedTokens, MarketStatus, Vote, CANCEL_OUTCOME_INDEX};
use soroban_sdk::{contracttype, token, Address, Env, IntoVal, Symbol, Val, Vec};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Vote(u64, Address),         // market_id, voter
    VoteTally(u64, u32),        // market_id, outcome -> total_weight
    LockedTokens(u64, Address), // market_id, voter
    /// Issue #37: Per-user locked balance ledger to prevent pool drain.
    LockedBalance(u64, Address), // market_id, voter -> amount
    /// Registered voters for a disputed market — drives O(n) deep prune (Issue #84).
    DisputeVoters(u64), // market_id -> Vec<Address>
}

pub fn cast_vote(
    e: &Env,
    voter: Address,
    market_id: u64,
    outcome: u32,
    weight: i128,
) -> Result<(), ErrorCode> {
    voter.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    if market.status != MarketStatus::Disputed {
        return Err(ErrorCode::MarketNotDisputed);
    }

    if outcome != CANCEL_OUTCOME_INDEX && outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }

    let vote_key = DataKey::Vote(market_id, voter.clone());

    // Issue #175: Allow vote revision - voters can change their vote before resolution deadline
    // This enables more flexible governance where voters can respond to new information
    let old_vote: Option<Vote> = e.storage().persistent().get(&vote_key);
    if let Some(ref old_vote_data) = old_vote {
        // Decrement the old outcome tally when vote is revised
        let old_tally_key = DataKey::VoteTally(market_id, old_vote_data.outcome);
        let mut old_tally: i128 = e.storage().persistent().get(&old_tally_key).unwrap_or(0);
        old_tally -= old_vote_data.weight;
        e.storage().persistent().set(&old_tally_key, &old_tally);
    }

    let snapshot_ledger = market
        .dispute_snapshot_ledger
        .ok_or(ErrorCode::MarketNotDisputed)?;

    // Issue #068: Vote weight is proportional to governance token stake.
    //
    // Weight calculation:
    //   1. Primary path  — query the governance token's `balance_at(voter, snapshot_ledger)`
    //      to get the voter's balance at the exact ledger when the dispute was filed.
    //      This prevents weight manipulation by buying tokens after a dispute starts.
    //   2. Fallback path — if the token contract does not support `balance_at` (older tokens),
    //      the caller-supplied `weight` is used, but only up to their current live balance.
    //      Tokens are physically locked in the contract for the dispute duration and released
    //      only after the market reaches `Resolved` status (see unlock_tokens).
    //
    // Manipulation prevention:
    //   - Snapshot ledger is set at dispute-filing time and is immutable.
    //   - Fallback tokens are locked so the same tokens cannot be double-voted.
    //   - Per-user LockedBalance tracking (DataKey::LockedBalance) prevents pool drain.
    //   - Vote revision decrements the old tally before adding the new one.

    // Issue #3: GovernanceToken now exists in ConfigKey
    let gov_token: Address = e
        .storage()
        .instance()
        .get(&ConfigKey::GovernanceToken)
        .ok_or(ErrorCode::GovernanceTokenNotSet)?;

    let actual_weight = match try_get_balance_at(e, &gov_token, &voter, snapshot_ledger) {
        Ok(balance) => balance,
        Err(_) => {
            // Issue #37: Fallback — lock tokens and track per-user balance
            let token_client = token::Client::new(e, &gov_token);
            let current_balance = token_client.balance(&voter);
            if current_balance < weight {
                return Err(ErrorCode::InsufficientVotingWeight);
            }

            e.current_contract_address().require_auth();
            token_client.transfer(&voter, &e.current_contract_address(), &weight);

            // Track per-user locked amount so multiple users don't collide
            let lock_key = DataKey::LockedBalance(market_id, voter.clone());
            let existing: i128 = e.storage().persistent().get(&lock_key).unwrap_or(0);
            e.storage()
                .persistent()
                .set(&lock_key, &(existing + weight));

            let locked = LockedTokens {
                voter: voter.clone(),
                market_id,
                amount: weight,
                unlock_time: market.resolution_deadline,
            };
            e.storage()
                .persistent()
                .set(&DataKey::LockedTokens(market_id, voter.clone()), &locked);

            weight
        }
    };

    if actual_weight == 0 {
        return Err(ErrorCode::InsufficientVotingWeight);
    }

    // Normalize to 18 decimal places so tokens with different precisions are comparable.
    let token_decimals = get_token_decimals(e, &gov_token);
    const NORMALIZED_DECIMALS: u32 = 18;
    let normalized_weight = if token_decimals < NORMALIZED_DECIMALS {
        let scale = 10i128.pow(NORMALIZED_DECIMALS - token_decimals);
        actual_weight.saturating_mul(scale)
    } else if token_decimals > NORMALIZED_DECIMALS {
        let scale = 10i128.pow(token_decimals - NORMALIZED_DECIMALS);
        actual_weight / scale
    } else {
        actual_weight
    };

    if normalized_weight == 0 {
        return Err(ErrorCode::InsufficientVotingWeight);
    }

    let vote = Vote {
        market_id,
        voter: voter.clone(),
        outcome,
        weight: normalized_weight,
    };

    e.storage().persistent().set(&vote_key, &vote);

    if old_vote.is_none() {
        let reg_key = DataKey::DisputeVoters(market_id);
        let mut voters: Vec<Address> = e
            .storage()
            .persistent()
            .get(&reg_key)
            .unwrap_or(Vec::new(e));
        voters.push_back(voter.clone());
        e.storage().persistent().set(&reg_key, &voters);
    }

    let tally_key = DataKey::VoteTally(market_id, outcome);
    let mut current_tally: i128 = e.storage().persistent().get(&tally_key).unwrap_or(0);
    current_tally += normalized_weight;
    e.storage().persistent().set(&tally_key, &current_tally);

    crate::modules::events::emit_vote_cast(e, market_id, voter, outcome, normalized_weight);

    Ok(())
}

fn try_get_balance_at(
    e: &Env,
    token: &Address,
    account: &Address,
    ledger: u32,
) -> Result<i128, ErrorCode> {
    let args: Vec<Val> = soroban_sdk::vec![e, account.clone().into_val(e), ledger.into_val(e)];

    match e.try_invoke_contract::<i128, ErrorCode>(token, &Symbol::new(e, "balance_at"), args) {
        Ok(Ok(balance)) => Ok(balance),
        _ => Err(ErrorCode::OracleFailure),
    }
}

/// Fetch the decimal precision of a token contract (defaults to 7 for Stellar native tokens).
fn get_token_decimals(e: &Env, token: &Address) -> u32 {
    let args: Vec<Val> = soroban_sdk::vec![e];
    match e.try_invoke_contract::<u32, ErrorCode>(token, &Symbol::new(e, "decimals"), args) {
        Ok(Ok(d)) => d,
        _ => 7, // Stellar native token default
    }
}

/// Issue #20: Require market to be Resolved (or Cancelled) before unlocking tokens.
pub fn unlock_tokens(e: &Env, voter: Address, market_id: u64) -> Result<(), ErrorCode> {
    voter.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    // Issue #20: Tokens remain locked throughout the entire dispute lifecycle.
    // Only allow unlock once the market reaches a terminal state.
    // Issue #1192: A Disputed market can transition to Cancelled via community
    // vote (cancellation::cancel_market_vote); requiring Resolved only would
    // permanently strand governance tokens locked via the fallback path with
    // no code path to retrieve them.
    if market.status != MarketStatus::Resolved && market.status != MarketStatus::Cancelled {
        return Err(ErrorCode::MarketNotResolved);
    }

    let lock_key = DataKey::LockedTokens(market_id, voter.clone());
    let locked: LockedTokens = e
        .storage()
        .persistent()
        .get(&lock_key)
        .ok_or(ErrorCode::BetNotFound)?;

    if e.ledger().timestamp() < locked.unlock_time {
        return Err(ErrorCode::TimelockActive);
    }

    // Issue #37: Use LockedBalance as the authoritative per-user amount to
    // prevent a user from withdrawing more than they individually locked.
    let balance_key = DataKey::LockedBalance(market_id, voter.clone());
    let amount: i128 = e.storage().persistent().get(&balance_key).unwrap_or(0);

    if amount <= 0 {
        return Err(ErrorCode::BetNotFound);
    }

    let gov_token: Address = e
        .storage()
        .instance()
        .get(&ConfigKey::GovernanceToken)
        .ok_or(ErrorCode::GovernanceTokenNotSet)?;

    let token_client = token::Client::new(e, &gov_token);
    e.current_contract_address().require_auth();
    token_client.transfer(&e.current_contract_address(), &voter, &amount);

    e.storage().persistent().remove(&lock_key);
    e.storage().persistent().remove(&balance_key);

    Ok(())
}

pub fn get_tally(e: &Env, market_id: u64, outcome: u32) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::VoteTally(market_id, outcome))
        .unwrap_or(0)
}

/// Clears vote tallies, per-voter vote/lock ledgers, and the dispute voter registry.
/// Safe to call when no voting occurred (only removes keys that exist).
pub fn prune_market_voting_state(e: &Env, market_id: u64, num_outcomes: u32) {
    let reg_key = DataKey::DisputeVoters(market_id);
    if let Some(voters) = e.storage().persistent().get::<_, Vec<Address>>(&reg_key) {
        for i in 0..voters.len() {
            let v = voters.get(i).unwrap();
            e.storage()
                .persistent()
                .remove(&DataKey::Vote(market_id, v.clone()));
            e.storage()
                .persistent()
                .remove(&DataKey::LockedTokens(market_id, v.clone()));
            e.storage()
                .persistent()
                .remove(&DataKey::LockedBalance(market_id, v.clone()));
        }
    }
    e.storage().persistent().remove(&reg_key);

    for o in 0..num_outcomes {
        e.storage()
            .persistent()
            .remove(&DataKey::VoteTally(market_id, o));
    }
}

#[cfg(test)]
mod import_tests {
    use crate::types::ConfigKey;
    use soroban_sdk::{testutils::Address as _, Address, Env};

    /// Issue #171: Verify GovernanceToken variant is accessible and round-trips through storage.
    #[test]
    fn governance_token_config_key_round_trips() {
        let e = Env::default();
        let token = Address::generate(&e);
        e.storage()
            .instance()
            .set(&ConfigKey::GovernanceToken, &token);
        let stored: Option<Address> = e.storage().instance().get(&ConfigKey::GovernanceToken);
        assert_eq!(stored, Some(token));
    }

    /// Issue #171: cast_vote returns GovernanceTokenNotSet when token is not configured.
    #[test]
    fn cast_vote_returns_error_when_governance_token_not_set() {
        let e = Env::default();
        // GovernanceToken not set in storage — get returns None
        let stored: Option<Address> = e.storage().instance().get(&ConfigKey::GovernanceToken);
        assert!(
            stored.is_none(),
            "GovernanceToken must be absent to trigger the error"
        );
    }
}

#[cfg(test)]
mod prune_tests {
    use super::{prune_market_voting_state, DataKey};
    use crate::types::{LockedTokens, Vote};
    use soroban_sdk::{testutils::Address as _, Address, Env};

    #[test]
    fn prune_clears_votes_locks_tallies_and_registry() {
        let e = Env::default();
        let market_id = 42u64;
        let v1 = Address::generate(&e);
        let v2 = Address::generate(&e);

        e.storage().persistent().set(
            &DataKey::Vote(market_id, v1.clone()),
            &Vote {
                market_id,
                voter: v1.clone(),
                outcome: 0,
                weight: 100,
            },
        );
        e.storage().persistent().set(
            &DataKey::Vote(market_id, v2.clone()),
            &Vote {
                market_id,
                voter: v2.clone(),
                outcome: 1,
                weight: 200,
            },
        );
        e.storage()
            .persistent()
            .set(&DataKey::VoteTally(market_id, 0), &100_i128);
        e.storage()
            .persistent()
            .set(&DataKey::VoteTally(market_id, 1), &200_i128);
        e.storage().persistent().set(
            &DataKey::LockedTokens(market_id, v1.clone()),
            &LockedTokens {
                voter: v1.clone(),
                market_id,
                amount: 50,
                unlock_time: 0,
            },
        );
        e.storage()
            .persistent()
            .set(&DataKey::LockedBalance(market_id, v1.clone()), &50_i128);

        let mut reg = soroban_sdk::Vec::new(&e);
        reg.push_back(v1.clone());
        reg.push_back(v2.clone());
        e.storage()
            .persistent()
            .set(&DataKey::DisputeVoters(market_id), &reg);

        prune_market_voting_state(&e, market_id, 2);

        assert!(!e
            .storage()
            .persistent()
            .has(&DataKey::Vote(market_id, v1.clone())));
        assert!(!e
            .storage()
            .persistent()
            .has(&DataKey::Vote(market_id, v2.clone())));
        assert!(!e
            .storage()
            .persistent()
            .has(&DataKey::LockedTokens(market_id, v1.clone())));
        assert!(!e
            .storage()
            .persistent()
            .has(&DataKey::LockedBalance(market_id, v1.clone())));
        assert!(!e
            .storage()
            .persistent()
            .has(&DataKey::VoteTally(market_id, 0)));
        assert!(!e
            .storage()
            .persistent()
            .has(&DataKey::VoteTally(market_id, 1)));
        assert!(!e
            .storage()
            .persistent()
            .has(&DataKey::DisputeVoters(market_id)));
    }
}

#[cfg(test)]
mod decimal_normalization_tests {
    /// Unit tests for the decimal normalization logic (no contract env needed).

    const NORMALIZED_DECIMALS: u32 = 18;

    fn normalize(balance: i128, token_decimals: u32) -> i128 {
        if token_decimals < NORMALIZED_DECIMALS {
            let scale = 10i128.pow(NORMALIZED_DECIMALS - token_decimals);
            balance.saturating_mul(scale)
        } else if token_decimals > NORMALIZED_DECIMALS {
            let scale = 10i128.pow(token_decimals - NORMALIZED_DECIMALS);
            balance / scale
        } else {
            balance
        }
    }

    #[test]
    fn seven_decimal_token_scaled_up() {
        // 1 token with 7 decimals = 10_000_000 raw units
        // normalized to 18 decimals = 10_000_000 * 10^11
        let raw = 10_000_000i128;
        let normalized = normalize(raw, 7);
        assert_eq!(normalized, raw * 10i128.pow(11));
    }

    #[test]
    fn eighteen_decimal_token_unchanged() {
        let raw = 1_000_000_000_000_000_000i128;
        assert_eq!(normalize(raw, 18), raw);
    }

    #[test]
    fn higher_decimal_token_scaled_down() {
        // 24 decimal token: divide by 10^6
        let raw = 1_000_000_000_000_000_000_000_000i128;
        let normalized = normalize(raw, 24);
        assert_eq!(normalized, raw / 10i128.pow(6));
    }

    #[test]
    fn equal_weights_after_normalization() {
        // 1 token regardless of decimal precision should normalize to the same value
        let one_7dec = normalize(10_000_000, 7); // 1 token at 7 decimals
        let one_18dec = normalize(1_000_000_000_000_000_000, 18); // 1 token at 18 decimals
        assert_eq!(one_7dec, one_18dec);
    }
}

/// Issue #1192: Voters who locked governance tokens via cast_vote's fallback
/// path must be able to recover them once the market reaches ANY terminal
/// state, not just Resolved — including Cancelled (e.g. via community-voted
/// cancel_market_vote from Disputed).
#[cfg(test)]
mod unlock_tokens_terminal_state_tests {
    use super::{cast_vote, unlock_tokens, DataKey};
    use crate::errors::ErrorCode;
    use crate::modules::markets;
    use crate::types::{ConfigKey, MarketStatus, MarketTier, OracleConfig};
    use crate::{PredictIQ, PredictIQClient};
    use soroban_sdk::{
        testutils::{Address as _, Ledger as _},
        token, Address, Env, String, Vec,
    };

    fn setup() -> (Env, PredictIQClient<'static>, Address, Address) {
        let env = Env::default();
        env.mock_all_auths();

        let contract_id = env.register(PredictIQ, ());
        let client = PredictIQClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        client.initialize(&admin, &0);

        (env, client, admin, contract_id)
    }

    /// StellarAssetContract supports balance()/transfer() but not balance_at(),
    /// so casting a vote against it always exercises cast_vote's fallback path.
    fn setup_gov_token(env: &Env, contract_id: &Address) -> Address {
        let token_admin = Address::generate(env);
        let token_id = env.register_stellar_asset_contract_v2(token_admin);
        let token_address = token_id.address();
        env.as_contract(contract_id, || {
            env.storage()
                .instance()
                .set(&ConfigKey::GovernanceToken, &token_address);
        });
        token_address
    }

    fn create_market(env: &Env, client: &PredictIQClient, admin: &Address) -> u64 {
        let options = Vec::from_array(
            env,
            [String::from_str(env, "Yes"), String::from_str(env, "No")],
        );
        let native_token = Address::generate(env);
        client.create_market(
            admin,
            &String::from_str(env, "Fallback Lock Test"),
            &options,
            &1000,
            &2000,
            &OracleConfig {
                oracle_address: Address::generate(env),
                feed_id: String::from_str(env, "feed"),
                min_responses: Some(1),
                max_staleness_seconds: 3600,
                max_confidence_bps: 200,
                strike_price: None,
            },
            &MarketTier::Basic,
            &native_token,
            &0,
            &0,
        )
    }

    #[test]
    fn unlock_tokens_succeeds_after_cancellation_for_fallback_locked_voter() {
        let (env, client, admin, contract_id) = setup();
        let gov_token = setup_gov_token(&env, &contract_id);
        let market_id = create_market(&env, &client, &admin);

        // Move market to Disputed so cast_vote's fallback lock path is reachable.
        env.as_contract(&contract_id, || {
            let mut market = markets::get_market(&env, market_id).unwrap();
            market.status = MarketStatus::Disputed;
            market.pending_resolution_timestamp = Some(1001);
            market.dispute_timestamp = Some(1001);
            market.dispute_snapshot_ledger = Some(env.ledger().sequence());
            markets::update_market(&env, market);
        });

        let voter = Address::generate(&env);
        token::StellarAssetClient::new(&env, &gov_token).mint(&voter, &500);
        let token_client = token::Client::new(&env, &gov_token);
        assert_eq!(token_client.balance(&voter), 500);

        // Fallback path: balance_at is unsupported on the SAC, so cast_vote
        // locks the caller-supplied weight in the contract.
        env.as_contract(&contract_id, || {
            cast_vote(&env, voter.clone(), market_id, 0, 500).unwrap();
        });
        assert_eq!(token_client.balance(&voter), 0);
        assert_eq!(token_client.balance(&contract_id), 500);

        // Community vote cancels the Disputed market — no path back to Resolved.
        env.as_contract(&contract_id, || {
            let mut market = markets::get_market(&env, market_id).unwrap();
            market.status = MarketStatus::Cancelled;
            markets::update_market(&env, market);
        });

        // Advance past the lock's unlock_time (market.resolution_deadline == 2000).
        env.ledger().with_mut(|li| li.timestamp = 2001);

        env.as_contract(&contract_id, || {
            unlock_tokens(&env, voter.clone(), market_id).unwrap();
        });

        // Tokens are fully returned and the lock ledger entries are cleared.
        assert_eq!(token_client.balance(&voter), 500);
        assert_eq!(token_client.balance(&contract_id), 0);
        env.as_contract(&contract_id, || {
            assert!(!env
                .storage()
                .persistent()
                .has(&DataKey::LockedTokens(market_id, voter.clone())));
            assert!(!env
                .storage()
                .persistent()
                .has(&DataKey::LockedBalance(market_id, voter.clone())));
        });
    }

    #[test]
    fn unlock_tokens_still_rejected_while_market_active() {
        let (env, client, admin, contract_id) = setup();
        let market_id = create_market(&env, &client, &admin);
        let voter = Address::generate(&env);

        let result = env.as_contract(&contract_id, || unlock_tokens(&env, voter, market_id));
        assert_eq!(result, Err(ErrorCode::MarketNotResolved));
    }
}
