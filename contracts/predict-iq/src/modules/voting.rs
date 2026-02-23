use crate::errors::ErrorCode;
use crate::modules::markets;
use crate::types::{MarketStatus, Vote};
use soroban_sdk::{contracttype, Address, Env};

#[contracttype]
pub enum DataKey {
    Vote(u64, Address),  // market_id, voter
    VoteTally(u64, u32), // market_id, outcome -> total_weight
    LockedTokens(u64, Address), // market_id, voter
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
    if e.storage().persistent().has(&vote_key) {
        return Err(ErrorCode::AlreadyVoted);
    }

    let snapshot_ledger = market.dispute_snapshot_ledger.ok_or(ErrorCode::MarketNotDisputed)?;
    
    // Get governance token
    let gov_token: Address = e.storage().instance()
        .get(&ConfigKey::GovernanceToken)
        .ok_or(ErrorCode::GovernanceTokenNotSet)?;
    
    // Try snapshot-based balance first
    let actual_weight = match try_get_balance_at(e, &gov_token, &voter, snapshot_ledger) {
        Ok(balance) => balance,
        Err(_) => {
            // Fallback: lock tokens for 3-day resolution period
            let token_client = token::Client::new(e, &gov_token);
            let current_balance = token_client.balance(&voter);
            if current_balance < weight {
                return Err(ErrorCode::InsufficientVotingWeight);
            }
            
            token_client.transfer(&voter, &e.current_contract_address(), &weight);
            
            let locked = LockedTokens {
                voter: voter.clone(),
                market_id,
                amount: weight,
                unlock_time: market.resolution_deadline,
            };
            e.storage().persistent().set(&DataKey::LockedTokens(market_id, voter.clone()), &locked);
            
            weight
        }
    };

    if actual_weight == 0 {
        return Err(ErrorCode::InsufficientVotingWeight);
    }

    let vote = Vote {
        market_id,
        voter: voter.clone(),
        outcome,
        weight: actual_weight,
    };

    e.storage().persistent().set(&vote_key, &vote);

    let tally_key = DataKey::VoteTally(market_id, outcome);
    let mut current_tally: i128 = e.storage().persistent().get(&tally_key).unwrap_or(0);
    current_tally += actual_weight;
    e.storage().persistent().set(&tally_key, &current_tally);

    // Emit standardized VoteCast event
    // Topics: [VoteCast, market_id, voter]
    crate::modules::events::emit_vote_cast(e, market_id, voter, outcome, weight);

    Ok(())
}

fn try_get_balance_at(e: &Env, token: &Address, account: &Address, ledger: u32) -> Result<i128, ErrorCode> {
    // Try to invoke balance_at method if token supports snapshots
    let args = (account.clone(), ledger).into_val(e);
    
    match e.try_invoke_contract::<Val, ErrorCode>(token, &Symbol::new(e, "balance_at"), args) {
        Ok(Ok(val)) => i128::try_from_val(e, &val).map_err(|_| ErrorCode::OracleFailure),
        _ => Err(ErrorCode::OracleFailure),
    }
}

pub fn unlock_tokens(e: &Env, voter: Address, market_id: u64) -> Result<(), ErrorCode> {
    voter.require_auth();
    
    let lock_key = DataKey::LockedTokens(market_id, voter.clone());
    let locked: LockedTokens = e.storage().persistent()
        .get(&lock_key)
        .ok_or(ErrorCode::BetNotFound)?;
    
    if e.ledger().timestamp() < locked.unlock_time {
        return Err(ErrorCode::VotingNotStarted);
    }
    
    let gov_token: Address = e.storage().instance()
        .get(&ConfigKey::GovernanceToken)
        .ok_or(ErrorCode::GovernanceTokenNotSet)?;
    
    let token_client = token::Client::new(e, &gov_token);
    token_client.transfer(&e.current_contract_address(), &voter, &locked.amount);
    
    e.storage().persistent().remove(&lock_key);
    
    Ok(())
}

pub fn get_tally(e: &Env, market_id: u64, outcome: u32) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::VoteTally(market_id, outcome))
        .unwrap_or(0)
}
