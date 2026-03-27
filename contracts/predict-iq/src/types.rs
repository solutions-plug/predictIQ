use soroban_sdk::{contracttype, Address, BytesN, Map, String, Vec};

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
    pub payout_mode: PayoutMode,
    pub tier: MarketTier,
    pub creation_deposit: i128,
    pub parent_id: u64,                        // 0 means no parent (independent market)
    pub parent_outcome_idx: u32,               // Required outcome of parent market
    pub resolved_at: Option<u64>,              // Timestamp when market was resolved (for TTL pruning)
    pub token_address: Address,                // Token used for betting
    pub outcome_stakes: Map<u32, i128>,        // Stake per outcome
    pub pending_resolution_timestamp: Option<u64>, // Timestamp when resolution was initiated
    pub dispute_snapshot_ledger: Option<u32>,  // Ledger sequence for snapshot voting
    pub dispute_timestamp: Option<u64>,        // Timestamp when dispute was filed
<<<<<<< fix/issue-24-precise-winner-tracking
    /// Issue #24: Precise per-outcome winner counter incremented on every new
    /// unique bettor. Replaces the unsafe tally/100 heuristic in disputes.rs.
    pub winner_counts: Map<u32, u32>,          // outcome -> unique bettor count
=======
>>>>>>> main
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayoutMode {
    Push, // Reserved compatibility flag; automatic push distribution is not implemented
    Pull, // Active payout path: winners claim individually via claim_winnings
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
pub struct CreatorReputation {
    pub score: u32, // Reputation score (0-1000+)
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

/// Issue #16: Oracle configuration for Pyth price feed integration.
<<<<<<< fix/issue-24-precise-winner-tracking
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub oracle_address: Address,    // Deployed Pyth contract address on this network
    pub feed_id: String,            // 64-char hex-encoded 32-byte Pyth price feed ID
    pub min_responses: u32,         // Minimum oracle responses required (default: 1)
    pub max_staleness_seconds: u64, // Max age of price data in seconds (default: 300)
    pub max_confidence_bps: u64,    // Max confidence interval in basis points (default: 200 = 2%)
=======
/// Issue #25: oracle_address and feed_id are used for the live cross-contract call.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub oracle_address: Address,     // Deployed Pyth contract address on this network
    pub feed_id: String,             // 64-char hex-encoded 32-byte Pyth price feed ID
    pub min_responses: u32,          // Minimum oracle responses required (default: 1)
    pub max_staleness_seconds: u64,  // Max age of price data in seconds (default: 300)
    pub max_confidence_bps: u64,     // Max confidence interval in basis points (default: 200 = 2%)
>>>>>>> main
    pub oracle_address: Address,
    pub feed_id: String,
    pub min_responses: Option<u32>, // Optimized: None defaults to 1
    pub max_staleness_seconds: u64, // Max age of price data in seconds
    pub max_confidence_bps: u64,    // Max confidence interval in basis points
}

// Gas optimization constants
pub const MAX_PUSH_PAYOUT_WINNERS: u32 = 50;
/// Hard cap on outcomes per market — bounds iteration cost in `calculate_voting_outcome`.
pub const MAX_OUTCOMES_PER_MARKET: u32 = 32;

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
    UpgradeRejectedAt(BytesN<32>),
    GovernanceToken,
    MaxPushPayoutWinners,
    PendingGuardianRemoval,
    MinimumBetAmount,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CircuitBreakerState {
    Closed,
    Open,
    HalfOpen,
    Paused,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Guardian {
    pub address: Address,
    pub voting_power: u32,
}

/// Issue #32: wasm_hash changed from String to BytesN<32>.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingUpgrade {
    pub wasm_hash: BytesN<32>,
    pub initiated_at: u64,
    pub votes_for: Vec<Address>,
    pub votes_against: Vec<Address>,
}

/// Issue #13: Default timelock — 48 hours.
pub const TIMELOCK_DURATION: u64 = 48 * 60 * 60;
pub const MAJORITY_THRESHOLD_PERCENT: u32 = 51;
pub const UPGRADE_COOLDOWN_DURATION: u64 = 7 * 24 * 60 * 60; // 7 days

// TTL Management Constants (in ledgers, ~5 seconds per ledger)
pub const TTL_LOW_THRESHOLD: u32 = 17_280;     // ~1 day
/// Issue #36: Raised from 30 days to 90 days so data outlives the prune grace period.
pub const TTL_HIGH_THRESHOLD: u32 = 1_555_200; // ~90 days
pub const PRUNE_GRACE_PERIOD: u64 = 2_592_000; // 30 days in seconds

pub const GOV_TTL_LOW_THRESHOLD: u32 = 1_555_200;  // ~90 days
pub const GOV_TTL_HIGH_THRESHOLD: u32 = 3_110_400; // ~180 days
