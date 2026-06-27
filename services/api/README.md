# predictiq-api

Rust/Axum HTTP API service for the PredictIQ platform.

## Running

```bash
cargo run -p predictiq-api
```

## Running Tests

```bash
# All tests (unit + integration)
cargo test -p predictiq-api

# Integration tests only
cargo test -p predictiq-api --test integration_test

# Security / unit tests
cargo test -p predictiq-api --test security_tests

# Single-run (no watch mode)
cargo test -p predictiq-api -- --nocapture
```

## Environment Variables

| Variable | Default | Description |
|---|---|---|
| `API_BIND_ADDR` | `0.0.0.0:8080` | TCP address to listen on |
| `DATABASE_URL` | `postgres://postgres:postgres@127.0.0.1/predictiq` | PostgreSQL connection string |
| `REDIS_URL` | `redis://127.0.0.1:6379` | Redis connection string |
| `DB_POOL_MIN_CONNECTIONS` | `5` | Minimum pool connections |
| `DB_POOL_MAX_CONNECTIONS` | `25` | Maximum pool connections |
| `DB_POOL_ACQUIRE_TIMEOUT_SECS` | `5` | Seconds to wait for a free connection |
| `DB_POOL_IDLE_TIMEOUT_SECS` | _(sqlx default)_ | Seconds before idle connections are reaped |
| `DB_POOL_MAX_LIFETIME_SECS` | _(sqlx default)_ | Max lifetime of a connection |
| `DB_QUERY_TIMEOUT_SECS` | `30` | Per-query execution timeout |
| `DB_STATEMENT_TIMEOUT_MS` | `30000` | PostgreSQL `statement_timeout` per connection (ms) |
| `DB_LOCK_TIMEOUT_MS` | `10000` | PostgreSQL `lock_timeout` per connection (ms) |

### Recommended production pool settings

| Scenario | `MIN` | `MAX` | `ACQUIRE_TIMEOUT` | `IDLE_TIMEOUT` | `MAX_LIFETIME` |
|---|---|---|---|---|---|
| Low traffic / staging | `2` | `10` | `5s` | `300s` | `1800s` |
| Standard production | `5` | `25` | `5s` | `600s` | `1800s` |
| High-throughput production | `10` | `50` | `10s` | `600s` | `3600s` |

Rule of thumb: `MAX` ≤ PostgreSQL `max_connections` minus connections reserved for migrations, pg_bouncer, and maintenance sessions.

### Pool metrics

The following Prometheus gauges are exported on `/metrics` and updated on each scrape:

| Metric | Description |
|---|---|
| `db_pool_size` | Total connections in the pool (idle + active) |
| `db_pool_idle` | Idle connections waiting for work |
| `db_pool_active` | Connections currently executing a query |
| `BLOCKCHAIN_RPC_URL` | testnet default | Soroban RPC endpoint |
| `PREDICTIQ_CONTRACT_ID` | `predictiq_contract` | On-chain contract ID |
| `API_KEYS` | _(none)_ | Comma-separated admin API keys |
| `ADMIN_WHITELIST_IPS` | _(none)_ | Comma-separated IPs allowed to hit admin routes |
| `TRUST_PROXY` | `true` | Trust `X-Forwarded-For` header |
| `METRICS_PUBLIC` | `false` | Expose `/metrics` without auth |
| `HMAC_KEY` | _(required)_ | Current HMAC secret key for signing tokens |
| `HMAC_KEY_PREVIOUS` | _(none)_ | Previous HMAC key for zero-downtime key rotation |
| `HMAC_KEY_ROTATION_GRACE_SECONDS` | `3600` | Grace period (seconds) for accepting tokens signed with the previous key |

See `DATABASE.md` for database-specific configuration.

## HMAC Key Rotation

To rotate the HMAC key without downtime:

1. **Generate a new key** and set it as `HMAC_KEY_PREVIOUS`:
   ```bash
   export HMAC_KEY_PREVIOUS="<current-value-of-HMAC_KEY>"
   ```
   Deploy the API with this change. Tokens signed with both keys are now accepted.

2. **Promote the new key** by setting the new key as `HMAC_KEY` and clearing `HMAC_KEY_PREVIOUS`:
   ```bash
   export HMAC_KEY="<new-key-value>"
   unset HMAC_KEY_PREVIOUS  # or set to empty
   ```
   Deploy the API. All future tokens are signed with the new key.

3. **Grace period**: By default, tokens signed with `HMAC_KEY_PREVIOUS` are accepted for 3600 seconds (1 hour). Adjust with `HMAC_KEY_ROTATION_GRACE_SECONDS` if needed. Tokens beyond the grace period are rejected.

**Example timeline**:
- 12:00 PM: Deploy with `HMAC_KEY_PREVIOUS` set to old key. New tokens use new key; old tokens still accepted.
- 1:00 PM: Grace period expires. Old tokens are now rejected.
- Deploy with `HMAC_KEY_PREVIOUS` unset to complete rotation.
