# PredictIQ API Caching Service

This service adds a multi-layer cache strategy for API responses, database query results, and blockchain RPC data.

## Features

- Redis cache-aside implementation
- Stellar Soroban JSON-RPC integration with retry/backoff
- Network switching support (testnet/mainnet/custom)
- Contract query methods:
  - Market data
  - Platform statistics
  - User bets
  - Oracle results
- Event listener/sync worker with transaction monitoring
- Reorg-aware synchronization using confirmation lag and cursor tracking
- Cache layers:
  - API response cache (`api:v1:*`)
  - DB query cache (`dbq:v1:*`)
  - Blockchain data cache (`chain:v1:*`)
- Endpoint TTLs:
  - `/api/statistics`: 5 minutes
  - `/api/markets/featured`: 2 minutes
  - `/api/content`: 1 hour
- Cache warming on startup (statistics + featured markets)
- Invalidation endpoint for write flows (`POST /api/markets/:market_id/resolve`)
- Prometheus metrics at `/metrics` for cache hit/miss and latency
- PostgreSQL connection pooling (min 5 / max 25)
- Paginated content endpoint (`page`, `page_size`)

## Environment Variables

- `API_BIND_ADDR` (default: `0.0.0.0:8080`)
- `REDIS_URL` (default: `redis://127.0.0.1:6379`)
- `DATABASE_URL` (default: `postgres://postgres:postgres@127.0.0.1/predictiq`)
- `BLOCKCHAIN_NETWORK` (`testnet` | `mainnet` | `custom`, default: `testnet`)
- `BLOCKCHAIN_RPC_URL` (optional override for network endpoint)
- `PREDICTIQ_CONTRACT_ID` (contract identifier used for reads)
- `RPC_RETRY_ATTEMPTS` (default: `3`)
- `RPC_RETRY_BASE_DELAY_MS` (default: `200`)
- `EVENT_POLL_INTERVAL_SECS` (default: `5`)
- `TX_POLL_INTERVAL_SECS` (default: `4`)
- `CONFIRMATION_LEDGER_LAG` (default: `3`)
- `SYNC_MARKET_IDS` (comma-separated market IDs for background sync)
- `FEATURED_LIMIT` (default: `10`)
- `CONTENT_DEFAULT_PAGE_SIZE` (default: `20`)
- `SENDGRID_API_KEY` (required for newsletter confirmation emails)
- `FROM_EMAIL` (sender address for newsletter confirmation emails)
- `BASE_URL` (default: `http://localhost:8080`, used in confirmation links)

## Key Naming Convention

- API keys: `api:v1:<resource>`
- DB keys: `dbq:v1:<query-shape>`
- Chain keys: `chain:v1:<entity>`

Examples:

- `api:v1:statistics`
- `api:v1:featured_markets`
- `api:v1:content:page:1:size:20`
- `dbq:v1:featured_markets:limit:10`
- `chain:v1:market:42`

## Run

```bash
cargo run -p predictiq-api
```

## Blockchain Endpoints

- `GET /api/blockchain/health`
- `GET /api/blockchain/stats`
- `GET /api/blockchain/markets/:market_id`
- `GET /api/blockchain/users/:user/bets?page=1&page_size=20`
- `GET /api/blockchain/oracle/:market_id`
- `GET /api/blockchain/tx/:tx_hash` (also registers hash for ongoing monitor polling)

## Newsletter Endpoints

- `POST /api/v1/newsletter/subscribe` body: `{ "email": "user@example.com", "source": "direct" }`
- `GET /api/v1/newsletter/confirm?token=<token>`
- `DELETE /api/v1/newsletter/unsubscribe` body: `{ "email": "user@example.com" }`
- `GET /api/v1/newsletter/gdpr/export?email=user@example.com`
- `DELETE /api/v1/newsletter/gdpr/delete` body: `{ "email": "user@example.com" }`

The subscribe endpoint applies an in-memory per-IP limit of 5 attempts per 15 minutes.

Before using newsletter endpoints, apply [`sql/newsletter_schema.sql`](./sql/newsletter_schema.sql) to your database.

## Notes

- `getContractData` key shapes are currently convention-based (`market:<id>`, `platform:stats`, etc.). Align them to your deployed contract storage schema.
- `del_by_pattern` currently uses `KEYS` for clarity. For large production datasets, switch to a `SCAN` cursor strategy.
