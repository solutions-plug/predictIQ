# PredictIQ Contract API Specification

> Reflects the on-chain implementation as of the current `contracts/predict-iq` source.

---

## Table of Contents

1. [Initialization](#initialization)
2. [Market Lifecycle](#market-lifecycle)
3. [Betting](#betting)
4. [Oracle & Resolution](#oracle--resolution)
5. [Disputes & Voting](#disputes--voting)
6. [Governance & Upgrades](#governance--upgrades)
7. [Fees & Referrals](#fees--referrals)
8. [Circuit Breaker](#circuit-breaker)
9. [Queries (Paginated)](#queries-paginated)
10. [Error Codes](#error-codes)
11. [Events](#events)

---

## Initialization

### `initialize(admin: Address, base_fee: i128) → Result<(), ErrorCode>`

Bootstraps the contract. Can only be called once.

| Param | Type | Description |
|-------|------|-------------|
| `admin` | `Address` | Master admin account (must authorize) |
| `base_fee` | `i128` | Protocol fee in stroops |

**Errors:** `AlreadyInitialized`

---

## Market Lifecycle

### `create_market(creator, description, options, deadline, resolution_deadline, oracle_config, tier, native_token, parent_id, parent_outcome_idx) → Result<u64, ErrorCode>`

| Param | Type | Description |
|-------|------|-------------|
| `creator` | `Address` | Market creator (must authorize) |
| `description` | `String` | Human-readable market question |
| `options` | `Vec<String>` | Outcome labels (max `MAX_OUTCOMES_PER_MARKET = 32`) |
| `deadline` | `u64` | Unix timestamp — betting closes |
| `resolution_deadline` | `u64` | Unix timestamp — resolution must occur by |
| `oracle_config` | `OracleConfig` | Multi-oracle configuration (see below) |
| `tier` | `MarketTier` | `Basic` \| `Pro` \| `Institutional` |
| `native_token` | `Address` | SAC token used for bets |
| `parent_id` | `u64` | `0` for independent markets; parent market ID for conditional |
| `parent_outcome_idx` | `u32` | Required parent outcome (ignored when `parent_id = 0`) |

**Returns:** new `market_id`

**OracleConfig fields:**

| Field | Type | Description |
|-------|------|-------------|
| `oracle_address` | `Address` | Deployed Pyth contract address |
| `feed_id` | `String` | 64-char hex-encoded 32-byte Pyth price feed ID |
| `min_responses` | `Option<u32>` | Minimum oracle responses required; `None` defaults to 1 |
| `max_staleness_seconds` | `u64` | Max age of price data in seconds |
| `max_confidence_bps` | `u64` | Max confidence interval in basis points |

**Errors:** `InvalidDeadline`, `TooManyOutcomes`, `InsufficientDeposit`, `MarketIdOverflow`, `MarketIdCollision`, `ParentMarketNotResolved`, `ParentMarketInvalidOutcome`

---

### `get_market(id: u64) → Option<Market>`

Returns the full `Market` struct or `None` if not found.

---

### `cancel_market_admin(market_id: u64) → Result<(), ErrorCode>`

Admin-only hard cancellation. Emits `mkt_cncl`.

**Errors:** `NotAuthorized`, `MarketNotFound`

---

### `prune_market(market_id: u64) → Result<(), ErrorCode>`

Permissionless cleanup after the 30-day grace period post-resolution.

**Errors:** `MarketNotFound`, `MarketStillActive`, `MarketNotResolved`

---

### `set_creator_reputation(creator: Address, reputation: CreatorReputation) → Result<(), ErrorCode>`

Admin-only. Sets `None | Basic | Pro | Institutional`.

---

### `set_creation_deposit(amount: i128) → Result<(), ErrorCode>` / `get_creation_deposit() → i128`

Admin-only deposit required to create a market.

---

### `claim_creation_deposit(market_id: u64, caller: Address) → Result<(), ErrorCode>`

Creator reclaims deposit after the dispute window closes without a challenge.

**Errors:** `MarketNotFound`, `NotAuthorized`, `DisputeWindowStillOpen`, `MarketNotDisputed`

---

## Betting

### `place_bet(bettor, market_id, outcome, amount, token_address, referrer) → Result<(), ErrorCode>`

| Param | Type | Description |
|-------|------|-------------|
| `bettor` | `Address` | Must authorize |
| `market_id` | `u64` | Target market |
| `outcome` | `u32` | Zero-based outcome index |
| `amount` | `i128` | Gross bet amount in token units |
| `token_address` | `Address` | Must match market's `token_address` |
| `referrer` | `Option<Address>` | Optional referral address |

**Errors:** `MarketNotFound`, `MarketClosed`, `MarketNotActive`, `InvalidBetAmount`, `InvalidOutcome`, `ContractPaused`, `InvalidReferrer`, `AssetClawedBack`, `TransferFailed`

---

### `claim_winnings(bettor: Address, market_id: u64) → Result<i128, ErrorCode>`

Pull-model payout. Returns amount transferred.

**Errors:** `MarketNotFound`, `MarketNotResolved`, `BetNotFound`, `NoWinnings`, `AlreadyClaimed`

---

### `withdraw_refund(bettor: Address, market_id: u64) → Result<i128, ErrorCode>`

Refund on cancelled markets.

**Errors:** `MarketNotFound`, `BetNotFound`, `AlreadyClaimed`

---

### `get_outcome_stake(market_id: u64, outcome: u32) → i128`

Total staked on a specific outcome.

---

### `count_bets_for_outcome(market_id: u64, outcome: u32) → u32`

Unique bettor count per outcome (analytics).

---

### `get_minimum_bet_amount() → i128` / `set_minimum_bet_amount(amount: i128) → Result<(), ErrorCode>`

---

## Oracle & Resolution

### `set_oracle_result(market_id: u64, oracle_id: u32, outcome: u32) → Result<(), ErrorCode>`

Admin-only. `oracle_id = 0` is the primary oracle. Supports multiple oracle sources per market.

**Errors:** `NotAuthorized`, `MarketNotFound`

---

### `get_oracle_result(market_id: u64, oracle_id: u32) → Option<u32>`

### `get_oracle_last_update(market_id: u64, oracle_id: u32) → Option<u64>`

---

### `attempt_oracle_resolution(market_id: u64) → Result<(), ErrorCode>`

Permissionless. Reads the oracle result and transitions the market to `PendingResolution` if conditions are met.

**Errors:** `MarketNotFound`, `MarketNotActive`, `OracleFailure`, `StalePrice`, `ConfidenceTooLow`, `ResolutionNotReady`

---

### `finalize_resolution(market_id: u64) → Result<(), ErrorCode>`

Permissionless. Moves `PendingResolution → Resolved` after the grace period.

**Errors:** `MarketNotFound`, `MarketNotPendingResolution`, `GracePeriodActive`, `ResolutionDeadlinePassed`

---

### `resolve_market(market_id: u64, winning_outcome: u32) → Result<(), ErrorCode>`

Admin-only resolution for disputed markets.

**Errors:** `NotAuthorized`, `MarketNotFound`, `MarketNotDisputed`

---

### `admin_fallback_resolution(market_id: u64, winning_outcome: u32) → Result<(), ErrorCode>`

Admin fallback when community voting deadlocks (no 60% majority after 72-hour window).

**Errors:** `NotAuthorized`, `MarketNotFound`, `MarketNotDisputed`, `VotingPeriodNotElapsed`, `NoMajorityReached`

---

### `set_dispute_window(seconds: u64) → Result<(), ErrorCode>` / `get_dispute_window() → u64`

Admin-only. Minimum 24 hours. Default 72 hours.

---

## Disputes & Voting

### `file_dispute(disciplinarian: Address, market_id: u64) → Result<(), ErrorCode>`

Opens a dispute window. Requires contract to be unpaused.

**Errors:** `MarketNotFound`, `MarketNotPendingResolution`, `DisputeWindowClosed`, `ContractPaused`

---

### `cast_vote(voter, market_id, outcome, weight) → Result<(), ErrorCode>`

Governance token holders vote on disputed outcome. Requires contract to be unpaused.

**Errors:** `MarketNotFound`, `MarketNotDisputed`, `AlreadyVoted`, `InsufficientVotingWeight`, `GovernanceTokenNotSet`, `ContractPaused`

---

### `unlock_tokens(voter: Address, market_id: u64) → Result<(), ErrorCode>`

Releases locked governance tokens after voting concludes.

---

### `get_resolution_metrics(market_id: u64, outcome: u32) → ResolutionMetrics`

### `set_max_push_payout_winners(threshold: u32)` / `get_max_push_payout_winners() → u32`

---

## Governance & Upgrades

### `add_guardian(guardian: Guardian) → Result<(), ErrorCode>`

### `remove_guardian(address: Address) → Result<(), ErrorCode>`

### `vote_on_guardian_removal(voter: Address, approve: bool) → Result<(), ErrorCode>`

### `get_guardians() → Vec<Guardian>`

### `emergency_pause(voter: Address) → Result<(), ErrorCode>`

Triggered by 2/3 Guardian majority.

---

### `initiate_upgrade(wasm_hash: BytesN<32>) → Result<(), ErrorCode>`

### `vote_for_upgrade(voter: Address, vote_for: bool) → Result<bool, ErrorCode>`

### `execute_upgrade() → Result<(), ErrorCode>`

### `get_pending_upgrade() → Option<PendingUpgrade>`

### `get_upgrade_votes() → Result<UpgradeStats, ErrorCode>`

Returns `{ votes_for: u32, votes_against: u32 }`.

### `is_timelock_satisfied() → Result<bool, ErrorCode>`

### `set_timelock_duration(seconds: u64) → Result<(), ErrorCode>` / `get_timelock_duration() → u64`

Range: 6 hours – 7 days. Default: 48 hours.

**Errors:** `TimelockActive`, `UpgradeNotInitiated`, `AlreadyVotedOnUpgrade`, `UpgradeAlreadyPending`, `UpgradeHashInCooldown`

---

### `set_guardian(guardian: Address) → Result<(), ErrorCode>` / `get_guardian() → Option<Address>`

Legacy single-guardian slot.

---

### `set_governance_token(token: Address) → Result<(), ErrorCode>`

---

## Fees & Referrals

### `set_base_fee(amount: i128) → Result<(), ErrorCode>` / `get_base_fee() → i128`

### `set_fee_admin(fee_admin: Address) → Result<(), ErrorCode>` / `get_fee_admin() → Option<Address>`

### `get_revenue(token: Address) → i128`

### `withdraw_protocol_fees(token: Address, recipient: Address) → Result<i128, ErrorCode>`

### `claim_referral_rewards(address: Address, token: Address) → Result<i128, ErrorCode>`

---

## Circuit Breaker

### `set_circuit_breaker(state: CircuitBreakerState) → Result<(), ErrorCode>`

States: `Closed | Open | HalfOpen | Paused`

### `pause() → Result<(), ErrorCode>` / `unpause() → Result<(), ErrorCode>`

### `reset_monitoring() → Result<(), ErrorCode>`

Admin-only. Clears error counters.

---

## Queries (Paginated)

All paginated queries silently clamp `limit` to **100** (`MAX_PAGE_LIMIT`). Callers requesting more receive at most 100 records — no error is returned.

### `get_markets(offset: u32, limit: u32) → Vec<Market>`

Returns all markets regardless of status, ordered by creation (ascending).

### `get_markets_by_status(status: MarketStatus, offset: u32, limit: u32) → Vec<Market>`

Filters by `Active | PendingResolution | Disputed | Resolved | Cancelled`. Iterates newest-first for fresher results.

### `get_guardians_paginated(offset: u32, limit: u32) → Vec<Guardian>`

### `get_admin() → Option<Address>`

---

## Error Codes

| Code | Value | Description |
|------|-------|-------------|
| `AlreadyInitialized` | 1 | Contract already initialized |
| `NotAuthorized` | 2 | Caller lacks required authorization |
| `GuardianNotSet` | 3 | Guardian account not configured |
| `MarketNotFound` | 4 | No market with the given ID |
| `MarketClosed` | 5 | Market deadline has passed |
| `MarketStillActive` | 6 | Market is still accepting bets |
| `MarketNotActive` | 7 | Market is not in Active state |
| `MarketNotResolved` | 8 | Market has not been resolved yet |
| `MarketNotDisputed` | 9 | Market is not in Disputed state |
| `MarketNotPendingResolution` | 10 | Market is not in PendingResolution state |
| `CannotChangeOutcome` | 11 | Outcome is already finalized |
| `InvalidDeadline` | 12 | Deadline is in the past or malformed |
| `DeadlinePassed` | 13 | Action attempted after deadline |
| `ResolutionDeadlinePassed` | 14 | Resolution deadline has elapsed |
| `ResolutionNotReady` | 15 | Conditions for resolution not yet met |
| `GracePeriodActive` | 16 | Grace period has not elapsed |
| `MarketIdOverflow` | 17 | Market ID counter overflowed |
| `MarketIdCollision` | 18 | Market ID already in use |
| `InvalidOutcome` | 19 | Outcome index out of range |
| `TooManyOutcomes` | 20 | Exceeds `MAX_OUTCOMES_PER_MARKET` (32) |
| `InvalidBetAmount` | 21 | Bet amount is zero or below minimum |
| `InsufficientBalance` | 22 | Caller token balance too low |
| `InsufficientDeposit` | 23 | Creation deposit not met |
| `InvalidAmount` | 24 | Generic invalid amount |
| `BetNotFound` | 25 | No bet record for this bettor/market |
| `NoWinnings` | 26 | Bettor did not back the winning outcome |
| `AlreadyClaimed` | 27 | Winnings or refund already claimed |
| `OracleFailure` | 28 | Oracle cross-contract call failed |
| `StalePrice` | 29 | Price feed `publish_time` older than `max_staleness_seconds` |
| `ConfidenceTooLow` | 30 | Oracle confidence interval exceeds `max_confidence_bps` |
| `InvalidTimestamp` | 31 | Timestamp value is invalid |
| `AssetClawedBack` | 32 | SAC token clawback reduced contract balance unexpectedly |
| `TransferFailed` | 33 | SAC token transfer failed programmatically |
| `DisputeWindowClosed` | 34 | Dispute window has expired |
| `DisputeWindowStillOpen` | 35 | Dispute window has not yet closed |
| `AlreadyVoted` | 36 | Address has already cast a vote |
| `InsufficientVotes` | 37 | Not enough votes to proceed |
| `InsufficientVotingWeight` | 38 | Voter's governance token balance too low |
| `NoMajorityReached` | 39 | No outcome reached the 60% majority threshold |
| `GovernanceTokenNotSet` | 40 | Governance token address not configured |
| `TimelockActive` | 41 | Upgrade timelock has not elapsed |
| `UpgradeNotInitiated` | 42 | No pending upgrade to act on |
| `AlreadyVotedOnUpgrade` | 43 | Address already voted on this upgrade |
| `UpgradeAlreadyPending` | 44 | An upgrade proposal is already pending |
| `UpgradeHashInCooldown` | 45 | This wasm hash is in the 7-day cooldown period |
| `ParentMarketNotResolved` | 46 | Conditional market's parent is not yet resolved |
| `ParentMarketInvalidOutcome` | 47 | Parent market resolved to a different outcome |
| `ContractPaused` | 48 | Contract is paused via circuit breaker |
| `InvalidReferrer` | 49 | Referrer address is invalid or self-referral |
| `VotingPeriodNotElapsed` | 50 | Admin fallback called before 72-hour voting window elapsed |

---

## Events

All events follow the topic layout:
- **Topic 0:** Event name (short symbol, ≤ 9 chars)
- **Topic 1:** `market_id: u64` (primary indexer key; `0` for contract-level events)
- **Topic 2:** Triggering address

| Event | Topic Symbol | Data Payload |
|-------|-------------|--------------|
| MarketCreated | `mkt_creat` | `(description: String, num_outcomes: u32, deadline: u64)` |
| BetPlaced | `bet_place` | `(outcome: u32, amount: i128)` |
| DisputeFiled | `disp_file` | `new_deadline: u64` |
| ResolutionFinalized | `resolv_fx` | `(winning_outcome: u32, total_payout: i128)` |
| RewardsClaimed | `reward_fx` | `(amount: i128, token_address: Address, is_refund: bool)` |
| VoteCast | `vote_cast` | `(outcome: u32, weight: i128)` |
| CircuitBreakerTriggered | `cb_state` | `state: String` |
| OracleResultSet | `oracle_ok` | `outcome: u32` |
| OracleResolved | `orcl_res` | `outcome: u32` |
| MarketFinalized | `mkt_final` | `winning_outcome: u32` |
| DisputeResolved | `disp_res` | `winning_outcome: u32` |
| MarketCancelled (admin) | `mkt_cncl` | `()` |
| MarketCancelledVote (community) | `mk_cn_vt` | `()` |
| ReferralReward | `ref_rwrd` | `amount: i128` |
| ReferralClaimed | `ref_claim` | `amount: i128` |
| CircuitBreakerAuto | `cb_auto` | `error_count: u32` |
| MonitoringStateReset | `mon_reset` | `(previous_error_count: u32, previous_last_observation: u64)` |
| FeeCollected | `fee_colct` | `amount: i128` |
| AdminFallbackResolution | `adm_fbk` | `winning_outcome: u32` |
| CreatorReputationSet | `rep_set` | `(old_score: u32, new_score: u32)` |
| CreationDepositSet | `dep_set` | `(old_amount: i128, new_amount: i128)` |

> **Note:** `MonitoringStateReset`, `CircuitBreakerTriggered`, `CircuitBreakerAuto`, and `FeeCollected` use `market_id = 0` and the contract address as Topic 2. `CreatorReputationSet` uses `(symbol, creator)` with no `market_id`. `CreationDepositSet` uses `(symbol,)` only.
