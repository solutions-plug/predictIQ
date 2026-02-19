use soroban_sdk::{contracttype, Address, String, Vec, Map};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketStatus {
    Active,
    PendingResolution,
    Disputed,
    Resolved,
    Cancelled,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Market {
    pub id: u64,
    pub creator: Address,
    pub description: String,
    pub options: Vec<String>,
    pub status: MarketStatus,
    pub deadline: u64,
    pub resolution_deadline: u64,
    pub winning_outcome: Option<u32>,
    pub oracle_config: OracleConfig,
    pub total_staked: i128,
    pub outcome_stakes: Map<u32, i128>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Bet {
    pub market_id: u64,
    pub bettor: Address,
    pub outcome: u32,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Vote {
    pub market_id: u64,
    pub voter: Address,
    pub outcome: u32,
    pub weight: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub oracle_address: Address,
    pub feed_id: String,
    pub min_responses: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigKey {
    Admin,
    MarketAdmin,
    FeeAdmin,
    BaseFee,
    CircuitBreakerState,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
}
