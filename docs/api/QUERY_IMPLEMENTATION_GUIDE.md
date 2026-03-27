# Query Implementation & Pagination Guide

This guide describes the pagination patterns used in PredictIQ smart contracts to handle large datasets while respecting Soroban's resource limits (gas and memory).

## 💡 Overview

Smart contracts on Stellar have strict limits on the amount of data they can return in a single call. Returning large vectors of data (markets, bets, etc.) can lead to "Resource Limit Exceeded" errors and contract failures. 

To prevent this, all heavy queries in PredictIQ utilize a consistent **Offset/Limit Pagination Pattern**.

---

## 🛠️ Pagination Pattern

The contract provides paginated methods for most read-only data retrieval. 

### Core Query Pattern

```rust
pub fn get_markets(e: Env, offset: u32, limit: u32) -> Vec<Market> 
```

- **`offset`**: The starting index (0-based) relative to the total set size.
- **`limit`**: The maximum number of items to return in this specific call.
- **`Vec<T>`**: The returned subset of data.

---

## 🛰️ Available Paginated Queries

### 1. Markets
| Method | Description |
|--------|-------------|
| `get_markets(offset, limit)` | Retrieve all markets regardless of status. |
| `get_markets_by_status(status, offset, limit)` | Filter markets by state (Active, Resolved, etc.). |

### 2. Governance
| Method | Description |
|--------|-------------|
| `get_guardians_paginated(offset, limit)` | Retrieve the list of active guardians. |

### 3. Event Archive (Pruned Markets)
| Method | Description |
|--------|-------------|
| `get_archived_market_ids(offset, limit)` | Retrieve IDs of markets that have been pruned. |

---

## 🏗️ Implementation Details

### Markets Search Freshness
The `get_markets_by_status` query iterates **backwards** from the most recently created markets. This ensures that frontend integrations see the "freshest" data first when using standard pagination.

### Gas Optimization
Pagination is implemented using native `u64` iteration over known market count bounds to ensure $O(limit)$ performance rather than $O(total\_size)$.

```rust
for i in (start + 1)..=(end) {
    if let Some(market) = get_market(e, i) {
        markets_vec.push_back(market);
    }
}
```

---

## ⚠️ Best Practices for Integrators

1. **Set Reasonable Limits**: Always keep `limit` below 50. High limits may still hit total gas limits if structures are complex.
2. **Handle Sparse Results**: The pagination implementation handles empty results gracefully returning an empty `Vec`.
3. **Poll for Updates**: Use the `Archived` IDs to know when to remove local cached data that has been pruned on-chain.

---

*Last Updated: 2026*
