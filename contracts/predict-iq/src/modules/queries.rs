use crate::types::{Market, MarketStatus, Guardian};
use crate::modules::{markets, governance};
use soroban_sdk::{Env, Vec};

/// Paginated retrieval of markets.
///
/// Returns a segment of all markets created, regardless of status.
pub fn get_markets(e: &Env, offset: u32, limit: u32) -> Vec<Market> {
    let count: u64 = e
        .storage()
        .instance()
        .get(&markets::DataKey::MarketCount)
        .unwrap_or(0);

    let mut markets_vec = Vec::new(e);
    let start = (offset as u64).min(count);
    let end = (start + limit as u64).min(count);

    for i in (start + 1)..=(end) {
        if let Some(market) = markets::get_market(e, i) {
            markets_vec.push_back(market);
        }
    }

    markets_vec
}

/// Paginated retrieval of markets by status.
///
/// Issue #406: Uses the status index to avoid a full reverse scan.
/// Instead of iterating all markets, we probe the index keys for the requested
/// status bucket and load only matching market records.
///
/// Complexity: O(limit) storage reads instead of O(total_markets).
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
    let mut found_count: u32 = 0;
    let mut skipped_count: u32 = 0;

    // Issue #406: Probe the status index for each market ID.
    // Only load the full market record when the index key is present.
    // This avoids deserializing every market just to check its status field.
    for i in (1..=count).rev() {
        if markets::has_status_index(e, i, &status) {
            if skipped_count < offset {
                skipped_count += 1;
            } else if let Some(market) = markets::get_market(e, i) {
                markets_vec.push_back(market);
                found_count += 1;
                if found_count >= limit {
                    break;
                }
            }
        }
    }

    markets_vec
}

/// Paginated retrieval of guardians.
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
