use soroban_sdk::{contracttype, Env, Vec};

use crate::modules::queries::MAX_PAGE_LIMIT;

#[contracttype]
pub enum DataKey {
    ArchivedMarketCount,
    ArchivedMarket(u64), // index -> market_id
}

/// Number of ledgers a persistent archive entry stays live before its TTL
/// needs to be extended again. Matches the bump pattern used by other
/// persistent per-entity data in this contract.
const ARCHIVE_TTL_THRESHOLD: u32 = 100;
const ARCHIVE_TTL_EXTEND: u32 = 1000;

/// Record a market ID as pruned (archived).
///
/// This provides a lightweight tombstone record so external indexers can
/// recognize that a market's on-chain data has been deleted for gas optimization.
///
/// Storage-cost tradeoff: the archive index is stored in *persistent* storage
/// rather than instance storage. Instance storage is a single footprint entry
/// shared by the whole contract instance, so every archived market written
/// there would be read/rehydrated and rent-managed together as one blob on
/// every invocation that touches instance storage — permanently inflating the
/// cost profile of the shared footprint as the archive grows without bound.
/// Persistent entries are independently rent-managed and only enter the
/// footprint when explicitly accessed, keeping the instance footprint constant.
pub fn archive_market(e: &Env, market_id: u64) {
    let mut count: u64 = e
        .storage()
        .persistent()
        .get(&DataKey::ArchivedMarketCount)
        .unwrap_or(0);

    count += 1;
    let key = DataKey::ArchivedMarket(count);
    e.storage().persistent().set(&key, &market_id);
    e.storage()
        .persistent()
        .extend_ttl(&key, ARCHIVE_TTL_THRESHOLD, ARCHIVE_TTL_EXTEND);

    let count_key = DataKey::ArchivedMarketCount;
    e.storage().persistent().set(&count_key, &count);
    e.storage()
        .persistent()
        .extend_ttl(&count_key, ARCHIVE_TTL_THRESHOLD, ARCHIVE_TTL_EXTEND);
}

/// Paginated retrieval of archived market IDs.
///
/// Efficiently returns a paginated segment of archived market IDs.
///
/// # Arguments
/// * `offset` - Starting global index (0-based)
/// * `limit` - Maximum number of IDs to return (clamped to `MAX_PAGE_LIMIT`)
pub fn get_archived_market_ids(e: &Env, offset: u32, limit: u32) -> Vec<u64> {
    let count: u64 = e
        .storage()
        .persistent()
        .get(&DataKey::ArchivedMarketCount)
        .unwrap_or(0);

    // Bound the page size to prevent unbounded gas/memory usage, matching
    // the clamp applied by every other paginated query in `queries.rs`.
    let limit = limit.min(MAX_PAGE_LIMIT);

    let mut archived_vec = Vec::new(e);
    let start = (offset as u64).min(count);
    let end = (start + limit as u64).min(count);

    // IDs are stored using 1-based indexing for the archive map keys
    for i in (start + 1)..=(end) {
        let key = DataKey::ArchivedMarket(i);
        if let Some(id) = e.storage().persistent().get(&key) {
            e.storage()
                .persistent()
                .extend_ttl(&key, ARCHIVE_TTL_THRESHOLD, ARCHIVE_TTL_EXTEND);
            archived_vec.push_back(id);
        }
    }

    archived_vec
}

/// Returns the total volume of archived (pruned) markets.
pub fn get_archived_count(e: &Env) -> u64 {
    e.storage()
        .persistent()
        .get(&DataKey::ArchivedMarketCount)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::modules::queries::MAX_PAGE_LIMIT;

    #[test]
    fn test_get_archived_market_ids_clamps_limit() {
        let e = Env::default();

        // Archive more entries than the page cap allows in a single call.
        let total = MAX_PAGE_LIMIT + 10;
        for i in 0..total {
            archive_market(&e, i as u64);
        }

        // Requesting a limit larger than the cap must be clamped.
        let result = get_archived_market_ids(&e, 0, u32::MAX);
        assert_eq!(result.len(), MAX_PAGE_LIMIT);

        // A limit within the cap is honored as-is.
        let small = get_archived_market_ids(&e, 0, 5);
        assert_eq!(small.len(), 5);
    }

    #[test]
    fn test_archive_uses_persistent_not_instance_storage() {
        let e = Env::default();

        // Archiving many markets must not write per-entry to instance storage;
        // the archive index lives entirely in persistent storage so the shared
        // instance footprint stays constant regardless of archive size.
        let total = 50u64;
        for i in 0..total {
            archive_market(&e, i);
        }

        assert_eq!(get_archived_count(&e), total);

        // The count and every per-entry index are readable from persistent
        // storage, and absent from instance storage.
        assert!(e
            .storage()
            .persistent()
            .has(&DataKey::ArchivedMarketCount));
        assert!(!e
            .storage()
            .instance()
            .has(&DataKey::ArchivedMarketCount));

        for i in 1..=total {
            let key = DataKey::ArchivedMarket(i);
            assert!(e.storage().persistent().has(&key));
            assert!(!e.storage().instance().has(&key));
        }
    }
}
