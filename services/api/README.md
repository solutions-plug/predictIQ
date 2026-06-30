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
| `PREDICTIQ_CONTRACT_ID` | `predictiq_contract` | On-chain contract ID |
| `API_KEYS` | _(none)_ | Comma-separated admin API keys |
| `ADMIN_WHITELIST_IPS` | _(none)_ | Comma-separated IPs allowed to hit admin routes |
| `TRUST_PROXY` | `true` | Trust `X-Forwarded-For` header |
| `METRICS_PUBLIC` | `false` | Expose `/metrics` without auth |
| `HMAC_KEY` | _(required)_ | Current HMAC secret key for signing tokens |
| `HMAC_KEY_PREVIOUS` | _(none)_ | Previous HMAC key for zero-downtime key rotation |
| `HMAC_KEY_ROTATION_GRACE_SECONDS` | `3600` | Grace period (seconds) for accepting tokens signed with the previous key |

### Blockchain network configuration

| Variable | Default | Description |
|---|---|---|
| `BLOCKCHAIN_NETWORK` | `testnet` | Network to connect to: `testnet`, `mainnet`, or `custom` |
| `BLOCKCHAIN_RPC_URL` | _(network default)_ | Soroban RPC endpoint |
| `STELLAR_NETWORK_PASSPHRASE` | _(network default)_ | Expected network passphrase; validated against the RPC node at startup |
| `WATCHED_TX_TTL_SECS` | `1800` | TTL (seconds) for entries in the in-memory watched-transaction map. Entries older than this are evicted on the next write regardless of finalization status. Applied to the `expires_at` column of the `watched_transactions` DB table too. |
| `WATCHED_TX_MAX_SIZE` | `10000` | Maximum number of transaction hashes that may be tracked simultaneously. When the cap is reached, new `GET /api/v1/blockchain/tx/:hash` registrations return `503 Service Unavailable`. |
| `PREDICTIQ_ENV` | _(empty)_ | Set to `production` to make the Stellar RPC reachability startup probe fail-fast with `exit(1)` on failure. In all other environments only a warning is logged. |

Expected passphrases per `BLOCKCHAIN_NETWORK`:

| Network | Passphrase |
|---|---|
| `testnet` | `Test SDF Network ; September 2015` |
| `mainnet` | `Public Global Stellar Network ; September 2015` |
| `custom` | _(empty — validation skipped)_ |

At startup the API queries the RPC node's `getNetwork` endpoint. If the returned passphrase does not match the configured `STELLAR_NETWORK_PASSPHRASE`, the service rejects startup with a fatal error. This prevents silently signing transactions for the wrong network.

In production (`PREDICTIQ_ENV=production`) a mismatch or unreachable RPC causes `process::exit(1)`.  In development environments only a warning is logged and the process continues.

To disable validation entirely (e.g. for a local custom network without a fixed passphrase), leave `STELLAR_NETWORK_PASSPHRASE` unset.

### Health endpoints

| Endpoint | Description |
|---|---|
| `GET /health` | Liveness probe — checks Redis, DB, and email queue worker status |
| `GET /health/ready` | Readiness probe — validates the Stellar RPC endpoint is reachable and returns the expected network passphrase. Returns `200 OK` with `{ "ready": true, "stellar_rpc": "ok" }` on success, or `503 Service Unavailable` with `{ "ready": false, "stellar_rpc": "unreachable" }` on failure. Use this for Kubernetes `readinessProbe` configuration. |

### Watched-transaction metrics

The following Prometheus gauge is exported on `/metrics`:

| Metric | Description |
|---|---|
| `watched_tx_count` | Current number of transaction hashes being monitored in the in-memory watch map |

An alert (`WatchedTxCountHigh`) fires when `watched_tx_count` exceeds 8 000 (80% of the default 10 000 cap). A critical alert (`WatchedTxCountCritical`) fires when the map is full and new registrations are being rejected.

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

## TLS / HTTPS

TLS termination is handled **at the AWS Application Load Balancer (ALB)**, not
at the application layer.  The API service receives only plain HTTP traffic from
the ALB inside the VPC.

### ALB configuration

The Terraform module in `infrastructure/terraform/modules/ecs/` configures:

- **Port 80 (HTTP)** — issues a `301 Moved Permanently` redirect to port 443.
- **Port 443 (HTTPS)** — terminates TLS using the ACM certificate specified by
  `acm_certificate_arn`, then forwards decrypted traffic to the ECS target group.

### Application-layer HTTPS enforcement

The API supports an optional defense-in-depth redirect middleware activated by
environment variables:

| Variable | Default | Description |
|---|---|---|
| `APP_ENV` | `development` | Set to `production` in production deployments |
| `REQUIRE_HTTPS` | `false` | Set to `true` to activate the HTTP→HTTPS redirect middleware |

When `APP_ENV=production` and `REQUIRE_HTTPS=false`, a **WARNING** is logged at
startup to signal the potential misconfiguration.

When `REQUIRE_HTTPS=true`, the API middleware redirects any request whose
`X-Forwarded-Proto` header is `http` to the equivalent HTTPS URL.

**Recommended production settings:**
```bash
APP_ENV=production
REQUIRE_HTTPS=true
```

## API Key Rotation Runbook

API keys are managed in the `api_keys` database table.  Static env-var keys
(`API_KEYS`) are still supported and always checked first.

### List active keys

```bash
curl -H "X-Api-Key: $ADMIN_KEY" \
     https://api.predictiq.com/api/v1/admin/api-keys
```

Example response:
```json
[
  {
    "id": "d290f1ee-6c54-4b01-90e6-d701748f0851",
    "label": "ci-deploy-2026-06",
    "created_at": "2026-06-01T00:00:00Z",
    "expires_at": null,
    "is_expiring": false
  }
]
```

### Rotate a key

```bash
curl -X POST \
     -H "X-Api-Key: $ADMIN_KEY" \
     -H "Content-Type: application/json" \
     -d '{"key_label": "ci-deploy-2026-06", "overlap_days": 7}' \
     https://api.predictiq.com/api/v1/admin/api-keys/rotate
```

Example response:
```json
{
  "new_key": "4a3b2c1d...",
  "new_key_label": "ci-deploy-2026-06",
  "old_key_expires_at": "2026-07-07T05:54:35Z",
  "old_key_label": "ci-deploy-2026-06"
}
```

### Overlap window

During `overlap_days` (default: 7) **both the old key and the new key are
valid**.  Update all clients to use the new key before the overlap window
expires.  After `expires_at`, the old key is hard-deleted by the hourly
background cleanup task.

### Example rotation timeline

```
Day 0: POST /api/v1/admin/api-keys/rotate
         → new key issued, old key expires in 7 days
Day 0–7: Both old and new keys accepted by the API.
         Update CI/CD secrets and any other consumers.
Day 7:  Background task hard-deletes the old key.
         Only the new key is valid.
```

