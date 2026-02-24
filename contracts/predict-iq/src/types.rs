use soroban_sdk::{contracttype, Address, Map, String, Vec};

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
    pub payout_mode: PayoutMode, // New: determines push vs pull payouts
    pub tier: MarketTier,
    pub creation_deposit: i128,
    pub parent_id: u64,          // 0 means no parent (independent market)
    pub parent_outcome_idx: u32, // Required outcome of parent market
    pub resolved_at: Option<u64>, // Timestamp when market was resolved (for TTL pruning)
    pub token_address: Address,   // Token used for betting
    pub outcome_stakes: Map<u32, i128>, // Stake per outcome
    pub pending_resolution_timestamp: Option<u64>, // Timestamp when resolution was initiated
    pub dispute_snapshot_ledger: Option<u32>, // Ledger sequence for snapshot voting
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayoutMode {
    Push, // Contract distributes to all winners (small markets)
    Pull, // Winners claim individually (large markets)
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MarketTier {
    Basic,
    Pro,
    Institutional,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CreatorReputation {
    None,
    Basic,
    Pro,
    Institutional,
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
pub struct LockedTokens {
    pub voter: Address,
    pub market_id: u64,
    pub amount: i128,
    pub unlock_time: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub oracle_address: Address,
    pub feed_id: String,
    pub min_responses: Option<u32>, // Optimized: None defaults to 1
}

// Gas optimization constants
pub const MAX_PUSH_PAYOUT_WINNERS: u32 = 50; // Threshold for switching to pull mode
pub const MAX_OUTCOMES_PER_MARKET: u32 = 100; // Limit to prevent excessive iteration

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigKey {
    Admin,
    MarketAdmin,
    FeeAdmin,
    GuardianAccount,
    BaseFee,
    CircuitBreakerState,
    CreationDeposit,
    GuardianSet,
    PendingUpgrade,
    UpgradeVotes,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
    Paused, // Emergency pause state - blocks high-risk operations
}

// Governance and Upgrade Types
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Guardian {
    pub address: Address,
    pub voting_power: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: String,
    pub initiated_at: u64,
    pub votes_for: Vec<Address>,
    pub votes_against: Vec<Address>,
}

// Constants for upgrade governance
pub const TIMELOCK_DURATION: u64 = 48 * 60 * 60; // 48 hours in seconds
pub const MAJORITY_THRESHOLD_PERCENT: u32 = 51; // 51% for majority

// TTL Management Constants (in ledgers, ~5 seconds per ledger)
pub const TTL_LOW_THRESHOLD: u32 = 17_280; // ~1 day (86400 seconds / 5)
pub const TTL_HIGH_THRESHOLD: u32 = 518_400; // ~30 days (2592000 seconds / 5)
pub const PRUNE_GRACE_PERIOD: u64 = 2_592_000; // 30 days in seconds
