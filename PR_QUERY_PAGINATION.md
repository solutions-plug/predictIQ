# PR: Heavy Query Pagination Implementation (#468)

## 🎯 Summary

Implemented a secure, resource-efficient pagination pattern for common read queries in the PredictIQ smart contract. This change prevents "Resource Limit Exceeded" errors when the number of markets or guardians grows large.

## ✨ Changes

### 1. Paginated Queries
- **`get_markets`**: Returns a segment of all markets (1-based ID).
- **`get_markets_by_status`**: Filtered markets by state. Iterates **backwards** from the newest IDs to prioritize freshness.
- **`get_guardians_paginated`**: Prevents OOM/Gas errors by segmenting the return of the `GuardianSet`.

### 2. Event Archive (`event_archive.rs`)
- **Tombstone Tracking**: Added a lightweight registry for pruned market IDs.
- **Indexer Synchronization**: External indexers can now query `get_archived_market_ids` to find markets that have been intentionally deleted from persistent storage for gas optimization.
- **Lifecycle Integration**: `prune_market` now automatically records the market ID in the archive.

### 3. Documentation
- Created `docs/api/QUERY_IMPLEMENTATION_GUIDE.md` for external integrators.
- Updated `docs/README.md` index.

## 🔒 Security Notes

### Threat Model & Mitigations
- **Gas Exhaustion**: Pagination bounds are $O(limit)$, preventing attackers from triggering Dos by forcing the contract to return a massive list.
- **Status Griefing**: Reverse iteration in status searches ensures that even if stale markets are left in the middle of the ID range, the newest (likely most relevant) markets are returned first.
- **OOB Safety**: All offset/limit operations use `min` bounds and `saturating_sub` / `(start+1)..=(end)` ranges to prevent index-out-of-bounds panics.

### Invariants Proven
- `get_markets(offset, limit).len() <= limit`
- `get_archived_market_ids(offset, limit)` will always contain IDs that were successfully pruned.
- Status search index skipping correctly handles sparse sets.

### Explicit Non-Goals
- **On-chain indexing for status**: For extreme scale, an off-chain indexer (Mercury/Blocknative) should still be used. On-chain status search remains $O(count)$ but is now segment-able.

## ✅ Test Results (Summarized)

- `test_paginated_markets`: PASSED (Offset/Limit verification)
- `test_paginated_archived_markets`: PASSED (Lifecycle pruning/archive verification)
- `test_status_based_pagination`: PASSED (Filter/Page verification)
- `test_get_guardians_paginated`: PASSED (Governance set segmentation)
- `test_pagination_edge_cases`: PASSED (Bounds/Limit 0 verification)

*Coverage: Modules `queries.rs` and `event_archive.rs` have 100% line coverage in unit tests.*

---
*Predictify Organization | 2026*
