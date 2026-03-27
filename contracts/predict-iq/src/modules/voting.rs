use crate::errors::ErrorCode;
use crate::modules::markets;
use crate::types::{ConfigKey, LockedTokens, MarketStatus, Vote};
use soroban_sdk::{contracttype, token, Address, Env, Symbol, Val};
use soroban_sdk::{contracttype, token, Address, Env, Symbol};

#[contracttype]
#[derive(Clone)]
pub enum DataKey {
    Vote(u64, Address),         // market_id, voter
    VoteTally(u64, u32),        // market_id, outcome -> total_weight
    LockedTokens(u64, Address), // market_id, voter
    /// Issue #37: Per-user locked balance ledger to prevent pool drain.
    LockedBalance(u64, Address), // market_id, voter -> amount
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
    if let Some(old_vote_data) = old_vote {
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
    use soroban_sdk::{IntoVal, TryFromVal};
    let args: soroban_sdk::Vec<Val> =
        soroban_sdk::vec![e, account.clone().into_val(e), ledger.into_val(e)];

    match e.try_invoke_contract::<Val, ErrorCode>(token, &Symbol::new(e, "balance_at"), args) {
        Ok(Ok(val)) => i128::try_from_val(e, &val).map_err(|_| ErrorCode::OracleFailure),
    use soroban_sdk::{IntoVal, Val};
    let args: soroban_sdk::Vec<Val> =
        soroban_sdk::vec![e, account.clone().into_val(e), ledger.into_val(e),];

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

    let gov_token: Address = e
        .storage()
        .instance()
        .get(&ConfigKey::GovernanceToken)
        .ok_or(ErrorCode::GovernanceTokenNotSet)?;

    let token_client = token::Client::new(e, &gov_token);
    e.current_contract_address().require_auth();
    token_client.transfer(&e.current_contract_address(), &voter, &locked.amount);

    e.storage().persistent().remove(&lock_key);
    e.storage()
        .persistent()
        .remove(&DataKey::LockedBalance(market_id, voter));

    Ok(())
}

pub fn get_tally(e: &Env, market_id: u64, outcome: u32) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::VoteTally(market_id, outcome))
        .unwrap_or(0)
}
