use soroban_sdk::{Env, Address, contracttype};
use crate::types::{Vote, MarketStatus};
use crate::modules::markets;
use crate::errors::ErrorCode;

#[contracttype]
pub enum DataKey {
    Vote(u64, Address), // market_id, voter
    VoteTally(u64, u32), // market_id, outcome -> total_weight
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

    let vote = Vote {
        market_id,
        voter: voter.clone(),
        outcome,
        weight,
    };

    e.storage().persistent().set(&vote_key, &vote);

    let tally_key = DataKey::VoteTally(market_id, outcome);
    let mut current_tally: i128 = e.storage().persistent().get(&tally_key).unwrap_or(0);
    current_tally += weight;
    e.storage().persistent().set(&tally_key, &current_tally);

    // Emit standardized VoteCast event
    // Topics: [VoteCast, market_id, voter]
    crate::modules::events::emit_vote_cast(e, market_id, voter, outcome, weight);
    
    Ok(())
}

pub fn get_tally(e: &Env, market_id: u64, outcome: u32) -> i128 {
    e.storage().persistent().get(&DataKey::VoteTally(market_id, outcome)).unwrap_or(0)
}
