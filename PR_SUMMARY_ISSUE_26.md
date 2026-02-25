# Pull Request: Virtual AMM (CPMM) Bet Liquidity

## Issue
Closes #26

## Summary
Implements a Constant Product Market Maker (CPMM) using the formula `x * y = k` to enable instant betting and exiting without waiting for market resolution. This transitions PredictIQ from a parimutuel staking model to an AMM-based liquidity system.

## Changes

### New Module: `modules/amm.rs`
- **AMMPool struct**: Tracks USDC reserves, share reserves, constant product (k), and circulating shares
- **initialize_pools()**: Creates independent AMM pools for each market outcome
- **buy_shares()**: Swaps USDC for outcome shares with dynamic pricing
- **sell_shares()**: Swaps shares back to USDC anytime before deadline
- **get_buy_price()**: Returns current marginal price (x/y)
- **quote_buy/sell()**: Preview swap outcomes without execution
- **verify_invariant()**: Auditing function to ensure x*y=k holds

### Contract Interface Updates (`lib.rs`)
Added 9 new public functions:
- `initialize_amm_pools()` - Admin function to set up pools
- `buy_shares()` - User function to buy outcome shares
- `sell_shares()` - User function to sell shares back
- `get_buy_price()` - Query current price
- `get_user_shares()` - Query user balance
- `get_amm_pool()` - Query pool state
- `quote_buy()` - Preview buy operation
- `quote_sell()` - Preview sell operation
- `verify_pool_invariant()` - Verify mathematical integrity

### Test Suite (`test_amm.rs`)
7 comprehensive tests covering all verification requirements:
1. ✅ **test_amm_buy_increases_price** - Verifies x*y=k formula
2. ✅ **test_amm_buy_sell_roundtrip_with_slippage** - Tests roundtrip with fees
3. ✅ **test_amm_pool_invariant_maintained** - Ensures invariant across operations
4. ✅ **test_amm_usdc_backing_matches_shares** - Validates accounting integrity
5. ✅ **test_amm_quote_accuracy** - Confirms preview matches execution
6. ✅ **test_amm_insufficient_shares_error** - Tests error handling
7. ✅ **test_amm_multiple_outcomes_independent** - Verifies pool independence

**All tests passing:** `test result: ok. 7 passed; 0 failed`

## Key Features

### 1. Dynamic Pricing
- Prices adjust automatically based on supply and demand
- Buying increases price, selling decreases price
- Formula: `price = usdc_reserve / share_reserve`

### 2. Instant Liquidity
- Users can enter/exit positions anytime before market deadline
- No need to wait for counterparties or market resolution
- Enables active trading and price discovery

### 3. Fee Structure
- 0.3% fee (30 basis points) on all swaps
- Protects against arbitrage
- Generates protocol revenue

### 4. Independent Pools
- Each outcome has its own AMM pool
- Pools don't affect each other
- Enables multi-outcome markets

### 5. Mathematical Integrity
- Constant product formula: `x * y = k`
- Invariant verification function for auditing
- Allows 0.01% tolerance for rounding errors

## Verification Checklist

All requirements from issue #26 completed:

- ✅ Buying "YES" shares correctly increases price according to x*y=k
- ✅ User A buys 100 shares, sells 100 shares, receives correct USDC minus slippage/fees
- ✅ Total USDC in pool matches virtual backing of all circulating shares
- ✅ Branch created: `features/issue-26-virtual-amm-cpmm-bet-liquidity`
- ✅ PR created against develop branch

## Usage Example

```rust
// Initialize pools (admin)
client.initialize_amm_pools(&market_id, &2, &10_000_0000000);

// Buy shares (user)
let (shares, price) = client.buy_shares(
    &user, &market_id, &0, &100_0000000, &usdc_token
);

// Check price
let current_price = client.get_buy_price(&market_id, &0);

// Sell shares (user)
let (usdc_out, exit_price) = client.sell_shares(
    &user, &market_id, &0, &shares, &usdc_token
);
```

## Security Considerations

1. **Invariant Protection**: `verify_invariant()` ensures x*y=k always holds
2. **Fee Protection**: 0.3% fee prevents zero-cost arbitrage
3. **Authorization**: All trades require user authentication
4. **Circuit Breaker**: Respects pause state for high-risk operations
5. **Overflow Protection**: Checked arithmetic throughout

## Gas Optimization

- Minimal storage operations per trade
- Single pool update per transaction
- No iteration over users or outcomes
- Efficient integer arithmetic (no floating point)

## Documentation

- **AMM_IMPLEMENTATION.md**: Comprehensive implementation guide
- Inline code documentation
- Test documentation with clear assertions

## Testing

```bash
cd contracts/predict-iq
cargo test --lib test_amm
```

Output:
```
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured
```

## Breaking Changes

None. This is an additive feature that doesn't modify existing functionality.

## Migration Path

1. Deploy updated contract
2. Initialize AMM pools for new markets
3. Existing parimutuel markets continue to work
4. Gradually transition to AMM-based markets

## Future Enhancements

- Dynamic fee adjustment based on volatility
- Liquidity mining rewards
- Multi-asset collateral support
- Concentrated liquidity ranges

## Checklist

- [x] Code compiles without errors
- [x] All tests pass
- [x] Documentation added
- [x] Verification checklist completed
- [x] Security considerations addressed
- [x] Gas optimization implemented
- [x] Branch created from develop
- [x] Commit messages follow convention

## Reviewer Notes

Please pay special attention to:
1. CPMM formula implementation in `buy_shares()` and `sell_shares()`
2. Fee calculation and application
3. Invariant verification logic
4. Test coverage of edge cases
5. Integer arithmetic overflow protection

---

**Ready for review and merge into develop branch.**
