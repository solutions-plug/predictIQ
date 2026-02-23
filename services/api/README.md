# PredictIQ API Caching Service

This service adds a multi-layer cache strategy for API responses, database query results, and blockchain RPC data.

## Features

- Redis cache-aside implementation
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
- `BLOCKCHAIN_RPC_URL` (default: `https://soroban-testnet.stellar.org:443`)
- `FEATURED_LIMIT` (default: `10`)
- `CONTENT_DEFAULT_PAGE_SIZE` (default: `20`)

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

## Notes

- Replace the placeholder blockchain HTTP call in `src/blockchain.rs` with your production Soroban RPC call shape.
- `del_by_pattern` currently uses `KEYS` for clarity. For large production datasets, switch to a `SCAN` cursor strategy.
