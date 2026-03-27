use crate::types::{Market, MarketStatus, Guardian};
use crate::modules::{markets, governance};
use soroban_sdk::{Env, Vec};

/// Paginated retrieval of markets.
///
/// Returns a segment of all markets created, regardless of status.
/// This prevents resource limit exhaustion (gas/memory) on large datasets.
///
/// # Arguments
/// * `offset` - Starting index for pagination (0-based)
/// * `limit` - Maximum number of markets to return
pub fn get_markets(e: &Env, offset: u32, limit: u32) -> Vec<Market> {
    let count: u64 = e
        .storage()
        .instance()
        .get(&markets::DataKey::MarketCount)
        .unwrap_or(0);
    
    let mut markets_vec = Vec::new(e);
    let start = (offset as u64).min(count);
    let end = (start + limit as u64).min(count);
    
    // Counting from 1 since market IDs are 1-based in this contract
    for i in (start + 1)..=(end) {
        if let Some(market) = markets::get_market(e, i) {
            markets_vec.push_back(market);
        }
    }
    
    markets_vec
}

/// Paginated retrieval of markets by status.
///
/// Returns a segment of markets that match the specified status.
/// Implementation iterates backwards from the newest markets to prioritize freshness.
///
/// # Arguments
/// * `status` - The status to filter by (e.g., Active, Resolved)
/// * `offset` - Starting element in the filtered list
/// * `limit` - Maximum number of markets to return
pub fn get_markets_by_status(e: &Env, status: MarketStatus, offset: u32, limit: u32) -> Vec<Market> {
    let count: u64 = e
        .storage()
        .instance()
        .get(&markets::DataKey::MarketCount)
        .unwrap_or(0);
    
    let mut markets_vec = Vec::new(e);
    let mut found_count = 0;
    let mut skipped_count = 0;
    
    // Status-based search requires iteration.
    // Iterating backwards from the latest market for fresher results.
    for i in (1..=count).rev() {
        if let Some(market) = markets::get_market(e, i) {
            if market.status == status {
                if skipped_count < offset {
                    skipped_count += 1;
                } else {
                    markets_vec.push_back(market);
                    found_count += 1;
                    if found_count >= limit {
                        break;
                    }
                }
            }
        }
    }
    
    markets_vec
}

/// Paginated retrieval of guardians.
///
/// Avoids the gas cost of loading the entire guardian set into memory
/// when the set grows large.
///
/// # Arguments
/// * `offset` - Starting index
/// * `limit` - Maximum number of guardians to return
pub fn get_guardians_paginated(e: &Env, offset: u32, limit: u32) -> Vec<Guardian> {
    let all_guardians = governance::get_guardians(e);
    let mut segment = Vec::new(e);
    
    let start = offset.min(all_guardians.len());
    let end = (start + limit).min(all_guardians.len());
    
    for i in start..end {
        if let Some(g) = all_guardians.get(i) {
            segment.push_back(g);
        }
    }
    
    segment
}
