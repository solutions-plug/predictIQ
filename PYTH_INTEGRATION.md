# Pyth Network Oracle Integration

This document describes the implementation of Pyth Network oracle integration for PredictIQ.

## Overview

The Pyth Network integration replaces the dummy oracle with live on-chain price feeds, enabling automatic market resolution based on real-world price data.

## Features

### 1. Price Feed Mapping
- `OracleConfig.feed_id` maps to Pyth's Price ID (32-byte identifier)
- `OracleConfig.oracle_address` stores the Pyth contract address on Soroban

### 2. Freshness Validation
- Configurable `max_staleness_seconds` (default: 300 seconds / 5 minutes)
- Prices older than the threshold are rejected with `ErrorCode::StalePrice`
- Validation checks: `current_time - publish_time <= max_staleness_seconds`

### 3. Confidence Check
- Configurable `max_confidence_bps` in basis points (default: 200 = 2%)
- Formula: `price.conf <= (price.price * max_confidence_bps) / 10000`
- Low confidence prices trigger `ErrorCode::ConfidenceTooLow` and set `MarketStatus::Disputed`

### 4. Auto-Resolution
- Successfully validated prices automatically resolve markets
- Market status transitions: `PendingResolution` → `Resolved`
- Failed validations transition to `Disputed` status

## Configuration

### OracleConfig Structure
```rust
pub struct OracleConfig {
    pub oracle_address: Address,        // Pyth contract address
    pub feed_id: String,                // Pyth Price ID (hex string)
    pub min_responses: u32,             // Minimum oracle responses
    pub max_staleness_seconds: u64,     // Maximum price age (seconds)
    pub max_confidence_bps: u64,        // Max confidence interval (basis points)
}
```

### Example Configuration
```rust
let oracle_config = OracleConfig {
    oracle_address: pyth_contract_address,
    feed_id: String::from_str(&e, "btc_usd_price_feed_id"),
    min_responses: 1,
    max_staleness_seconds: 300,  // 5 minutes
    max_confidence_bps: 200,     // 2%
};
```

## API Functions

### `resolve_with_oracle(market_id: u64)`
Resolves a market using Pyth price data.

**Requirements:**
- Market must be in `PendingResolution` status
- Circuit breaker must be closed

**Behavior:**
- Fetches price from Pyth contract
- Validates freshness and confidence
- On success: Sets market to `Resolved` with winning outcome
- On validation failure: Sets market to `Disputed`

**Events:**
```rust
("oracle_resolution", market_id) → (outcome, price, confidence)
```

### `set_oracle_result(market_id: u64, outcome: u32)`
Manual override for testing (admin only).

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 121 | `StalePrice` | Price data is older than `max_staleness_seconds` |
| 122 | `ConfidenceTooLow` | Price confidence interval exceeds `max_confidence_bps` |
| 108 | `OracleFailure` | General oracle communication failure |

## Implementation Details

### Price Validation Flow
```
1. Fetch price from Pyth contract
2. Check freshness: age <= max_staleness_seconds
3. Check confidence: conf <= (price * max_confidence_bps) / 10000
4. If valid: Store result and resolve market
5. If invalid: Mark market as disputed
```

### PythPrice Structure
```rust
pub struct PythPrice {
    pub price: i64,          // Price value
    pub conf: u64,           // Confidence interval
    pub expo: i32,           // Price exponent (for decimals)
    pub publish_time: i64,   // Unix timestamp
}
```

## Testing

### Test Coverage
- ✅ Fresh price validation
- ✅ Stale price rejection
- ✅ Low confidence rejection
- ✅ Manual oracle result setting
- ✅ Market lifecycle with oracle config

### Running Tests
```bash
cargo test
```

### Mock Pyth Contract
For testing, implement a mock Pyth contract that returns:
- Valid prices (fresh, good confidence)
- Stale prices (old timestamp)
- Low confidence prices (high conf value)

## Integration Guide

### 1. Deploy Pyth Contract
Deploy or reference the Pyth Network contract on Soroban testnet/mainnet.

### 2. Get Price Feed ID
Obtain the 32-byte Price ID for your desired asset from [Pyth Network](https://pyth.network/developers/price-feed-ids).

### 3. Create Market with Oracle Config
```rust
let oracle_config = OracleConfig {
    oracle_address: pyth_contract_address,
    feed_id: price_feed_id,
    min_responses: 1,
    max_staleness_seconds: 300,
    max_confidence_bps: 200,
};

client.create_market(
    &creator,
    &description,
    &options,
    &deadline,
    &resolution_deadline,
    &oracle_config
);
```

### 4. Resolve Market
```rust
// After market deadline passes
client.resolve_with_oracle(&market_id);
```

## Production Considerations

### Freshness Configuration
- **High-frequency markets**: 60-120 seconds
- **Standard markets**: 300 seconds (5 minutes)
- **Low-frequency markets**: 600+ seconds

### Confidence Configuration
- **Volatile assets**: 300-500 bps (3-5%)
- **Stable assets**: 100-200 bps (1-2%)
- **Critical markets**: 50-100 bps (0.5-1%)

### Circuit Breaker Integration
The oracle resolution respects the circuit breaker state:
- `Closed`: Normal operation
- `Open`: All resolutions blocked
- `HalfOpen`: Limited resolution attempts

## Future Enhancements

1. **Multi-Oracle Support**: Aggregate prices from multiple Pyth feeds
2. **Custom Outcome Logic**: Configurable price-to-outcome mapping
3. **Historical Price Queries**: Support for time-weighted average prices
4. **Fallback Oracles**: Secondary oracle if Pyth is unavailable

## References

- [Pyth Network Documentation](https://docs.pyth.network/)
- [Pyth Soroban SDK](https://github.com/pyth-network/pyth-crosschain/tree/main/target_chains/soroban)
- [Soroban Documentation](https://soroban.stellar.org/docs)

