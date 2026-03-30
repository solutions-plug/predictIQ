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
    pub winner_counts: Map<u32, u32>,          // Issue #24: unique bettor count per outcome
    pub total_claimed: i128,                   // Issue #17: total winnings claimed (for prune guard)
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
    pub amount: i128,   // net amount after fee (used for payout calculations)
    pub fee_paid: i128, // protocol fee deducted at bet time (needed for cancellation reversal)
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
/// Issue #25: oracle_address and feed_id are used for the live cross-contract call.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleConfig {
    pub oracle_address: Address,    // Deployed Pyth contract address on this network
    pub feed_id: String,            // 64-char hex-encoded 32-byte Pyth price feed ID
    pub min_responses: Option<u32>, // Minimum oracle responses required; None defaults to 1
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
    GuardianAccount,
    BaseFee,
    /// Optional dedicated account allowed to withdraw protocol fees (Issue #26).
    FeeAdmin,
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
    /// Issue #8: Configurable dispute window duration in seconds.
    DisputeWindow,
    /// Issue #170: Configurable voting period duration in seconds.
    VotingPeriod,
    /// Issue #170: Configurable majority threshold in basis points.
    MajorityThreshold,
    /// Effective upgrade timelock duration in seconds (governance override).
    /// Issue #403: Added missing variant referenced by governance module.
    TimelockDuration,
    /// Issue #406: Status index key — maps (status_tag, market_id) for O(1) status queries.
    StatusIndex(u32, u64),
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

/// Issue #404: Pending guardian removal proposal (was missing from types.rs).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingGuardianRemoval {
    pub target_guardian: Address,
    pub initiated_at: u64,
    pub votes_for: Vec<Address>,
}

/// Issue #404: Vote counts surfaced for upgrade proposals (`get_upgrade_votes`).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UpgradeStats {
    pub votes_for: u32,
    pub votes_against: u32,
}

/// Issue #13: Default timelock — 48 hours.
pub const TIMELOCK_DURATION: u64 = 48 * 60 * 60;
/// Issue #403: Bounds for configurable upgrade timelock (6 hours … 7 days).
pub const TIMELOCK_MIN_SECONDS: u64 = 6 * 60 * 60;
pub const TIMELOCK_MAX_SECONDS: u64 = 7 * 24 * 60 * 60;
pub const MAJORITY_THRESHOLD_PERCENT: u32 = 51;
pub const UPGRADE_COOLDOWN_DURATION: u64 = 7 * 24 * 60 * 60; // 7 days

// TTL Management Constants (in ledgers, ~5 seconds per ledger)
pub const TTL_LOW_THRESHOLD: u32 = 17_280;     // ~1 day
/// Issue #36: Raised from 30 days to 90 days so data outlives the prune grace period.
pub const TTL_HIGH_THRESHOLD: u32 = 1_555_200; // ~90 days
pub const PRUNE_GRACE_PERIOD: u64 = 2_592_000; // 30 days in seconds

/// Issue #100: Bet records must survive the full market lifecycle including
/// extended dispute windows. A market can remain Disputed for up to 72 hours
/// of voting, plus any admin fallback period. We set bet TTL to ~180 days so
/// a record placed on day 0 is still readable when the winner claims on day 90+.
/// Low threshold triggers a refresh when fewer than 90 days remain.
pub const BET_TTL_LOW_THRESHOLD: u32 = 1_555_200;  // ~90 days  — refresh trigger
pub const BET_TTL_HIGH_THRESHOLD: u32 = 3_110_400; // ~180 days — target lifetime

pub const GOV_TTL_LOW_THRESHOLD: u32 = 1_555_200;  // ~90 days
pub const GOV_TTL_HIGH_THRESHOLD: u32 = 3_110_400; // ~180 days

/// Issue #54: Reserved sentinel index for cancellation votes, distinct from any valid outcome index.
pub const CANCEL_OUTCOME_INDEX: u32 = u32::MAX;

/// Maps MarketStatus to a stable u32 tag for use in StatusIndex keys.
pub fn status_tag(status: &MarketStatus) -> u32 {
    match status {
        MarketStatus::Active => 0,
        MarketStatus::PendingResolution => 1,
        MarketStatus::Disputed => 2,
        MarketStatus::Resolved => 3,
        MarketStatus::Cancelled => 4,
    }
}
