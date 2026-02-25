import { Registry, Counter, Histogram, Gauge, collectDefaultMetrics } from 'prom-client';

export const register = new Registry();

// Collect default metrics (CPU, memory, etc.)
collectDefaultMetrics({ register });

// Custom metrics
export const httpRequestDuration = new Histogram({
  name: 'http_request_duration_seconds',
  help: 'Duration of HTTP requests in seconds',
  labelNames: ['method', 'route', 'status_code'],
  registers: [register],
});

export const httpRequestTotal = new Counter({
  name: 'http_requests_total',
  help: 'Total number of HTTP requests',
  labelNames: ['method', 'route', 'status_code'],
  registers: [register],
});

export const errorTotal = new Counter({
  name: 'errors_total',
  help: 'Total number of errors',
  labelNames: ['error_code', 'error_type'],
  registers: [register],
});

export const activeConnections = new Gauge({
  name: 'active_connections',
  help: 'Number of active connections',
  registers: [register],
});

export const blockchainRequestDuration = new Histogram({
  name: 'blockchain_request_duration_seconds',
  help: 'Duration of blockchain requests in seconds',
  labelNames: ['operation'],
  registers: [register],
});

export const blockchainErrors = new Counter({
  name: 'blockchain_errors_total',
  help: 'Total number of blockchain errors',
  labelNames: ['operation'],
  registers: [register],
});
