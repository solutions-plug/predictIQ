use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    // --- Initialization & Auth ---
    AlreadyInitialized = 1,
    NotAuthorized = 2,
    GuardianNotSet = 3,

    // --- Market lifecycle ---
    MarketNotFound = 4,
    MarketClosed = 5,
    MarketStillActive = 6,
    MarketNotActive = 7,
    MarketNotResolved = 8,
    MarketNotDisputed = 9,
    MarketNotPendingResolution = 10,
    CannotChangeOutcome = 11,
    InvalidDeadline = 12,
    DeadlinePassed = 13,
    ResolutionDeadlinePassed = 14,
    ResolutionNotReady = 15,
    GracePeriodActive = 16,

    // --- Market IDs ---
    MarketIdOverflow = 17,
    MarketIdCollision = 18,

    // --- Outcomes & options ---
    InvalidOutcome = 19,
    TooManyOutcomes = 20,

    // --- Bets & balances ---
    InvalidBetAmount = 21,
    InsufficientBalance = 22,
    InsufficientDeposit = 23,
    InvalidAmount = 24,
    BetNotFound = 25,
    NoWinnings = 26,
    AlreadyClaimed = 27,

    // --- Oracle & price ---
    OracleFailure = 28,
    /// Emitted when a price feed's publish_time is older than max_staleness_seconds.
    StalePrice = 29,
    /// Emitted when the oracle confidence interval exceeds the configured threshold.
    ConfidenceTooLow = 30,
    InvalidTimestamp = 31,

    // --- SAC / asset safety ---
    /// Emitted when a Stellar asset subject to clawback has been reclaimed,
    /// leaving the contract with less balance than expected.
    AssetClawedBack = 32,
    /// Emitted when a SAC token transfer fails programmatically instead of panicking.
    TransferFailed = 33,

    // --- Disputes & voting ---
    DisputeWindowClosed = 34,
    DisputeWindowStillOpen = 35,
    AlreadyVoted = 36,
    InsufficientVotes = 37,
    InsufficientVotingWeight = 38,
    NoMajorityReached = 39,
    GovernanceTokenNotSet = 40,

    // --- Timelocks & upgrades ---
    TimelockActive = 41,
    UpgradeNotInitiated = 42,
    AlreadyVotedOnUpgrade = 43,
    UpgradeAlreadyPending = 44,
    UpgradeHashInCooldown = 45,

    // --- Conditional markets ---
    ParentMarketNotResolved = 46,
    ParentMarketInvalidOutcome = 47,

    // --- Circuit breaker & referrals ---
    ContractPaused = 48,
    InvalidReferrer = 49,

    // --- Governance ---
    /// Emitted when an admin attempts fallback resolution but the voting period
    /// has not yet elapsed — the deadlock is not yet confirmed.
    VotingPeriodNotElapsed = 50,

    // --- Arithmetic safety ---
    /// Emitted when checked arithmetic operations overflow (Issue #192)
    ArithmeticOverflow = 51,

    // --- Configuration validation ---
    /// Emitted when an invalid threshold is provided (Issue #170)
    InvalidThreshold = 52,

    // --- Payout mode ---
    /// Issue #182: payout_mode cannot be changed once the resolution process
    /// has started (i.e. status is no longer Active).
    PayoutModeLocked = 53,
}
