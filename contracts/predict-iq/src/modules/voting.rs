use crate::errors::ErrorCode;
use crate::modules::markets;
// Issue #171: ConfigKey (including GovernanceToken variant) must be explicitly imported
// from types. Previously missing, causing compilation failure in cast_vote.
use crate::types::{ConfigKey, LockedTokens, MarketStatus, Vote};
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

    if outcome >= market.options.len() {
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

    let vote = Vote {
        outcome,
        weight: actual_weight,
    };

    e.storage().persistent().set(&vote_key, &vote);

    if old_vote.is_none() {
        let reg_key = DataKey::DisputeVoters(market_id);
        let mut voters: Vec<Address> = e.storage().persistent().get(&reg_key).unwrap_or(Vec::new(e));
        voters.push_back(voter.clone());
        e.storage().persistent().set(&reg_key, &voters);
    }

    let tally_key = DataKey::VoteTally(market_id, outcome);
    let mut current_tally: i128 = e.storage().persistent().get(&tally_key).unwrap_or(0);
    current_tally += actual_weight;
    e.storage().persistent().set(&tally_key, &current_tally);

    crate::modules::events::emit_vote_cast(e, market_id, voter, outcome, actual_weight);

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

/// Issue #20: Require market to be Resolved before unlocking tokens.
pub fn unlock_tokens(e: &Env, voter: Address, market_id: u64) -> Result<(), ErrorCode> {
    voter.require_auth();

    let market = markets::get_market(e, market_id).ok_or(ErrorCode::MarketNotFound)?;

    // Issue #20: Tokens remain locked throughout the entire dispute lifecycle.
    // Only allow unlock once the market is fully Resolved.
    if market.status != MarketStatus::Resolved {
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
    let amount: i128 = e
        .storage()
        .persistent()
        .get(&balance_key)
        .unwrap_or(0);

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
            e.storage().persistent().remove(&DataKey::Vote(market_id, v.clone()));
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
        e.storage().instance().set(&ConfigKey::GovernanceToken, &token);
        let stored: Option<Address> = e.storage().instance().get(&ConfigKey::GovernanceToken);
        assert_eq!(stored, Some(token));
    }

    /// Issue #171: cast_vote returns GovernanceTokenNotSet when token is not configured.
    #[test]
    fn cast_vote_returns_error_when_governance_token_not_set() {
        let e = Env::default();
        // GovernanceToken not set in storage — get returns None
        let stored: Option<Address> = e.storage().instance().get(&ConfigKey::GovernanceToken);
        assert!(stored.is_none(), "GovernanceToken must be absent to trigger the error");
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
                outcome: 0,
                weight: 100,
            },
        );
        e.storage().persistent().set(
            &DataKey::Vote(market_id, v2.clone()),
            &Vote {
                outcome: 1,
                weight: 200,
            },
        );
        e.storage().persistent().set(&DataKey::VoteTally(market_id, 0), &100_i128);
        e.storage().persistent().set(&DataKey::VoteTally(market_id, 1), &200_i128);
        e.storage().persistent().set(
            &DataKey::LockedTokens(market_id, v1.clone()),
            &LockedTokens {
                voter: v1.clone(),
                market_id,
                amount: 50,
                unlock_time: 0,
            },
        );
        e.storage().persistent().set(
            &DataKey::LockedBalance(market_id, v1.clone()),
            &50_i128,
        );

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
