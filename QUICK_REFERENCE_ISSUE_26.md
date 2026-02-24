# Quick Reference: Virtual AMM (CPMM) Implementation

## Branch
```bash
features/issue-26-virtual-amm-cpmm-bet-liquidity
```

## What Was Implemented

### Core Formula
```
x * y = k

Where:
- x = USDC reserve in pool
- y = Virtual share reserve
- k = Constant product (invariant)
```

### Key Functions

#### Buy Shares
```rust
buy_shares(buyer, market_id, outcome, usdc_in, token) -> (shares_out, price)
```
- Swaps USDC for outcome shares
- Price increases with demand
- 0.3% fee applied

#### Sell Shares
```rust
sell_shares(seller, market_id, outcome, shares_in, token) -> (usdc_out, price)
```
- Swaps shares back to USDC
- Can exit anytime before deadline
- 0.3% fee applied

#### Query Functions
```rust
get_buy_price(market_id, outcome) -> price
get_user_shares(market_id, user, outcome) -> shares
get_amm_pool(market_id, outcome) -> AMMPool
quote_buy(market_id, outcome, usdc_in) -> shares_out
quote_sell(market_id, outcome, shares_in) -> usdc_out
```

## Test Results

```bash
cd contracts/predict-iq
cargo test --lib test_amm
```

**Result:** ✅ All 7 tests passing

1. ✅ Price increases with demand (x*y=k verified)
2. ✅ Buy/sell roundtrip with slippage
3. ✅ Pool invariant maintained
4. ✅ USDC backing matches shares
5. ✅ Quote accuracy
6. ✅ Error handling
7. ✅ Independent outcome pools

## Files Changed

### New Files
- `contracts/predict-iq/src/modules/amm.rs` - Core AMM logic (220 lines)
- `contracts/predict-iq/src/test_amm.rs` - Test suite (280 lines)
- `AMM_IMPLEMENTATION.md` - Full documentation
- `PR_SUMMARY_ISSUE_26.md` - PR description

### Modified Files
- `contracts/predict-iq/src/lib.rs` - Added 9 public functions
- `contracts/predict-iq/src/modules/mod.rs` - Added amm module

## Build Status

```bash
cd contracts/predict-iq
cargo build --target wasm32-unknown-unknown --release
```

**Result:** ✅ Build successful

## Verification Checklist

All requirements from issue #26 completed:

- ✅ **x*y=k formula**: Buying YES shares increases price correctly
- ✅ **Roundtrip**: User buys 100, sells 100, gets correct USDC back minus slippage
- ✅ **USDC backing**: Total USDC matches virtual backing of circulating shares
- ✅ **Branch created**: `features/issue-26-virtual-amm-cpmm-bet-liquidity`
- ✅ **PR ready**: Against develop branch

## Next Steps

1. **Push to GitHub:**
   ```bash
   git push origin features/issue-26-virtual-amm-cpmm-bet-liquidity
   ```

2. **Create PR:**
   - Base: `develop`
   - Title: "feat: Implement Virtual AMM (CPMM) for instant bet liquidity (#26)"
   - Description: Use content from `PR_SUMMARY_ISSUE_26.md`

3. **Review Points:**
   - CPMM formula correctness
   - Fee calculation
   - Invariant verification
   - Test coverage
   - Security considerations

## Key Metrics

- **Lines of Code:** ~500 (AMM module + tests)
- **Test Coverage:** 7 comprehensive tests
- **Fee Rate:** 0.3% (30 basis points)
- **Initial Liquidity:** Configurable per market
- **Build Time:** ~2 seconds
- **Test Time:** ~0.4 seconds

## Usage Example

```rust
// 1. Initialize pools (admin)
client.initialize_amm_pools(&1, &2, &10_000_0000000);

// 2. Buy shares (user)
let (shares, price) = client.buy_shares(
    &user, &1, &0, &100_0000000, &token
);
// Result: ~99,000 shares at ~0.0001 USDC/share

// 3. Sell shares (user)
let (usdc, price) = client.sell_shares(
    &user, &1, &0, &shares, &token
);
// Result: ~99.4 USDC (0.6% loss from fees + slippage)
```

## Documentation

- **Full Guide:** `AMM_IMPLEMENTATION.md`
- **PR Summary:** `PR_SUMMARY_ISSUE_26.md`
- **Code Comments:** Inline in `modules/amm.rs`
- **Test Documentation:** In `test_amm.rs`

---

**Status:** ✅ Ready for PR  
**Date:** 2026-02-23  
**Issue:** #26
