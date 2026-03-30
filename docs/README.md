# PredictIQ Documentation

Welcome to the PredictIQ project documentation index.

## Contents

| Document | Description |
|----------|-------------|
| [API Specification](../API_SPEC.md) | Full on-chain contract API — methods, error codes, and events |
| [OpenAPI (REST)](../services/api/openapi.yaml) | REST API spec for the off-chain backend service |

## Repository Layout

```
predictIQ/
├── API_SPEC.md                  # On-chain contract API reference
├── contracts/
│   └── predict-iq/              # Soroban smart contract (Rust)
│       └── src/
│           ├── lib.rs           # Public contract entry points
│           ├── errors.rs        # ErrorCode enum
│           ├── types.rs         # Shared types and constants
│           └── modules/         # Feature modules (markets, bets, oracles, …)
├── services/
│   ├── api/                     # Rust/Axum backend API
│   │   ├── openapi.yaml         # REST API spec
│   │   ├── DATABASE.md          # Database schema and migration guide
│   │   └── database/
│   │       ├── migrations/      # Ordered SQL migration files
│   │       └── seeds/           # Development seed data
│   └── tts/                     # Text-to-speech microservice (TypeScript)
├── frontend/                    # Next.js frontend
└── docs/                        # This documentation index
    └── README.md
```

## Quick Links

- **Contract errors:** see [Error Codes](../API_SPEC.md#error-codes) in `API_SPEC.md`
- **Contract events:** see [Events](../API_SPEC.md#events) in `API_SPEC.md`
- **Database schema:** see `services/api/DATABASE.md`
- **REST endpoints:** see `services/api/openapi.yaml`

## Running Tests

```bash
# Run all API tests (unit + integration)
cd services/api && cargo test

# Run only the security integration tests
cd services/api && cargo test --test security_tests
```
