# Quick Start: Pyth Oracle Integration

## Creating a Market with Pyth Oracle

```rust
use soroban_sdk::{Address, String, Vec, Env};

let oracle_config = OracleConfig {
    oracle_address: pyth_contract_address,  // Pyth contract on Soroban
    feed_id: String::from_str(&e, "0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43"), // BTC/USD
    min_responses: 1,
    max_staleness_seconds: 300,  // 5 minutes
    max_confidence_bps: 200,     // 2%
};

let market_id = client.create_market(
    &creator,
    &String::from_str(&e, "Will BTC reach $100k by EOY?"),
    &options,
    &deadline,
    &resolution_deadline,
    &oracle_config
);
```

## Resolving a Market

```rust
// After market deadline passes
client.resolve_with_oracle(&market_id);

// Result:
// - Success: Market status → Resolved, winning_outcome set
// - Stale price: Market status → Disputed
// - Low confidence: Market status → Disputed
// - Oracle failure: Returns ErrorCode::OracleFailure
```

## Configuration Examples

### High-Frequency Trading Market
```rust
max_staleness_seconds: 60,   // 1 minute
max_confidence_bps: 500,     // 5% (volatile)
```

### Standard Prediction Market
```rust
max_staleness_seconds: 300,  // 5 minutes
max_confidence_bps: 200,     // 2%
```

### Long-Term Market
```rust
max_staleness_seconds: 3600, // 1 hour
max_confidence_bps: 100,     // 1% (stable)
```

## Error Handling

```rust
match client.try_resolve_with_oracle(&market_id) {
    Ok(_) => {
        // Market resolved successfully
        let market = client.get_market(&market_id).unwrap();
        assert_eq!(market.status, MarketStatus::Resolved);
    },
    Err(ErrorCode::StalePrice) => {
        // Price too old, market disputed
    },
    Err(ErrorCode::ConfidenceTooLow) => {
        // Price confidence too low, market disputed
    },
    Err(ErrorCode::OracleFailure) => {
        // Oracle communication failed
    },
    Err(e) => {
        // Other error
    }
}
```

## Pyth Price Feed IDs

Common feeds (Mainnet):
- BTC/USD: `0xe62df6c8b4a85fe1a67db44dc12de5db330f7ac66b72dc658afedf0f4a415b43`
- ETH/USD: `0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace`
- SOL/USD: `0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d`

Find more at: https://pyth.network/developers/price-feed-ids

## Testing

```bash
# Run all tests
cargo test

# Run oracle-specific tests
cargo test oracles_test

# Run with output
cargo test -- --nocapture
```

## Events

Listen for oracle resolution events:

```rust
// Event format
("oracle_resolution", market_id) → (outcome, price, confidence)

// Example
("oracle_resolution", 1) → (0, 98500, 1000)
// Market 1 resolved to outcome 0, price $98,500, confidence ±$1,000
```
