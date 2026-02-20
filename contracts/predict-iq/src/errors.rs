use soroban_sdk::contracterror;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum ErrorCode {
    AlreadyInitialized = 100,
    NotAuthorized = 101,
    MarketNotFound = 102,
    MarketClosed = 103,
    MarketStillActive = 104,
    InvalidOutcome = 105,
    InvalidBetAmount = 106,
    InsufficientBalance = 107,
    OracleFailure = 108,
    CircuitBreakerOpen = 109,
    DisputeWindowClosed = 110,
    VotingNotStarted = 111,
    VotingEnded = 112,
    AlreadyVoted = 113,
    FeeTooHigh = 114,
    MarketNotActive = 115,
    DeadlinePassed = 116,
    CannotChangeOutcome = 117,
    MarketNotDisputed = 118,
    MarketNotPendingResolution = 119,
    AdminNotSet = 120,
    BetNotFound = 121,
    AlreadyClaimed = 122,
    NotWinningOutcome = 123,
    InsufficientVotingWeight = 124,
    GovernanceTokenNotSet = 125,
    DisputeWindowStillOpen = 126,
    ResolutionNotReady = 127,
    NoMajorityReached = 128,
    MarketNotCancelled = 129,
}
