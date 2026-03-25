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
    /// Immutable after creation — set once in create_market (Issue #23).
    pub payout_mode: PayoutMode,
    pub tier: MarketTier,
    pub creation_deposit: i128,
    pub parent_id: u64,
    pub parent_outcome_idx: u32,
    pub resolved_at: Option<u64>,
    pub token_address: Address,
    pub outcome_stakes: Map<u32, i128>,
    pub pending_resolution_timestamp: Option<u64>,
    pub dispute_snapshot_ledger: Option<u32>,
    /// Timestamp when dispute was filed — used by resolution.rs (Issue #8).
    pub dispute_timestamp: Option<u64>,
    /// Total amount claimed so far — used to guard prune_market (Issue #17).
    pub total_claimed: i128,
    /// Actual winner count per outcome, maintained during place_bet (Issue #24).
    pub winner_counts: Map<u32, u32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PayoutMode {
    Push,
    Pull,
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

/// Issue #16: Added missing fields used in oracles.rs.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub oracle_address: Address,
    pub feed_id: String,
    pub min_responses: Option<u32>,
    /// Maximum age of a price in seconds before it is considered stale.
    /// None defaults to 3600 (1 hour).
    pub max_staleness_seconds: Option<u64>,
    /// Maximum confidence interval as basis points of price (e.g. 100 = 1%).
    /// None defaults to 200 (2%).
    pub max_confidence_bps: Option<u64>,
}

/// Issue #33: Named struct instead of raw tuple for upgrade vote stats.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeStats {
    pub votes_for: u32,
    pub votes_against: u32,
}

pub const MAX_PUSH_PAYOUT_WINNERS: u32 = 50;
/// Hard cap on outcomes per market — bounds iteration cost in finalize_resolution.
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
    /// Issue #3: Was missing — used in voting.rs for governance token address.
    GovernanceToken,
    /// Issue #13: Configurable timelock duration (seconds).
    TimelockDuration,
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

/// Issue #13: Default timelock — 48 hours. Overridable via ConfigKey::TimelockDuration.
pub const TIMELOCK_DURATION: u64 = 48 * 60 * 60;
pub const MAJORITY_THRESHOLD_PERCENT: u32 = 51;

// TTL Management Constants (in ledgers, ~5 seconds per ledger)
pub const TTL_LOW_THRESHOLD: u32 = 17_280;   // ~1 day
/// Issue #36: Raised from 30 days to 90 days so data outlives the prune grace period.
pub const TTL_HIGH_THRESHOLD: u32 = 1_555_200; // ~90 days
pub const PRUNE_GRACE_PERIOD: u64 = 2_592_000; // 30 days in seconds

pub const GOV_TTL_LOW_THRESHOLD: u32 = 1_555_200;  // ~90 days
pub const GOV_TTL_HIGH_THRESHOLD: u32 = 3_110_400; // ~180 days
