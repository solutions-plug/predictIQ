# PredictIQ Backend API

Backend API service for PredictIQ with comprehensive error handling and monitoring.

## Features

✅ Centralized error handling middleware  
✅ Standardized error response format  
✅ Pino logging with pretty printing in development  
✅ Sentry error tracking integration  
✅ Prometheus metrics collection  
✅ Health check endpoints (`/health`, `/health/ready`, `/health/live`)  
✅ Request tracing and slow query logging  
✅ Graceful shutdown handling  

## Quick Start

### Installation

```bash
cd backend
npm install
```

### Configuration

Copy `.env.example` to `.env` and configure:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:
- `SENTRY_DSN`: Your Sentry DSN for error tracking
- `PORT`: API server port (default: 3000)
- `LOG_LEVEL`: Logging level (debug, info, warn, error)

### Development

```bash
npm run dev
```

### Production

```bash
npm run build
npm start
```

## API Endpoints

### Health Checks

- `GET /health` - Basic health check
- `GET /health/ready` - Readiness probe (checks dependencies)
- `GET /health/live` - Liveness probe

### Metrics

- `GET /metrics` - Prometheus metrics endpoint

## Error Response Format

All errors follow this standardized format:

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

## Monitoring

### Prometheus Metrics

The following metrics are collected:

- `http_request_duration_seconds` - HTTP request duration histogram
- `http_requests_total` - Total HTTP requests counter
- `errors_total` - Total errors counter
- `active_connections` - Active connections gauge
- `blockchain_request_duration_seconds` - Blockchain request duration
- `blockchain_errors_total` - Blockchain errors counter
- Default Node.js metrics (CPU, memory, etc.)

### Grafana Dashboard

Import the Prometheus metrics into Grafana for visualization:

1. Add Prometheus as a data source
2. Create dashboards for:
   - Request rate and latency
   - Error rates by type
   - Active connections
   - System resources (CPU, memory)

### Alerting

Configure alerts in Prometheus/Grafana for:

- High error rates (> 5% of requests)
- Slow requests (> 1s)
- High memory usage (> 80%)
- Service unavailability

## Logging

Logs are structured JSON in production and pretty-printed in development.

Log levels:
- `debug` - Detailed debugging information
- `info` - General informational messages
- `warn` - Warning messages (operational errors)
- `error` - Error messages (unexpected errors)

Slow queries (> 1s) are automatically logged as warnings.

## Error Tracking

Sentry captures:
- Unhandled exceptions
- Unhandled promise rejections
- Non-operational errors
- Request context and stack traces

## Architecture

```
backend/
├── src/
│   ├── middleware/
│   │   ├── errorHandler.ts      # Centralized error handling
│   │   ├── requestLogger.ts     # Request logging with slow query detection
│   │   └── metrics.ts           # Prometheus metrics collection
│   ├── routes/
│   │   └── health.ts            # Health check endpoints
│   ├── types/
│   │   └── errors.ts            # Error types and custom error class
│   ├── utils/
│   │   ├── logger.ts            # Pino logger configuration
│   │   ├── metrics.ts           # Prometheus metrics definitions
│   │   └── sentry.ts            # Sentry initialization
│   └── index.ts                 # Application entry point
├── package.json
├── tsconfig.json
└── .env.example
```

## Testing

```bash
npm test
```

## Deployment

### Docker

```dockerfile
FROM node:20-alpine
WORKDIR /app
COPY package*.json ./
RUN npm ci --only=production
COPY dist ./dist
EXPOSE 3000
CMD ["node", "dist/index.js"]
```

### Kubernetes

Health check configuration:

```yaml
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

## License

MIT
