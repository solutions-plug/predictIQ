# Automated Hybrid Consensus Resolution - Implementation Summary

## Overview
Implemented a state machine that seamlessly transitions from Oracle Data to Community Voting for market resolution in the PredictIQ prediction market contract.

## State Machine Flow

### T+0: Oracle Resolution Attempt
- **Function**: `attempt_oracle_resolution(market_id)`
- **Trigger**: Called at or after `resolution_deadline`
- **Action**: Attempts to fetch oracle result
- **Success**: Market moves to `PendingResolution` status, 24-hour dispute window starts
- **Failure**: Returns `OracleFailure` error

### T+24h: Finalization or Dispute
- **Function**: `finalize_resolution(market_id)`
- **No Dispute Path**: 
  - If no `file_dispute` called within 24 hours
  - Anyone can trigger `finalize_resolution`
  - Market moves to `Resolved` with oracle outcome
- **Dispute Path**:
  - If `file_dispute` called within 24 hours
  - Market moves to `Disputed` status
  - 72-hour voting period begins

### T+96h: Voting Resolution
- **Function**: `finalize_resolution(market_id)` (same function, different logic)
- **Action**: Calculates voting outcome from community votes
- **Majority Check**: Requires >60% consensus
- **Success**: Market resolved with voting outcome
- **No Majority**: Returns `NoMajorityReached`, requires admin intervention

### Admin Override
- **Function**: `resolve_market(market_id, outcome)` (admin-only)
- **Purpose**: Manual resolution when no majority reached
- **Requirement**: Admin authorization

## Implementation Details

### New Module: `resolution.rs`
- `attempt_oracle_resolution()`: Initiates oracle-based resolution
- `finalize_resolution()`: Handles both no-dispute and post-voting finalization
- `calculate_voting_outcome()`: Computes majority with 60% threshold

### Updated Modules

#### `disputes.rs`
- Added 24-hour dispute window enforcement
- Tracks `dispute_timestamp` for voting period calculation
- Validates disputes only during window

#### `types.rs`
- Added `pending_resolution_timestamp: Option<u64>` to Market
- Added `dispute_timestamp: Option<u64>` to Market

#### `errors.rs`
- `DisputeWindowStillOpen` (126): Cannot finalize before 24h
- `ResolutionNotReady` (127): Market not ready for resolution
- `NoMajorityReached` (128): Voting didn't reach 60% threshold

### Constants
```rust
const DISPUTE_WINDOW_SECONDS: u64 = 86400;    // 24 hours
const VOTING_PERIOD_SECONDS: u64 = 259200;    // 72 hours
const MAJORITY_THRESHOLD_BPS: i128 = 6000;    // 60%
```

## Test Coverage

### Unit Tests (8 tests, all passing)

1. **test_stage1_oracle_resolution_success**
   - Verifies oracle resolution moves market to PendingResolution
   - Confirms timestamp tracking

2. **test_stage2_finalize_after_24h_no_dispute**
   - Tests successful finalization after dispute window
   - Verifies market moves to Resolved

3. **test_stage2_cannot_finalize_before_24h**
   - Ensures 24-hour window is enforced
   - Expects `DisputeWindowStillOpen` error

4. **test_stage3_dispute_filed_within_24h**
   - Verifies dispute filing within window
   - Confirms market moves to Disputed

5. **test_stage3_cannot_dispute_after_24h**
   - Ensures dispute window closes after 24h
   - Expects `DisputeWindowClosed` error

6. **test_stage4_voting_resolution_with_majority**
   - Tests voting with 70% majority
   - Verifies outcome reflects voting result

7. **test_stage4_no_majority_requires_admin**
   - Tests 55% vote (below 60% threshold)
   - Expects `NoMajorityReached` error

8. **test_payouts_blocked_until_resolved**
   - Verifies payouts fail during PendingResolution
   - Confirms payouts work after Resolved

## Verification Checklist

✅ Unit tests for all 4 stages of the state machine
✅ Verify that payouts are blocked until Finalized
✅ Feature branch created: `features/issue-4-automated-hybrid-consensus-resolution`
✅ All 16 tests passing (8 new + 8 existing)

## Public API

### New Functions
```rust
pub fn attempt_oracle_resolution(e: Env, market_id: u64) -> Result<(), ErrorCode>
pub fn finalize_resolution(e: Env, market_id: u64) -> Result<(), ErrorCode>
```

### Existing Functions (behavior unchanged)
```rust
pub fn file_dispute(e: Env, disciplinarian: Address, market_id: u64) -> Result<(), ErrorCode>
pub fn resolve_market(e: Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> // Admin only
```

## Usage Example

```rust
// T+0: Oracle resolution
client.attempt_oracle_resolution(&market_id)?;

// T+24h: No dispute - finalize
client.finalize_resolution(&market_id)?;

// OR: Dispute filed
client.file_dispute(&disputer, &market_id)?;

// Users vote during 72h period
client.cast_vote(&voter, &market_id, &outcome, &weight)?;

// T+96h: Finalize with voting outcome
client.finalize_resolution(&market_id)?;

// If no majority, admin resolves
client.resolve_market(&market_id, &outcome)?; // Admin only
```

## Security Considerations

1. **Time-based transitions**: All state transitions are time-locked and cannot be bypassed
2. **Payout protection**: Payouts blocked until market is fully Resolved
3. **Majority requirement**: 60% threshold prevents manipulation by small groups
4. **Admin fallback**: Manual resolution available when consensus fails
5. **Circuit breaker**: All resolution functions respect circuit breaker state

## Gas Optimization

- Single function (`finalize_resolution`) handles both paths
- Minimal storage updates (only status and outcome)
- Efficient vote tallying with early exit on majority

## Next Steps

1. Create PR against `develop` branch
2. Request code review
3. Consider adding:
   - Event emissions for better monitoring
   - Configurable thresholds (60%, 24h, 72h)
   - Automated keeper integration for finalization
