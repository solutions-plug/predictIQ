use soroban_sdk::{contracttype, Env, Vec};

#[contracttype]
pub enum DataKey {
    ArchivedMarketCount,
    ArchivedMarket(u64), // index -> market_id
}

/// Record a market ID as pruned (archived).
///
/// This provides a lightweight tombstone record so external indexers can 
/// recognize that a market's on-chain data has been deleted for gas optimization.
pub fn archive_market(e: &Env, market_id: u64) {
    let mut count: u64 = e
        .storage()
        .instance()
        .get(&DataKey::ArchivedMarketCount)
        .unwrap_or(0);
    
    count += 1;
    e.storage().instance().set(&DataKey::ArchivedMarket(count), &market_id);
    e.storage().instance().set(&DataKey::ArchivedMarketCount, &count);
}

/// Paginated retrieval of archived market IDs.
///
/// Efficiently returns a paginated segment of archived market IDs.
///
/// # Arguments
/// * `offset` - Starting global index (0-based)
/// * `limit` - Maximum number of IDs to return
pub fn get_archived_market_ids(e: &Env, offset: u32, limit: u32) -> Vec<u64> {
    let count: u64 = e
        .storage()
        .instance()
        .get(&DataKey::ArchivedMarketCount)
        .unwrap_or(0);
    
    let mut archived_vec = Vec::new(e);
    let start = (offset as u64).min(count);
    let end = (start + limit as u64).min(count);
    
    // IDs are stored using 1-based indexing for the archive map keys
    for i in (start + 1)..=(end) {
        if let Some(id) = e.storage().instance().get(&DataKey::ArchivedMarket(i)) {
            archived_vec.push_back(id);
        }
    }
    
    archived_vec
}

/// Returns the total volume of archived (pruned) markets.
pub fn get_archived_count(e: &Env) -> u64 {
    e.storage()
        .instance()
        .get(&DataKey::ArchivedMarketCount)
        .unwrap_or(0)
}
