# Issue #11 Quick Reference

## Quick Start

```bash
cd backend
npm install
cp .env.example .env
npm run dev
```

## Test Endpoints

```bash
curl http://localhost:3000/health
curl http://localhost:3000/health/ready
curl http://localhost:3000/health/live
curl http://localhost:3000/metrics
```

## Key Features

✅ **Error Handling** - Centralized middleware with standardized format  
✅ **Logging** - Pino with pretty printing (dev) and JSON (prod)  
✅ **Error Tracking** - Sentry integration  
✅ **Metrics** - Prometheus metrics collection  
✅ **Health Checks** - `/health`, `/health/ready`, `/health/live`  
✅ **Alerting** - Prometheus alert rules  
✅ **Slow Query Detection** - Automatic logging of requests > 1s  

## Error Response Format

```json
{
  "error": {
    "code": "VALIDATION_ERROR",
    "message": "Invalid email format",
    "details": {},
    "timestamp": "2024-01-01T00:00:00Z"
  }
}
```

## Throwing Errors

```typescript
import { AppError, ErrorCode } from './types/errors';

throw new AppError(
  ErrorCode.MARKET_NOT_FOUND,
  'Market not found',
  404,
  { marketId: 123 }
);
```

## Async Handlers

```typescript
import { asyncHandler } from './middleware/errorHandler';

router.get('/route', asyncHandler(async (req, res) => {
  // Your async code
}));
```

## Custom Metrics

```typescript
import { blockchainRequestDuration } from './utils/metrics';

const end = blockchainRequestDuration.startTimer({ operation: 'create_market' });
// ... do work
end();
```

## Docker

```bash
# Build
docker build -t predictiq-backend .

# Run
docker run -p 3000:3000 --env-file .env predictiq-backend

# Full stack (with Prometheus + Grafana)
docker-compose up -d
```

## Monitoring URLs

- Backend: http://localhost:3000
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3001 (admin/admin)
- Metrics: http://localhost:3000/metrics

## Alert Rules

- HighErrorRate: > 5% errors for 5min
- SlowRequests: p95 > 1s for 5min
- HighMemoryUsage: > 80% for 5min
- ServiceDown: Down for 1min
- HighBlockchainErrorRate: > 0.1 errors/sec
- HighActiveConnections: > 1000 connections

## Environment Variables

```bash
NODE_ENV=development
PORT=3000
SENTRY_DSN=your_sentry_dsn
SENTRY_ENVIRONMENT=development
LOG_LEVEL=info
```

## Scripts

```bash
npm run dev      # Development with hot reload
npm run build    # Build TypeScript
npm start        # Production
npm test         # Run tests
npm run lint     # Lint code
```

## File Structure

```
backend/
├── src/
│   ├── middleware/     # Error handler, logger, metrics
│   ├── routes/         # Health checks
│   ├── types/          # Error types
│   ├── utils/          # Logger, metrics, Sentry
│   └── index.ts        # Main app
├── prometheus.yml      # Prometheus config
├── alerts.yml          # Alert rules
├── grafana-dashboard.json
└── docker-compose.yml
```

## Next Steps

1. Configure Sentry DSN in `.env`
2. Add API routes for markets, betting, voting
3. Connect to Stellar blockchain
4. Add authentication
5. Add rate limiting
6. Write tests
7. Deploy to production
