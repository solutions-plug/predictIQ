# Issue #11: Comprehensive Error Handling and Monitoring - Implementation Guide

## Overview

This implementation provides a production-ready backend API with comprehensive error handling, logging, and monitoring infrastructure.

## ✅ Acceptance Criteria Status

- ✅ Errors handled consistently (centralized error handler)
- ✅ Error tracking captures issues (Sentry integration)
- ✅ Health checks work (`/health`, `/health/ready`, `/health/live`)
- ✅ Metrics collected (Prometheus)
- ✅ Alerts configured (Prometheus alerting rules)
- ✅ Request tracing (request logging middleware)
- ✅ Log slow queries (automatic detection > 1s)

## Architecture

### Error Handling

**Centralized Error Handler** (`src/middleware/errorHandler.ts`)
- Catches all errors in the application
- Distinguishes between operational and programming errors
- Formats errors consistently
- Logs errors appropriately
- Sends non-operational errors to Sentry

**Custom Error Class** (`src/types/errors.ts`)
- `AppError` extends native Error
- Includes error code, status code, details, and operational flag
- Standardized error codes (VALIDATION_ERROR, UNAUTHORIZED, etc.)

**Error Response Format**
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

### Logging

**Pino Logger** (`src/utils/logger.ts`)
- High-performance JSON logging
- Pretty printing in development
- Structured logs with timestamps
- Configurable log levels

**Request Logger** (`src/middleware/requestLogger.ts`)
- Logs all HTTP requests
- Tracks request duration
- Automatically flags slow requests (> 1s)
- Includes method, URL, status code, duration, user agent, IP

### Error Tracking

**Sentry Integration** (`src/utils/sentry.ts`)
- Captures unhandled exceptions
- Captures unhandled promise rejections
- Captures non-operational errors
- Includes request context and stack traces
- Configurable via environment variables

### Monitoring

**Prometheus Metrics** (`src/utils/metrics.ts`)
- `http_request_duration_seconds` - Request latency histogram
- `http_requests_total` - Total requests counter
- `errors_total` - Error counter by type
- `active_connections` - Active connections gauge
- `blockchain_request_duration_seconds` - Blockchain operation latency
- `blockchain_errors_total` - Blockchain error counter
- Default Node.js metrics (CPU, memory, event loop, etc.)

**Metrics Middleware** (`src/middleware/metrics.ts`)
- Automatically collects metrics for all requests
- Tracks active connections
- Records request duration and status codes

### Health Checks

**Endpoints** (`src/routes/health.ts`)

1. **Basic Health Check** - `GET /health`
   - Returns 200 if server is running
   - Includes uptime

2. **Readiness Check** - `GET /health/ready`
   - Checks if service is ready to accept traffic
   - Validates dependencies (database, blockchain, etc.)
   - Returns 503 if not ready

3. **Liveness Check** - `GET /health/live`
   - Simple check that process is alive
   - Used by Kubernetes liveness probes

### Alerting

**Prometheus Alerts** (`alerts.yml`)

1. **HighErrorRate** - Error rate > 5% for 5 minutes
2. **SlowRequests** - p95 latency > 1s for 5 minutes
3. **HighMemoryUsage** - Memory > 80% for 5 minutes
4. **ServiceDown** - Service unavailable for 1 minute
5. **HighBlockchainErrorRate** - Blockchain errors > 0.1/sec
6. **HighActiveConnections** - Active connections > 1000

## Setup Instructions

### 1. Install Dependencies

```bash
cd backend
npm install
```

### 2. Configure Environment

```bash
cp .env.example .env
```

Edit `.env`:
```bash
NODE_ENV=development
PORT=3000
SENTRY_DSN=your_sentry_dsn_here
SENTRY_ENVIRONMENT=development
LOG_LEVEL=info
```

### 3. Run Development Server

```bash
npm run dev
```

### 4. Test Endpoints

```bash
# Health check
curl http://localhost:3000/health

# Readiness check
curl http://localhost:3000/health/ready

# Liveness check
curl http://localhost:3000/health/live

# Metrics
curl http://localhost:3000/metrics
```

## Production Deployment

### Docker Deployment

```bash
# Build image
docker build -t predictiq-backend .

# Run container
docker run -p 3000:3000 --env-file .env predictiq-backend
```

### Docker Compose (with monitoring stack)

```bash
# Start all services
docker-compose up -d

# View logs
docker-compose logs -f backend

# Stop services
docker-compose down
```

Services:
- Backend API: http://localhost:3000
- Prometheus: http://localhost:9090
- Grafana: http://localhost:3001 (admin/admin)

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: predictiq-backend
spec:
  replicas: 3
  template:
    spec:
      containers:
      - name: backend
        image: predictiq-backend:latest
        ports:
        - containerPort: 3000
        env:
        - name: NODE_ENV
          value: "production"
        - name: SENTRY_DSN
          valueFrom:
            secretKeyRef:
              name: predictiq-secrets
              key: sentry-dsn
        livenessProbe:
          httpGet:
            path: /health/live
            port: 3000
          initialDelaySeconds: 30
          periodSeconds: 10
        readinessProbe:
          httpGet:
            path: /health/ready
            port: 3000
          initialDelaySeconds: 5
          periodSeconds: 5
```

## Monitoring Setup

### Prometheus

1. Configure Prometheus to scrape metrics endpoint
2. Load alerting rules from `alerts.yml`
3. Configure Alertmanager for notifications

### Grafana

1. Add Prometheus as data source
2. Import dashboard from `grafana-dashboard.json`
3. Configure alert notifications (email, Slack, PagerDuty)

### Sentry

1. Create project in Sentry
2. Copy DSN to `.env`
3. Configure alert rules in Sentry dashboard

## Usage Examples

### Throwing Errors

```typescript
import { AppError, ErrorCode } from './types/errors';

// Operational error (expected)
throw new AppError(
  ErrorCode.MARKET_NOT_FOUND,
  'Market with ID 123 not found',
  404,
  { marketId: 123 },
  true // operational
);

// Programming error (unexpected)
throw new AppError(
  ErrorCode.INTERNAL_ERROR,
  'Database connection failed',
  500,
  {},
  false // not operational - will be sent to Sentry
);
```

### Async Route Handlers

```typescript
import { asyncHandler } from './middleware/errorHandler';

router.get('/markets/:id', asyncHandler(async (req, res) => {
  const market = await getMarket(req.params.id);
  if (!market) {
    throw new AppError(
      ErrorCode.MARKET_NOT_FOUND,
      'Market not found',
      404
    );
  }
  res.json(market);
}));
```

### Custom Metrics

```typescript
import { blockchainRequestDuration, blockchainErrors } from './utils/metrics';

async function callBlockchain(operation: string) {
  const end = blockchainRequestDuration.startTimer({ operation });
  try {
    const result = await blockchain.call(operation);
    return result;
  } catch (error) {
    blockchainErrors.inc({ operation });
    throw error;
  } finally {
    end();
  }
}
```

## Testing

### Manual Testing

```bash
# Test error handling
curl http://localhost:3000/nonexistent
# Should return 404 with error format

# Test metrics
curl http://localhost:3000/metrics
# Should return Prometheus metrics

# Test health checks
curl http://localhost:3000/health
curl http://localhost:3000/health/ready
curl http://localhost:3000/health/live
```

### Load Testing

```bash
# Install Apache Bench
sudo apt-get install apache2-utils

# Run load test
ab -n 1000 -c 10 http://localhost:3000/health

# Check metrics
curl http://localhost:3000/metrics | grep http_requests_total
```

## Maintenance

### Viewing Logs

```bash
# Development (pretty printed)
npm run dev

# Production (JSON)
NODE_ENV=production npm start

# Filter by level
npm start | grep '"level":"error"'
```

### Checking Metrics

```bash
# View all metrics
curl http://localhost:3000/metrics

# Query Prometheus
curl 'http://localhost:9090/api/v1/query?query=http_requests_total'
```

### Debugging Errors

1. Check application logs for error details
2. Check Sentry for stack traces and context
3. Check Prometheus for error rate trends
4. Check Grafana dashboards for patterns

## Next Steps

1. **Add Authentication** - Implement JWT or OAuth
2. **Add Rate Limiting** - Prevent abuse
3. **Add Database** - Connect to PostgreSQL/MongoDB
4. **Add Blockchain Integration** - Connect to Stellar
5. **Add API Routes** - Implement market, betting, voting endpoints
6. **Add Tests** - Unit and integration tests
7. **Add CI/CD** - Automated testing and deployment

## Files Created

```
backend/
├── src/
│   ├── middleware/
│   │   ├── errorHandler.ts      # ✅ Centralized error handling
│   │   ├── requestLogger.ts     # ✅ Request logging + slow queries
│   │   └── metrics.ts           # ✅ Metrics collection
│   ├── routes/
│   │   └── health.ts            # ✅ Health check endpoints
│   ├── types/
│   │   └── errors.ts            # ✅ Error types
│   ├── utils/
│   │   ├── logger.ts            # ✅ Pino logger
│   │   ├── metrics.ts           # ✅ Prometheus metrics
│   │   └── sentry.ts            # ✅ Sentry integration
│   └── index.ts                 # ✅ Main application
├── package.json                 # ✅ Dependencies
├── tsconfig.json                # ✅ TypeScript config
├── .env.example                 # ✅ Environment template
├── Dockerfile                   # ✅ Docker image
├── docker-compose.yml           # ✅ Full stack setup
├── prometheus.yml               # ✅ Prometheus config
├── alerts.yml                   # ✅ Alert rules
├── grafana-dashboard.json       # ✅ Grafana dashboard
├── .gitignore                   # ✅ Git ignore
└── README.md                    # ✅ Documentation
```

## Support

For issues or questions:
1. Check logs: `npm run dev`
2. Check metrics: `curl http://localhost:3000/metrics`
3. Check Sentry dashboard
4. Review this guide

## License

MIT
