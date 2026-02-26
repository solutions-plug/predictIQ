# Quick Start: Gas Optimization Features

## For Market Creators

### Creating Gas-Efficient Markets

```rust
// ✓ GOOD: Small market (10 outcomes)
let options = vec!["Yes", "No", "Maybe", ...]; // 10 options
let market_id = contract.create_market(...);

// ⚠ CAUTION: Large market (100 outcomes)
let options = vec!["Option1", "Option2", ...]; // 100 options
let market_id = contract.create_market(...);

// ✗ BAD: Too many outcomes (will fail)
let options = vec![...]; // 101+ options
// Error: TooManyOutcomes
```

### Checking Gas Before Resolution

```rust
// Get metrics before resolving
let metrics = contract.get_resolution_metrics(market_id, winning_outcome);

println!("Winner count: {}", metrics.winner_count);
println!("Total stake: {}", metrics.total_winning_stake);
println!("Gas estimate: {}", metrics.gas_estimate);

// Decide based on metrics
if metrics.gas_estimate > SAFE_THRESHOLD {
    // Will automatically use pull payouts
}
```

## For Contract Operators

### Running Benchmarks

#### Quick Test (Rust)
```bash
cd contracts/predict-iq
cargo test --test gas_benchmark -- --nocapture
```

#### Full Integration Test (Shell)
```bash
cd contracts/predict-iq/benches
chmod +x gas_benchmark.sh

# For testnet
export NETWORK=testnet
export ADMIN_SECRET=YOUR_SECRET_KEY
./gas_benchmark.sh

# For local standalone
export NETWORK=standalone
./gas_benchmark.sh
```

### Monitoring Production

```rust
// Before resolving any market
let metrics = contract.get_resolution_metrics(market_id, outcome);

// Alert if gas estimate is high
if metrics.gas_estimate > 1_000_000 {
    log::warn!("High gas estimate for market {}: {}", 
        market_id, metrics.gas_estimate);
}

// Resolve with automatic payout mode selection
contract.resolve_market(market_id, winning_outcome);
```

## For Developers

### Understanding Payout Modes

```rust
pub enum PayoutMode {
    Push,  // Contract distributes to all winners (≤50 winners)
    Pull,  // Winners claim individually (>50 winners)
}
```

**Push Mode (Automatic for ≤50 winners):**
- Contract distributes winnings in one transaction
- Lower gas for winners (they don't pay)
- Higher gas for resolver
- Best for small markets

**Pull Mode (Automatic for >50 winners):**
- Winners claim individually
- Lower gas for resolver
- Winners pay their own gas
- Best for large markets

### Optimization Constants

```rust
// In types.rs
pub const MAX_OUTCOMES_PER_MARKET: u32 = 100;
pub const MAX_PUSH_PAYOUT_WINNERS: u32 = 50;
```

**Adjusting Thresholds:**
```rust
// To be more conservative (use pull mode earlier)
pub const MAX_PUSH_PAYOUT_WINNERS: u32 = 30;

// To allow more outcomes
pub const MAX_OUTCOMES_PER_MARKET: u32 = 150;
```

### Storage Optimization

```rust
// Before (always uses storage)
pub struct OracleConfig {
    pub min_responses: u32,  // Always stored, even if default
}

// After (saves storage when None)
pub struct OracleConfig {
    pub min_responses: Option<u32>,  // None = default (1)
}

// Usage
let config = OracleConfig {
    oracle_address: addr,
    feed_id: "feed".into(),
    min_responses: None,  // Saves storage!
};
```

## Common Scenarios

### Scenario 1: Small Prediction Market (10 outcomes)
```rust
// Create market
let market_id = contract.create_market(
    creator,
    "Will it rain tomorrow?",
    vec!["Yes", "No"],
    deadline,
    resolution_deadline,
    oracle_config,
);

// Place bets (multiple users)
contract.place_bet(user1, market_id, 0, 1000, token);
contract.place_bet(user2, market_id, 1, 2000, token);

// Resolve (automatic push mode - only 2 potential winners)
contract.resolve_market(market_id, 0);

// Winner claims (if pull mode was used)
contract.claim_winnings(user1, market_id, token);
```

### Scenario 2: Large Sports Tournament (100 teams)
```rust
// Create market with 100 outcomes
let teams = vec!["Team1", "Team2", ..., "Team100"];
let market_id = contract.create_market(
    creator,
    "Who will win the tournament?",
    teams,
    deadline,
    resolution_deadline,
    oracle_config,
);

// Many users bet on different teams
// ...

// Check metrics before resolving
let metrics = contract.get_resolution_metrics(market_id, winning_team);
// metrics.winner_count might be 75 (many people bet on winner)
// metrics.gas_estimate will be high

// Resolve (automatic pull mode - >50 winners)
contract.resolve_market(market_id, winning_team);

// Each winner claims individually
contract.claim_winnings(winner1, market_id, token);
contract.claim_winnings(winner2, market_id, token);
// ...
```

### Scenario 3: Monitoring & Alerts
```rust
// In your monitoring service
fn check_market_before_resolution(market_id: u64, outcome: u32) {
    let metrics = contract.get_resolution_metrics(market_id, outcome);
    
    // Alert on high gas
    if metrics.gas_estimate > 2_000_000 {
        alert!("Market {} has high gas estimate: {}", 
            market_id, metrics.gas_estimate);
    }
    
    // Alert on many winners
    if metrics.winner_count > 100 {
        alert!("Market {} has {} winners (pull mode will be used)", 
            market_id, metrics.winner_count);
    }
    
    // Log metrics
    log::info!("Market {}: {} winners, {} stake, {} gas",
        market_id,
        metrics.winner_count,
        metrics.total_winning_stake,
        metrics.gas_estimate
    );
}
```

## Troubleshooting

### Error: TooManyOutcomes
```
Problem: Tried to create market with >100 outcomes
Solution: Reduce number of outcomes or increase MAX_OUTCOMES_PER_MARKET
```

### High Gas Estimates
```
Problem: get_resolution_metrics shows gas_estimate > 5M
Solution: Market will automatically use pull payouts
Action: Inform users they need to claim winnings manually
```

### Benchmark Failures
```
Problem: Benchmark tests fail or show high CPU usage
Solution: 
1. Check if running on resource-constrained system
2. Reduce MAX_OUTCOMES_PER_MARKET
3. Lower MAX_PUSH_PAYOUT_WINNERS threshold
```

## Best Practices

1. **Always check metrics before resolution**
   ```rust
   let metrics = contract.get_resolution_metrics(market_id, outcome);
   ```

2. **Limit outcomes to what's necessary**
   - Prefer <50 outcomes for better performance
   - Use categories/grouping when possible

3. **Monitor gas usage in production**
   - Set up alerts for high gas estimates
   - Track average gas per operation type

4. **Test with maximum sizes**
   ```bash
   cargo test bench_create_market_100_outcomes
   ```

5. **Document payout mode for users**
   - Inform users if they need to claim manually
   - Provide clear UI for claiming winnings

## Resources

- Full documentation: [GAS_OPTIMIZATION.md](./GAS_OPTIMIZATION.md)
- Benchmark guide: [contracts/predict-iq/benches/README.md](./contracts/predict-iq/benches/README.md)
- Soroban limits: https://soroban.stellar.org/docs/fundamentals-and-concepts/resource-limits-fees
