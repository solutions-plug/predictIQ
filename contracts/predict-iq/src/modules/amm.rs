use soroban_sdk::{Env, Address, contracttype};
use crate::errors::ErrorCode;

/// Virtual AMM Pool for each outcome using Constant Product Market Maker (x * y = k)
#[contracttype]
#[derive(Clone, Debug)]
pub struct AMMPool {
    pub usdc_reserve: i128,      // x: USDC reserve
    pub share_reserve: i128,     // y: Virtual share reserve
    pub k: i128,                 // Constant product k = x * y
    pub total_shares_issued: i128, // Track circulating shares
}

#[contracttype]
pub enum DataKey {
    Pool(u64, u32),              // market_id, outcome -> AMMPool
    UserShares(u64, Address, u32), // market_id, user, outcome -> shares owned
}

const INITIAL_LIQUIDITY: i128 = 1_000_000_0000000; // 1M with 7 decimals
const FEE_BPS: i128 = 30; // 0.3% fee in basis points

/// Initialize AMM pools for a market with equal initial liquidity
pub fn initialize_pools(e: &Env, market_id: u64, num_outcomes: u32, initial_usdc: i128) {
    let usdc_per_pool = initial_usdc / (num_outcomes as i128);
    
    for outcome in 0..num_outcomes {
        let pool = AMMPool {
            usdc_reserve: usdc_per_pool,
            share_reserve: INITIAL_LIQUIDITY,
            k: usdc_per_pool * INITIAL_LIQUIDITY,
            total_shares_issued: 0,
        };
        e.storage().persistent().set(&DataKey::Pool(market_id, outcome), &pool);
    }
}

/// Buy shares: Swap USDC for outcome shares
/// Returns: (shares_out, effective_price)
pub fn buy_shares(
    e: &Env,
    market_id: u64,
    buyer: Address,
    outcome: u32,
    usdc_in: i128,
) -> Result<(i128, i128), ErrorCode> {
    if usdc_in <= 0 {
        return Err(ErrorCode::InvalidBetAmount);
    }

    let mut pool: AMMPool = e.storage()
        .persistent()
        .get(&DataKey::Pool(market_id, outcome))
        .ok_or(ErrorCode::MarketNotFound)?;

    // Apply fee: usdc_in_after_fee = usdc_in * (10000 - fee_bps) / 10000
    let usdc_after_fee = (usdc_in * (10000 - FEE_BPS)) / 10000;
    
    // CPMM formula: shares_out = y - k / (x + usdc_in)
    // shares_out = share_reserve - (k / (usdc_reserve + usdc_after_fee))
    let new_usdc_reserve = pool.usdc_reserve + usdc_after_fee;
    let new_share_reserve = pool.k / new_usdc_reserve;
    let shares_out = pool.share_reserve - new_share_reserve;

    if shares_out <= 0 {
        return Err(ErrorCode::InvalidBetAmount);
    }

    // Update pool state
    pool.usdc_reserve = new_usdc_reserve;
    pool.share_reserve = new_share_reserve;
    pool.total_shares_issued += shares_out;

    // Update user shares
    let user_key = DataKey::UserShares(market_id, buyer.clone(), outcome);
    let current_shares: i128 = e.storage().persistent().get(&user_key).unwrap_or(0);
    e.storage().persistent().set(&user_key, &(current_shares + shares_out));

    // Save pool
    e.storage().persistent().set(&DataKey::Pool(market_id, outcome), &pool);

    // Calculate effective price: usdc_in / shares_out
    let effective_price = (usdc_in * 1_0000000) / shares_out; // Price with 7 decimals

    Ok((shares_out, effective_price))
}

/// Sell shares: Swap outcome shares back for USDC
/// Returns: (usdc_out, effective_price)
pub fn sell_shares(
    e: &Env,
    market_id: u64,
    seller: Address,
    outcome: u32,
    shares_in: i128,
) -> Result<(i128, i128), ErrorCode> {
    if shares_in <= 0 {
        return Err(ErrorCode::InvalidBetAmount);
    }

    // Check user has enough shares
    let user_key = DataKey::UserShares(market_id, seller.clone(), outcome);
    let user_shares: i128 = e.storage().persistent().get(&user_key).unwrap_or(0);
    
    if user_shares < shares_in {
        return Err(ErrorCode::InsufficientBalance);
    }

    let mut pool: AMMPool = e.storage()
        .persistent()
        .get(&DataKey::Pool(market_id, outcome))
        .ok_or(ErrorCode::MarketNotFound)?;

    // CPMM formula: usdc_out = x - k / (y + shares_in)
    // usdc_out = usdc_reserve - (k / (share_reserve + shares_in))
    let new_share_reserve = pool.share_reserve + shares_in;
    let new_usdc_reserve = pool.k / new_share_reserve;
    let usdc_out_before_fee = pool.usdc_reserve - new_usdc_reserve;

    // Apply fee on output
    let usdc_out = (usdc_out_before_fee * (10000 - FEE_BPS)) / 10000;

    if usdc_out <= 0 {
        return Err(ErrorCode::InvalidBetAmount);
    }

    // Update pool state
    pool.usdc_reserve = new_usdc_reserve;
    pool.share_reserve = new_share_reserve;
    pool.total_shares_issued -= shares_in;

    // Update user shares
    e.storage().persistent().set(&user_key, &(user_shares - shares_in));

    // Save pool
    e.storage().persistent().set(&DataKey::Pool(market_id, outcome), &pool);

    // Calculate effective price
    let effective_price = (usdc_out * 1_0000000) / shares_in;

    Ok((usdc_out, effective_price))
}

/// Get current price for buying 1 share (marginal price)
pub fn get_buy_price(e: &Env, market_id: u64, outcome: u32) -> Result<i128, ErrorCode> {
    let pool: AMMPool = e.storage()
        .persistent()
        .get(&DataKey::Pool(market_id, outcome))
        .ok_or(ErrorCode::MarketNotFound)?;

    // Marginal price = dx/dy at current point
    // For CPMM: price = x / y
    let price = (pool.usdc_reserve * 1_0000000) / pool.share_reserve;
    Ok(price)
}

/// Get user's share balance for an outcome
pub fn get_user_shares(e: &Env, market_id: u64, user: Address, outcome: u32) -> i128 {
    e.storage()
        .persistent()
        .get(&DataKey::UserShares(market_id, user, outcome))
        .unwrap_or(0)
}

/// Get pool state for an outcome
pub fn get_pool(e: &Env, market_id: u64, outcome: u32) -> Option<AMMPool> {
    e.storage().persistent().get(&DataKey::Pool(market_id, outcome))
}

/// Calculate output for a given input (for quotes/previews)
pub fn quote_buy(e: &Env, market_id: u64, outcome: u32, usdc_in: i128) -> Result<i128, ErrorCode> {
    let pool: AMMPool = e.storage()
        .persistent()
        .get(&DataKey::Pool(market_id, outcome))
        .ok_or(ErrorCode::MarketNotFound)?;

    let usdc_after_fee = (usdc_in * (10000 - FEE_BPS)) / 10000;
    let new_usdc_reserve = pool.usdc_reserve + usdc_after_fee;
    let new_share_reserve = pool.k / new_usdc_reserve;
    let shares_out = pool.share_reserve - new_share_reserve;

    Ok(shares_out)
}

/// Calculate input needed for desired output (for quotes/previews)
pub fn quote_sell(e: &Env, market_id: u64, outcome: u32, shares_in: i128) -> Result<i128, ErrorCode> {
    let pool: AMMPool = e.storage()
        .persistent()
        .get(&DataKey::Pool(market_id, outcome))
        .ok_or(ErrorCode::MarketNotFound)?;

    let new_share_reserve = pool.share_reserve + shares_in;
    let new_usdc_reserve = pool.k / new_share_reserve;
    let usdc_out_before_fee = pool.usdc_reserve - new_usdc_reserve;
    let usdc_out = (usdc_out_before_fee * (10000 - FEE_BPS)) / 10000;

    Ok(usdc_out)
}

/// Verify pool invariant (for testing/auditing)
pub fn verify_invariant(e: &Env, market_id: u64, outcome: u32) -> Result<bool, ErrorCode> {
    let pool: AMMPool = e.storage()
        .persistent()
        .get(&DataKey::Pool(market_id, outcome))
        .ok_or(ErrorCode::MarketNotFound)?;

    let current_k = pool.usdc_reserve * pool.share_reserve;
    // Allow small rounding errors (0.01%)
    let diff = if current_k > pool.k {
        current_k - pool.k
    } else {
        pool.k - current_k
    };
    
    let tolerance = pool.k / 10000; // 0.01% tolerance
    Ok(diff <= tolerance)
}
