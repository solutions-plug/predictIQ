# Gas Benchmarking Suite

This directory contains benchmarking tools to measure CPU instructions and memory usage for PredictIQ contract operations.

## Files

- `gas_benchmark.rs` - Rust unit tests for measuring instruction counts
- `gas_benchmark.sh` - Shell script for integration testing with deployed contracts
- `README.md` - This file

## Running Benchmarks

### Rust Unit Tests

Run all benchmarks with output:
```bash
cargo test --test gas_benchmark -- --nocapture
```

Run specific benchmark:
```bash
cargo test --test gas_benchmark bench_create_market_100_outcomes -- --nocapture
```

### Shell Script (Requires Deployed Contract)

Make executable:
```bash
chmod +x gas_benchmark.sh
```

Run with testnet:
```bash
export NETWORK=testnet
export ADMIN_SECRET=YOUR_SECRET_KEY
./gas_benchmark.sh
```

Run with local standalone:
```bash
export NETWORK=standalone
./gas_benchmark.sh
```

## Benchmark Tests

### 1. Market Creation
- **10 outcomes**: Baseline small market
- **50 outcomes**: Medium market at push/pull threshold
- **100 outcomes**: Maximum allowed outcomes
- **101 outcomes**: Should fail with TooManyOutcomes error

### 2. Bet Placement
- Multiple sequential bets to measure incremental cost
- Tests storage growth impact

### 3. Market Resolution
- Measures resolution cost with automatic payout mode selection
- Tests winner count estimation

### 4. Resolution Metrics
- Tests the gas estimation function
- Validates winner count and stake calculations

## Expected Performance

| Operation | Outcomes/Bets | Expected CPU | Expected Memory | Status |
|-----------|---------------|--------------|-----------------|--------|
| Create Market | 10 | ~50K | ~2KB | ✓ Safe |
| Create Market | 50 | ~150K | ~8KB | ✓ Safe |
| Create Market | 100 | ~250K | ~15KB | ⚠ Monitor |
| Place Bet | 1 | ~30K | ~1KB | ✓ Safe |
| Resolve Market | - | ~100K | ~5KB | ✓ Safe |
| Get Metrics | - | ~20K | ~1KB | ✓ Safe |

## Interpreting Results

### CPU Instructions
- **< 1M**: Safe for all operations
- **1M - 5M**: Monitor closely, may hit limits with complex operations
- **> 5M**: Likely to exceed Soroban limits

### Memory Bytes
- **< 10KB**: Safe
- **10KB - 50KB**: Monitor
- **> 50KB**: May exceed limits

## Optimization Recommendations

Based on benchmark results:

1. **If create_market_100_outcomes > 300K CPU**:
   - Reduce MAX_OUTCOMES_PER_MARKET
   - Consider outcome batching

2. **If place_bet shows linear growth**:
   - Implement bet aggregation
   - Use more efficient storage keys

3. **If resolve_market > 1M CPU**:
   - Force pull payouts for all markets
   - Reduce MAX_PUSH_PAYOUT_WINNERS

## Continuous Monitoring

Add these benchmarks to CI/CD:

```yaml
# .github/workflows/benchmark.yml
- name: Run Gas Benchmarks
  run: |
    cd contracts/predict-iq
    cargo test --test gas_benchmark -- --nocapture > benchmark_results.txt
    
- name: Check for Regressions
  run: |
    # Compare with baseline
    # Fail if CPU usage increased by >10%
```

## Troubleshooting

### "WASM file not found"
Build the contract first:
```bash
cargo build --target wasm32-unknown-unknown --release
```

### "Contract not deployed"
Set CONTRACT_ID environment variable:
```bash
export CONTRACT_ID=YOUR_CONTRACT_ID
```

### "Budget exceeded"
The operation is too expensive. Consider:
- Reducing market size
- Using pull payouts
- Optimizing data structures

## References

- [Soroban Resource Limits](https://soroban.stellar.org/docs/fundamentals-and-concepts/resource-limits-fees)
- [Soroban Testing Guide](https://soroban.stellar.org/docs/learn/testing)
- [Gas Optimization Guide](../../GAS_OPTIMIZATION.md)
