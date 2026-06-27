use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    /// The contract has already been initialized and cannot be initialized again.
    AlreadyInitialized = 100,

    /// The caller does not have the required authorization to perform this action.
    NotAuthorized = 101,

    /// No market exists with the given ID.
    MarketNotFound = 102,

    /// The market has been closed and no longer accepts bets or updates.
    MarketClosed = 103,

    /// The market is still active and cannot be resolved or finalized yet.
    MarketStillActive = 104,

    /// The provided outcome index is not valid for this market.
    InvalidOutcome = 105,

    /// The bet amount is zero, negative, or otherwise outside the allowed range.
    InvalidBetAmount = 106,

    /// The caller's token balance is too low to cover the requested operation.
    InsufficientBalance = 107,

    /// The oracle failed to provide a result or returned an unreadable response.
    OracleFailure = 108,

    /// The circuit breaker is open due to repeated failures; operations are temporarily halted.
    CircuitBreakerOpen = 109,

    /// The dispute window for this market has already closed; disputes can no longer be filed.
    DisputeWindowClosed = 110,

    /// Voting on this market has not started yet.
    VotingNotStarted = 111,

    /// The voting period for this market has ended.
    VotingEnded = 112,

    /// The caller has already cast a vote on this market and cannot vote again.
    AlreadyVoted = 113,

    /// The requested fee exceeds the maximum allowed fee threshold.
    FeeTooHigh = 114,

    /// The market is not in an active state; it may be pending, closed, or resolved.
    MarketNotActive = 115,

    /// The submission deadline for this market has passed.
    DeadlinePassed = 116,

    /// The outcome for this market has already been set and cannot be changed.
    CannotChangeOutcome = 117,

    /// The market is not in a disputed state; dispute-specific operations are unavailable.
    MarketNotDisputed = 118,

    /// The market is not pending resolution; resolution cannot proceed at this time.
    MarketNotPendingResolution = 119,

    /// No admin address has been configured for this contract.
    AdminNotSet = 120,

    /// The contract is paused; all state-changing operations are disabled.
    ContractPaused = 121,

    /// No guardian address has been configured for this contract.
    GuardianNotSet = 122,

    /// The number of outcomes provided exceeds the maximum allowed per market.
    TooManyOutcomes = 123,

    /// The number of winning outcomes exceeds the maximum allowed for payout calculation.
    TooManyWinners = 124,

    /// The requested payout mode is not supported by this contract version.
    PayoutModeNotSupported = 125,

    /// The deposit provided is below the minimum required amount.
    InsufficientDeposit = 126,

    /// A timelock is currently active; the operation must wait until the timelock expires.
    TimelockActive = 127,

    /// No upgrade has been initiated; upgrade-related operations cannot proceed.
    UpgradeNotInitiated = 128,

    /// There are not enough governance votes to approve the requested action.
    InsufficientVotes = 129,

    /// The caller has already voted on this upgrade proposal.
    AlreadyVotedOnUpgrade = 130,

    /// The provided WASM hash is malformed or does not match the expected format.
    InvalidWasmHash = 131,

    /// The contract upgrade process failed; the new WASM could not be applied.
    UpgradeFailed = 132,

    /// The parent market has not been resolved yet; this conditional market cannot proceed.
    ParentMarketNotResolved = 133,

    /// The parent market resolved to an outcome that does not satisfy this market's condition.
    ParentMarketInvalidOutcome = 134,

    /// The resolution conditions have not been met yet; try again later.
    ResolutionNotReady = 135,

    /// The dispute window is still open; resolution must wait until it closes.
    DisputeWindowStillOpen = 136,

    /// No majority outcome was reached among the votes cast; resolution is inconclusive.
    NoMajorityReached = 137,

    /// The oracle price data is too old and considered stale; a fresh price feed is required.
    StalePrice = 138,

    /// The oracle's confidence score is below the minimum threshold required for resolution.
    ConfidenceTooLow = 139,

    /// The caller's governance token balance is too low to meet the minimum voting weight.
    InsufficientVotingWeight = 140,

    /// The market was not cancelled; refund or cancellation-specific operations are unavailable.
    MarketNotCancelled = 141,

    /// No bet was found for the given bet ID or caller address.
    BetNotFound = 142,

    /// An upgrade is already pending approval; only one upgrade can be in flight at a time.
    UpgradeAlreadyPending = 143,

    /// This WASM hash was recently used and is still within its cooldown period.
    UpgradeHashInCooldown = 144,

    /// The provided amount is invalid (e.g. zero, negative, or exceeds allowed limits).
    InvalidAmount = 145,

    /// No governance token contract address has been configured.
    GovernanceTokenNotSet = 146,

    /// The market has not been resolved yet; payout or post-resolution operations are unavailable.
    MarketNotResolved = 147,

    /// The provided deadline is in the past or otherwise invalid.
    InvalidDeadline = 148,

    PendingTransferNotFound = 149,
    NotPendingOwner = 150,
    TokenFrozen = 151,
    MigrationValidationError = 152,
    AssetClawedBack = 153,
    ArithmeticOverflow = 154,
    AlreadyClaimed = 155,
    NoWinnings = 156,
    InvalidReferrer = 157,
    ResolutionDeadlinePassed = 158,
    Overflow = 159,
    InvalidTimeRange = 160,
}
