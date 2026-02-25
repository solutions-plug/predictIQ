# Classic Stellar Assets Integration (Issue #21)

## Overview
This implementation enables seamless use of existing Stellar Classic assets (like USDC, EURC, etc.) within the PredictIQ Soroban contract through the Stellar Asset Contract (SAC) interface.

## Technical Implementation

### 1. SAC Integration
- Uses `soroban_sdk::token::Client` to interact with Stellar Asset Contracts
- All token operations (transfer, balance checks) go through the SAC interface
- No direct Classic asset manipulation required

### 2. Trustline Handling
- **No manual trustline management needed** - SAC handles this automatically
- The contract can receive any valid SAC-wrapped Classic asset
- Developers must ensure `token_address` points to a valid SAC contract
- Markets can be created with any SAC token via the `token_address` parameter

### 3. Clawback & Freeze Protection

#### Clawback Detection
The contract implements automatic clawback detection:
- `check_clawback(market_id)` function monitors prize pool integrity
- Compares actual contract balance vs expected market stakes
- If clawback detected, market automatically transitions to `Cancelled`
- Emits `MarketCancelled` event with clawback flag

#### Freeze Handling
- Transfer operations will fail if contract account is frozen
- Markets with frozen assets can be cancelled by admin
- Users can withdraw refunds from cancelled markets

### 4. Key Functions

#### `check_clawback(market_id: u64)`
```rust
pub fn check_clawback(e: Env, market_id: u64) -> Result<(), ErrorCode>
```
- Verifies contract balance matches market's total staked amount
- Automatically cancels market if clawback detected
- Returns `ErrorCode::AssetClawedBack` if funds seized

#### `safe_transfer()`
```rust
pub fn safe_transfer(
    e: &Env,
    token_address: &Address,
    from: &Address,
    to: &Address,
    amount: &i128,
) -> Result<(), ErrorCode>
```
- Wraps token transfers with error handling
- Handles Classic asset edge cases (freeze, clawback)

## Integration Tests

### Test Coverage
1. **test_classic_asset_sac_integration**
   - Creates market with Classic USDC
   - Places bets using SAC interface
   - Verifies balance tracking
   - Resolves market and claims winnings

2. **test_clawback_detection_cancels_market**
   - Demonstrates `check_clawback()` functionality
   - Verifies market cancellation on clawback

3. **test_classic_asset_full_lifecycle**
   - Complete lifecycle: create → bet → resolve → claim
   - Verifies winner/loser logic with Classic assets
   - Tests balance updates throughout lifecycle

4. **test_frozen_asset_handling**
   - Tests graceful handling of frozen assets
   - Verifies normal operations when not frozen

## Usage Example

```rust
// Create market with Classic USDC
let usdc_sac_address = Address::from_string("CBGTG...");
let market_id = client.create_market(
    &creator,
    &description,
    &options,
    &deadline,
    &resolution_deadline,
    &oracle_config,
    &MarketTier::Basic,
    &usdc_sac_address,  // SAC-wrapped USDC
    &0,
    &0,
);

// Place bet with Classic asset
client.place_bet(
    &bettor,
    &market_id,
    &outcome,
    &amount,
    &usdc_sac_address,
    &None
);

// Monitor for clawback
client.check_clawback(&market_id)?;

// Claim winnings in Classic asset
client.claim_winnings(&bettor, &market_id, &usdc_sac_address)?;
```

## Security Considerations

### Classic Asset Flags
Classic Stellar assets can have special authorization flags:
- **AUTH_REQUIRED**: Requires trustline authorization
- **AUTH_REVOCABLE**: Issuer can revoke trustlines
- **AUTH_CLAWBACK_ENABLED**: Issuer can reclaim tokens
- **AUTH_IMMUTABLE**: Flags cannot be changed

### Risk Mitigation
1. **Clawback Detection**: Automatic market cancellation protects users
2. **Event Emission**: All state changes emit events for transparency
3. **Refund Mechanism**: Users can withdraw from cancelled markets
4. **Balance Verification**: Regular checks ensure prize pool integrity

## Events

### MarketCancelled Event
```rust
// Topics: [mkt_cancl, market_id, contract_address]
// Data: (is_clawback: bool)
emit_market_cancelled(e, market_id, true);
```

## Error Codes
- `AssetClawedBack (135)`: Issuer clawed back contract funds
- `AssetFrozen (136)`: Contract account is frozen
- `InvalidBetAmount (106)`: Token address mismatch

## Verification Checklist ✅

- [x] SAC Integration using `soroban_sdk::token::Client`
- [x] Trustline handling (automatic via SAC)
- [x] Clawback detection and market cancellation
- [x] Freeze scenario handling
- [x] Integration test: Classic asset wrapper
- [x] Integration test: place_bet with Classic asset
- [x] Integration test: claim_winnings with Classic asset
- [x] Event emission for cancellations
- [x] Documentation and code comments

## Testing

Run Classic asset tests:
```bash
cargo test test_classic_asset --manifest-path contracts/predict-iq/Cargo.toml
```

## Future Enhancements
1. Periodic automated clawback checks
2. Asset whitelist/blacklist functionality
3. Multi-asset market support
4. Asset metadata validation
