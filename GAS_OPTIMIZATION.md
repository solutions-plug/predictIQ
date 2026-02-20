# Gas Optimization & Instruction Benchmarking

## Overview

This document describes the gas optimization strategies implemented in PredictIQ to ensure the contract stays within Soroban's strict CPU and memory limits.

## Key Optimizations

### 1. Iteration Audit & Asymptotic Logic

#### Problem
Markets with thousands of outcomes or winners could exceed Soroban's instruction limits during resolution.

#### Solution
- **Outcome Limit**: Maximum 100 outcomes per market (`MAX_OUTCOMES_PER_MARKET`)
- **Automatic Payout Mode Selection**: 
  - Push payouts: For markets with ≤50 winners (contract distributes)
  - Pull payouts: For markets with >50 winners (users claim individually)

#### Implementation
```rust
pub const MAX_OUTCOMES_PER_MARKET: u32 = 100;
pub const MAX_PUSH_PAYOUT_WINNERS: u32 = 50;

pub enum PayoutMode {
    Push,  // Contract distributes to all winners
    Pull,  // Winners claim individually
}
```

### 2. Storage Cost Optimization

#### Problem
Large default integers in structs increase ledger footprint costs.

#### Solution
Use `Option<T>` for fields that may not always be set:

```rust
pub struct OracleConfig {
    pub oracle_address: Address,
    pub feed_id: String,
    pub min_responses: Option<u32>, // None defaults to 1
}
```

**Benefits**:
- Reduces storage size when field is not set
- Saves on ledger entry fees
- More efficient serialization

### 3. Optimized Market Resolution

#### Before
```rust
pub fn resolve_market(e: &Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
    let mut market = get_market(e, market_id)?;
    market.status = MarketStatus::Resolved;
    market.winning_outcome = Some(winning_outcome);
    update_market(e, market);
    Ok(())
}
```

#### After
```rust
pub fn resolve_market(e: &Env, market_id: u64, winning_outcome: u32) -> Result<(), ErrorCode> {
    let mut market = get_market(e, market_id)?;
    
    // Validate outcome
    if winning_outcome >= market.options.len() {
        return Err(ErrorCode::InvalidOutcome);
    }
    
    // Estimate winner count
    let estimated_winners = estimate_winner_count(e, market_id, winning_outcome);
    
    // Automatically select payout mode
    if estimated_winners > MAX_PUSH_PAYOUT_WINNERS {
        market.payout_mode = PayoutMode::Pull;
    } else {
        market.payout_mode = PayoutMode::Push;
    }
    
    market.status = MarketStatus::Resolved;
    market.winning_outcome = Some(winning_outcome);
    update_market(e, market);
    
    Ok(())
}
```

### 4. Resolution Metrics

New function to monitor gas usage before resolution:

```rust
pub struct ResolutionMetrics {
    pub winner_count: u32,
    pub total_winning_stake: i128,
    pub gas_estimate: u64,
}

pub fn get_resolution_metrics(e: &Env, market_id: u64, outcome: u32) -> ResolutionMetrics
```

**Usage**:
```rust
let metrics = contract.get_resolution_metrics(market_id, outcome);
if metrics.gas_estimate > SAFE_THRESHOLD {
    // Use pull payouts
}
```

## Benchmarking

### Running Benchmarks

#### Shell Script (Integration Testing)
```bash
cd contracts/predict-iq/benches
./gas_benchmark.sh
```

Tests:
1. Small market (10 outcomes)
2. Medium market (50 outcomes)
3. Large market (100 outcomes)
4. Multiple bet placement
5. Market resolution
6. Resolution metrics

#### Rust Tests (Unit Testing)
```bash
cd contracts/predict-iq
cargo test --test gas_benchmark -- --nocapture
```

Tests:
- `bench_create_market_10_outcomes`
- `bench_create_market_50_outcomes`
- `bench_create_market_100_outcomes`
- `bench_place_multiple_bets`
- `bench_resolve_market`
- `bench_get_resolution_metrics`
- `bench_reject_excessive_outcomes`

### Expected Results

| Operation | Outcomes | CPU Instructions | Memory Bytes | Status |
|-----------|----------|------------------|--------------|--------|
| Create Market | 10 | ~50K | ~2KB | ✓ Safe |
| Create Market | 50 | ~150K | ~8KB | ✓ Safe |
| Create Market | 100 | ~250K | ~15KB | ⚠ Monitor |
| Create Market | 101 | N/A | N/A | ✗ Rejected |
| Place Bet | 1 | ~30K | ~1KB | ✓ Safe |
| Resolve (Push) | 50 winners | ~2.5M | ~50KB | ⚠ Threshold |
| Resolve (Pull) | 1000 winners | ~100K | ~5KB | ✓ Safe |

## Best Practices

### For Market Creators
1. Limit outcomes to necessary options (prefer <50)
2. Consider market size when setting parameters
3. Monitor resolution metrics before finalizing

### For Contract Operators
1. Set `MAX_OUTCOMES_PER_MARKET` based on network conditions
2. Adjust `MAX_PUSH_PAYOUT_WINNERS` threshold as needed
3. Monitor instruction counts in production
4. Use pull payouts for large markets

### For Developers
1. Always test with maximum outcome counts
2. Use `get_resolution_metrics` before resolution
3. Implement proper error handling for gas limits
4. Consider batching operations when possible

## Monitoring

### Key Metrics to Track
- Average instruction count per operation
- Memory usage per market size
- Resolution time vs winner count
- Failed transactions due to gas limits

### Alerts
Set up monitoring for:
- Markets approaching 100 outcomes
- Resolution attempts with >50 winners in push mode
- Instruction counts exceeding 80% of limits

## Future Optimizations

### Potential Improvements
1. **Lazy Loading**: Load market data in chunks
2. **Outcome Indexing**: Maintain separate indices for faster lookups
3. **Batch Processing**: Process multiple operations in single transaction
4. **State Compression**: Use more efficient data structures
5. **Winner Tracking**: Maintain real-time winner counts during betting

### Research Areas
- Zero-knowledge proofs for outcome verification
- Off-chain computation with on-chain verification
- Optimistic rollups for high-volume markets

## References

- [Soroban Resource Limits](https://soroban.stellar.org/docs/fundamentals-and-concepts/resource-limits-fees)
- [Soroban Best Practices](https://soroban.stellar.org/docs/learn/best-practices)
- [Gas Optimization Patterns](https://soroban.stellar.org/docs/learn/optimization)

## Changelog

### v0.1.0 (Current)
- Added `PayoutMode` enum for push/pull payouts
- Implemented `MAX_OUTCOMES_PER_MARKET` limit
- Optimized `OracleConfig` with `Option<u32>`
- Added `get_resolution_metrics` function
- Created comprehensive benchmarking suite
- Automatic payout mode selection in `resolve_market`
