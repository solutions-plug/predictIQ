# Virtual AMM (CPMM) Bet Liquidity - Issue #26

## Overview

This implementation transitions PredictIQ from a parimutuel staking model to a **Constant Product Market Maker (CPMM)** using the formula `x * y = k`. This enables instant betting and exiting without waiting for market resolution.

## Key Features

### 1. Virtual Token Pools
- Each outcome in a market has its own independent AMM pool
- Pools maintain USDC reserves and virtual share reserves
- Constant product invariant: `usdc_reserve * share_reserve = k`

### 2. Dynamic Pricing
- Prices adjust automatically based on supply and demand
- Buying shares increases the price for subsequent buyers
- Selling shares decreases the price
- Marginal price = `usdc_reserve / share_reserve`

### 3. Instant Liquidity
- Users can enter positions by swapping USDC for outcome shares
- Users can exit positions anytime before market deadline
- No need to wait for other bettors or market resolution

### 4. Fee Structure
- 0.3% fee (30 basis points) on all swaps
- Fees applied on input for buys, output for sells
- Protects against arbitrage and generates protocol revenue

## Architecture

### Core Module: `modules/amm.rs`

```rust
pub struct AMMPool {
    pub usdc_reserve: i128,           // x: USDC in pool
    pub share_reserve: i128,          // y: Virtual shares in pool
    pub k: i128,                      // Constant product
    pub total_shares_issued: i128,    // Circulating shares
}
```

### Key Functions

#### `initialize_pools(market_id, num_outcomes, initial_usdc)`
- Creates AMM pools for each outcome
- Distributes initial USDC equally across outcomes
- Sets initial share reserve to 1M units

#### `buy_shares(market_id, buyer, outcome, usdc_in) -> (shares_out, price)`
- Swaps USDC for outcome shares
- Formula: `shares_out = share_reserve - (k / (usdc_reserve + usdc_after_fee))`
- Updates pool reserves and user balances
- Returns shares received and effective price

#### `sell_shares(market_id, seller, outcome, shares_in) -> (usdc_out, price)`
- Swaps outcome shares back for USDC
- Formula: `usdc_out = usdc_reserve - (k / (share_reserve + shares_in))`
- Applies fee on output
- Returns USDC received and effective price

#### `get_buy_price(market_id, outcome) -> price`
- Returns current marginal price for buying 1 share
- Price = `usdc_reserve / share_reserve`

#### `quote_buy/quote_sell`
- Preview swap outcomes without executing
- Useful for frontends to show expected results

#### `verify_invariant(market_id, outcome) -> bool`
- Auditing function to verify `x * y = k` holds
- Allows 0.01% tolerance for rounding errors

## Contract Interface

### Public Functions

```rust
// Initialize AMM pools for a market (admin only)
pub fn initialize_amm_pools(market_id: u64, num_outcomes: u32, initial_usdc: i128)

// Buy outcome shares with USDC
pub fn buy_shares(buyer: Address, market_id: u64, outcome: u32, usdc_in: i128, token: Address) 
    -> (shares_out: i128, price: i128)

// Sell outcome shares for USDC
pub fn sell_shares(seller: Address, market_id: u64, outcome: u32, shares_in: i128, token: Address)
    -> (usdc_out: i128, price: i128)

// Get current buy price for an outcome
pub fn get_buy_price(market_id: u64, outcome: u32) -> i128

// Get user's share balance
pub fn get_user_shares(market_id: u64, user: Address, outcome: u32) -> i128

// Get pool state
pub fn get_amm_pool(market_id: u64, outcome: u32) -> Option<AMMPool>

// Preview swap outcomes
pub fn quote_buy(market_id: u64, outcome: u32, usdc_in: i128) -> i128
pub fn quote_sell(market_id: u64, outcome: u32, shares_in: i128) -> i128

// Verify pool invariant (auditing)
pub fn verify_pool_invariant(market_id: u64, outcome: u32) -> bool
```

## Verification Checklist ✅

### ✅ Price Increases with Demand
**Test:** `test_amm_buy_increases_price`
- Verifies buying shares increases price for next buyer
- Confirms CPMM formula `x*y=k` is working correctly

### ✅ Buy/Sell Roundtrip with Slippage
**Test:** `test_amm_buy_sell_roundtrip_with_slippage`
- User buys 100 shares, then sells 100 shares
- Receives back correct USDC amount minus slippage and fees
- Validates slippage is within acceptable range (<5%)

### ✅ USDC Backing Matches Shares
**Test:** `test_amm_usdc_backing_matches_shares`
- Verifies total USDC in pool matches virtual backing
- Confirms `total_shares_issued` equals sum of user holdings
- Ensures no USDC leakage or accounting errors

### ✅ Pool Invariant Maintained
**Test:** `test_amm_pool_invariant_maintained`
- Verifies `x * y = k` holds after multiple trades
- Tests buy operations, sell operations, and mixed scenarios
- Confirms mathematical integrity of CPMM

### ✅ Additional Tests
- **Quote Accuracy:** Previews match actual execution
- **Insufficient Balance:** Proper error handling
- **Independent Outcomes:** Pools don't affect each other

## Usage Example

```rust
// 1. Admin initializes AMM pools for a market
client.initialize_amm_pools(&market_id, &2, &10_000_0000000); // 10k USDC

// 2. User buys YES shares
let (shares, price) = client.buy_shares(
    &user,
    &market_id,
    &0,  // outcome 0 = YES
    &100_0000000,  // 100 USDC
    &usdc_token
);

// 3. Check current price
let current_price = client.get_buy_price(&market_id, &0);

// 4. User sells shares back
let (usdc_out, exit_price) = client.sell_shares(
    &user,
    &market_id,
    &0,
    &shares,
    &usdc_token
);
```

## Mathematical Properties

### Constant Product Formula
```
x * y = k

Where:
- x = USDC reserve
- y = Share reserve  
- k = Constant product (invariant)
```

### Price Impact
```
Price = x / y

As x increases (more USDC in), y decreases (fewer shares available)
Therefore price increases with demand
```

### Slippage
```
Slippage = (execution_price - marginal_price) / marginal_price

Larger trades experience more slippage due to curve shape
```

## Security Considerations

1. **Invariant Protection:** `verify_invariant()` ensures mathematical integrity
2. **Fee Protection:** 0.3% fee prevents zero-cost arbitrage
3. **Authorization:** All trades require user authentication
4. **Circuit Breaker:** High-risk operations respect pause state
5. **Overflow Protection:** All arithmetic uses checked operations

## Gas Optimization

- Minimal storage operations per trade
- Single pool update per transaction
- No iteration over users or outcomes
- Efficient integer arithmetic (no floating point)

## Future Enhancements

1. **Dynamic Fees:** Adjust fees based on market conditions
2. **Liquidity Mining:** Reward early liquidity providers
3. **Multi-Asset Pools:** Support multiple collateral types
4. **Concentrated Liquidity:** Allow LPs to provide liquidity in specific price ranges

## Integration Notes

### Frontend Integration
```javascript
// Get quote before executing
const quotedShares = await contract.quote_buy(marketId, outcome, usdcAmount);

// Execute buy
const [sharesOut, price] = await contract.buy_shares(
    user, marketId, outcome, usdcAmount, tokenAddress
);

// Display to user
console.log(`Bought ${sharesOut} shares at ${price} per share`);
```

### Indexer Integration
- Track `buy_shares` and `sell_shares` calls
- Monitor pool reserves for liquidity depth
- Calculate 24h volume and price changes
- Detect arbitrage opportunities

## Testing

Run all AMM tests:
```bash
cd contracts/predict-iq
cargo test --lib test_amm
```

All 7 tests pass:
- ✅ test_amm_buy_increases_price
- ✅ test_amm_buy_sell_roundtrip_with_slippage  
- ✅ test_amm_pool_invariant_maintained
- ✅ test_amm_usdc_backing_matches_shares
- ✅ test_amm_quote_accuracy
- ✅ test_amm_insufficient_shares_error
- ✅ test_amm_multiple_outcomes_independent

## Deployment Checklist

- [ ] Deploy contract with AMM module
- [ ] Initialize pools for existing markets
- [ ] Set appropriate initial liquidity levels
- [ ] Monitor pool health and invariants
- [ ] Set up frontend integration
- [ ] Configure indexer for AMM events
- [ ] Test on testnet before mainnet

---

**Implementation Date:** 2026-02-23  
**Issue:** #26  
**Branch:** `features/issue-26-virtual-amm-cpmm-bet-liquidity`
