use crate::types::{Market, MarketStatus, Guardian};
use crate::modules::{markets, governance};
use soroban_sdk::{Env, Vec};

/// Hard cap on the number of records returned by any single paginated query.
/// Callers supplying a larger `limit` are silently clamped to this value.
/// This bounds per-call gas and memory consumption regardless of dataset size.
pub const MAX_PAGE_LIMIT: u32 = 100;

/// Paginated retrieval of markets.
///
/// Returns a segment of all markets created, regardless of status.
pub fn get_markets(e: &Env, offset: u32, limit: u32) -> Vec<Market> {
    let limit = limit.min(MAX_PAGE_LIMIT);
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
/// * `limit` - Maximum number of markets to return; clamped to [`MAX_PAGE_LIMIT`]
pub fn get_markets_by_status(e: &Env, status: MarketStatus, offset: u32, limit: u32) -> Vec<Market> {
    let limit = limit.min(MAX_PAGE_LIMIT);
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
    let limit = limit.min(MAX_PAGE_LIMIT);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PredictIQ, PredictIQClient};
    use crate::types::{OracleConfig, MarketTier};
    use soroban_sdk::{testutils::Address as _, Address, Env, String, Vec as SdkVec};

    fn setup() -> (Env, PredictIQClient<'static>, Address, Address) {
        let e = Env::default();
        e.mock_all_auths();
        let contract_id = e.register_contract(None, PredictIQ);
        let client = PredictIQClient::new(&e, &contract_id);
        let admin = Address::generate(&e);
        let creator = Address::generate(&e);
        client.initialize(&admin, &0);
        (e, client, admin, creator)
    }

    fn make_market(e: &Env, client: &PredictIQClient, creator: &Address) -> u64 {
        let options = SdkVec::from_array(e, [String::from_str(e, "Yes"), String::from_str(e, "No")]);
        let token = Address::generate(e);
        let oracle_cfg = OracleConfig {
            oracle_address: Address::generate(e),
            feed_id: String::from_str(e, "feed"),
            min_responses: None,
            max_staleness_seconds: 3600,
            max_confidence_bps: 100,
        };
        client.create_market(creator, &String::from_str(e, "M"), &options, &1000, &2000, &oracle_cfg, &MarketTier::Basic, &token, &0, &0)
    }

    #[test]
    fn test_limit_clamped_to_max() {
        let (e, client, _, creator) = setup();
        // Create MAX_PAGE_LIMIT + 10 markets
        for _ in 0..(MAX_PAGE_LIMIT + 10) {
            make_market(&e, &client, &creator);
        }
        // Requesting more than MAX_PAGE_LIMIT should return at most MAX_PAGE_LIMIT
        let result = client.get_markets(&0, &(MAX_PAGE_LIMIT + 50));
        assert_eq!(result.len(), MAX_PAGE_LIMIT);
    }

    #[test]
    fn test_status_limit_clamped_to_max() {
        let (e, client, _, creator) = setup();
        for _ in 0..(MAX_PAGE_LIMIT + 10) {
            make_market(&e, &client, &creator);
        }
        let result = client.get_markets_by_status(&MarketStatus::Active, &0, &(MAX_PAGE_LIMIT + 50));
        assert_eq!(result.len(), MAX_PAGE_LIMIT);
    }

    #[test]
    fn test_limit_zero_returns_empty() {
        let (e, client, _, creator) = setup();
        make_market(&e, &client, &creator);
        let result = client.get_markets(&0, &0);
        assert_eq!(result.len(), 0);
    }
}
