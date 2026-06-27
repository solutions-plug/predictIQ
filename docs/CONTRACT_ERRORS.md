# PredictIQ Contract Error Reference

Error codes returned by the `predict-iq` Soroban smart contract.  
Source of truth: [`contracts/predict-iq/src/errors.rs`](../contracts/predict-iq/src/errors.rs)

When a contract call fails, the Soroban SDK surfaces the error as a `u32` value.
The table below maps each code to its variant name and a human-readable description.

| Code | Variant | Description |
|------|---------|-------------|
| 100 | `AlreadyInitialized` | The contract has already been initialized and cannot be initialized again. |
| 101 | `NotAuthorized` | The caller does not have the required authorization to perform this action. |
| 102 | `MarketNotFound` | No market exists with the given ID. |
| 103 | `MarketClosed` | The market has been closed and no longer accepts bets or updates. |
| 104 | `MarketStillActive` | The market is still active and cannot be resolved or finalized yet. |
| 105 | `InvalidOutcome` | The provided outcome index is not valid for this market. |
| 106 | `InvalidBetAmount` | The bet amount is zero, negative, or otherwise outside the allowed range. |
| 107 | `InsufficientBalance` | The caller's token balance is too low to cover the requested operation. |
| 108 | `OracleFailure` | The oracle failed to provide a result or returned an unreadable response. |
| 109 | `CircuitBreakerOpen` | The circuit breaker is open due to repeated failures; operations are temporarily halted. |
| 110 | `DisputeWindowClosed` | The dispute window for this market has already closed; disputes can no longer be filed. |
| 111 | `VotingNotStarted` | Voting on this market has not started yet. |
| 112 | `VotingEnded` | The voting period for this market has ended. |
| 113 | `AlreadyVoted` | The caller has already cast a vote on this market and cannot vote again. |
| 114 | `FeeTooHigh` | The requested fee exceeds the maximum allowed fee threshold. |
| 115 | `MarketNotActive` | The market is not in an active state; it may be pending, closed, or resolved. |
| 116 | `DeadlinePassed` | The submission deadline for this market has passed. |
| 117 | `CannotChangeOutcome` | The outcome for this market has already been set and cannot be changed. |
| 118 | `MarketNotDisputed` | The market is not in a disputed state; dispute-specific operations are unavailable. |
| 119 | `MarketNotPendingResolution` | The market is not pending resolution; resolution cannot proceed at this time. |
| 120 | `AdminNotSet` | No admin address has been configured for this contract. |
| 121 | `ContractPaused` | The contract is paused; all state-changing operations are disabled. |
| 122 | `GuardianNotSet` | No guardian address has been configured for this contract. |
| 123 | `TooManyOutcomes` | The number of outcomes provided exceeds the maximum allowed per market. |
| 124 | `TooManyWinners` | The number of winning outcomes exceeds the maximum allowed for payout calculation. |
| 125 | `PayoutModeNotSupported` | The requested payout mode is not supported by this contract version. |
| 126 | `InsufficientDeposit` | The deposit provided is below the minimum required amount. |
| 127 | `TimelockActive` | A timelock is currently active; the operation must wait until the timelock expires. |
| 128 | `UpgradeNotInitiated` | No upgrade has been initiated; upgrade-related operations cannot proceed. |
| 129 | `InsufficientVotes` | There are not enough governance votes to approve the requested action. |
| 130 | `AlreadyVotedOnUpgrade` | The caller has already voted on this upgrade proposal. |
| 131 | `InvalidWasmHash` | The provided WASM hash is malformed or does not match the expected format. |
| 132 | `UpgradeFailed` | The contract upgrade process failed; the new WASM could not be applied. |
| 133 | `ParentMarketNotResolved` | The parent market has not been resolved yet; this conditional market cannot proceed. |
| 134 | `ParentMarketInvalidOutcome` | The parent market resolved to an outcome that does not satisfy this market's condition. |
| 135 | `ResolutionNotReady` | The resolution conditions have not been met yet; try again later. |
| 136 | `DisputeWindowStillOpen` | The dispute window is still open; resolution must wait until it closes. |
| 137 | `NoMajorityReached` | No majority outcome was reached among the votes cast; resolution is inconclusive. |
| 138 | `StalePrice` | The oracle price data is too old and considered stale; a fresh price feed is required. |
| 139 | `ConfidenceTooLow` | The oracle's confidence score is below the minimum threshold required for resolution. |
| 140 | `InsufficientVotingWeight` | The caller's governance token balance is too low to meet the minimum voting weight. |
| 141 | `MarketNotCancelled` | The market was not cancelled; refund or cancellation-specific operations are unavailable. |
| 142 | `BetNotFound` | No bet was found for the given bet ID or caller address. |
| 143 | `UpgradeAlreadyPending` | An upgrade is already pending approval; only one upgrade can be in flight at a time. |
| 144 | `UpgradeHashInCooldown` | This WASM hash was recently used and is still within its cooldown period. |
| 145 | `InvalidAmount` | The provided amount is invalid (e.g. zero, negative, or exceeds allowed limits). |
| 146 | `GovernanceTokenNotSet` | No governance token contract address has been configured. |
| 147 | `MarketNotResolved` | The market has not been resolved yet; payout or post-resolution operations are unavailable. |
| 148 | `InvalidDeadline` | The provided deadline is in the past or otherwise invalid. |

## Error Groups

### Authorization & Setup
100 `AlreadyInitialized`, 101 `NotAuthorized`, 120 `AdminNotSet`, 121 `ContractPaused`, 122 `GuardianNotSet`, 146 `GovernanceTokenNotSet`

### Market Lifecycle
102 `MarketNotFound`, 103 `MarketClosed`, 104 `MarketStillActive`, 115 `MarketNotActive`, 116 `DeadlinePassed`, 148 `InvalidDeadline`

### Betting
105 `InvalidOutcome`, 106 `InvalidBetAmount`, 107 `InsufficientBalance`, 126 `InsufficientDeposit`, 142 `BetNotFound`, 145 `InvalidAmount`

### Resolution & Disputes
108 `OracleFailure`, 110 `DisputeWindowClosed`, 117 `CannotChangeOutcome`, 118 `MarketNotDisputed`, 119 `MarketNotPendingResolution`, 133 `ParentMarketNotResolved`, 134 `ParentMarketInvalidOutcome`, 135 `ResolutionNotReady`, 136 `DisputeWindowStillOpen`, 137 `NoMajorityReached`, 138 `StalePrice`, 139 `ConfidenceTooLow`, 141 `MarketNotCancelled`, 147 `MarketNotResolved`

### Voting & Governance
111 `VotingNotStarted`, 112 `VotingEnded`, 113 `AlreadyVoted`, 114 `FeeTooHigh`, 129 `InsufficientVotes`, 130 `AlreadyVotedOnUpgrade`, 140 `InsufficientVotingWeight`

### Upgrades
127 `TimelockActive`, 128 `UpgradeNotInitiated`, 131 `InvalidWasmHash`, 132 `UpgradeFailed`, 143 `UpgradeAlreadyPending`, 144 `UpgradeHashInCooldown`

### System
109 `CircuitBreakerOpen`, 123 `TooManyOutcomes`, 124 `TooManyWinners`, 125 `PayoutModeNotSupported`
