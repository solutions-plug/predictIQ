# Metrics Reference

This document describes all Prometheus metrics exposed by the API service,
their labels, and the **cardinality policy** that must be followed when
adding or modifying metrics.

## Metric Inventory

| Metric | Type | Labels | Emitted From |
|--------|------|--------|--------------|
| `cache_hits_total` | Counter | `layer`, `endpoint` | DB, chain, API handlers |
| `cache_misses_total` | Counter | `layer`, `endpoint` | DB, chain, API handlers |
| `cache_invalidations_total` | Counter | `scope` | Market resolve, reorg, pagination |
| `http_request_duration_seconds` | Histogram | `route`, `status_code` | API response handlers |
| `rpc_errors_total` | Counter | `method` | Blockchain client |
| `rpc_fallbacks_total` | Counter | `endpoint` | Blockchain client |
| `db_timeouts_total` | Counter | `operation` | Database query wrapper |
| `email_dlq_size` | Gauge | *(none)* | Email queue handler |
| `email_queue_depth` | Gauge | *(none)* | Email queue handler |
| `db_pool_connections_active` | Gauge | `pool` | `/metrics` render |
| `db_pool_connections_idle` | Gauge | `pool` | `/metrics` render |
| `db_pool_acquire_duration_seconds` | Histogram | `pool` | Pool checkout hook |
| `rate_limit_rejections_total` | Counter | `route` | Rate-limit middleware |
| `cache_circuit_breaker_state` | Gauge | *(none)* | Health endpoint |

## Cardinality Policy

### Maximum Unique Values

No metric label may exceed **1 000 unique values** in production. If a label
is expected to exceed this bound, the value must be bucketed or normalised
before it reaches the counter/gauge.

### Hard Bans

The following value patterns are **never permitted** as Prometheus label
values:

| Pattern | Reason |
|---------|--------|
| Raw IP addresses | Cardinality scales with client count |
| User IDs / wallet addresses | Per-user series multiply linearly with user base |
| Market IDs | Per-market series multiply with active markets |
| Request paths with query strings | Each unique query string is a new series |
| Transaction hashes | One-time values create immortal time series |
| UUIDs or other random tokens | Unbounded by design |

### Normalisation Rules

All label values are passed through `normalize_label()` in
`services/api/src/metrics.rs` before being registered:

1. **Lowercase** all ASCII characters.
2. **Replace** any non-alphanumeric character with `_`.
3. **Trim** leading/trailing `_`.
4. **Truncate** values longer than 48 characters and append the suffix
   `_hotlbl` so operators can detect and bucket them in queries.

### Known-Bounded Labels

The following labels are currently confined to a static, bounded set of
values. New values must be added to the list of accepted constants in the
relevant code path; dynamic or user-supplied strings are not allowed.

| Label | Current Values | Bound |
|-------|---------------|-------|
| `layer` | `api`, `db`, `chain` | 3 |
| `route` | `statistics`, `featured_markets`, `content`, … | ≤ number of handlers |
| `endpoint` | `statistics`, `featured_markets`, `content`, `market_data`, `platform_stats`, `user_bets`, `oracle_result`, `tx_status`, `health` | ≤ 10 |
| `scope` | `market_resolve`, `events_pagination_pages`, `chain_reorg`, `tx_watch_eviction` | ≤ 4 |
| `method` | `getContractData`, `getTransaction`, `getLatestLedger`, `getEvents` | ≤ 4 |
| `pool` | `pool_{max_connections}` e.g. `pool_10` | ≤ configured pool sizes |
| `status_code` | HTTP status code integers (200, 404, 429, 500, …) | ≤ standard codes |
| `operation` | Named database query functions, e.g. `statistics`, `featured_markets` | ≤ query count |

### Adding a New Metric

1. Define the metric in `services/api/src/metrics.rs`.
2. Register it in `Metrics::new()`.
3. Add at least **one** label that is bounded by a fixed set of constants.
4. Never pass raw request data, user input, or database IDs as label values.
5. Document the new metric in this file and add a Grafana panel if needed.

### Review Checklist

- [ ] Every label has a bounded set of possible values defined above.
- [ ] No label value can be derived from user-supplied input without bucketing.
- [ ] `normalize_label()` is applied to all label values (already enforced by the helper methods on `Metrics`).
- [ ] The metric is registered in `Metrics::new()`.
- [ ] The metric is documented in this file.
- [ ] A Prometheus recording rule or alert is added if the metric signals a failure condition (see `performance/config/alerts.yaml`).

## Grafana Dashboards and Alerts

Relevant Grafana panels: `performance/config/grafana-dashboard.json`.

Relevant alerting rules: `performance/config/alerts.yaml`.

## Related Files

- `services/api/src/metrics.rs` — metric definitions
- `services/api/src/db.rs` — database metric call sites
- `services/api/src/blockchain.rs` — RPC metric call sites
- `services/api/src/handlers.rs` — HTTP latency and cache metric call sites
- `services/api/src/rate_limit.rs` — rate-limit rejection call sites
