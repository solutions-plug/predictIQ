# Performance Testing Suite

Comprehensive performance testing and benchmarking for PredictIQ platform.

## Overview

This suite provides performance testing for:
- Backend API endpoints (response times, throughput, load handling)
- Smart contract operations (gas costs, execution times)
- Database query performance
- Cache hit rates
- Rate limiting verification

## Structure

```
performance/
├── backend/           # Backend API performance tests
│   ├── k6/           # k6 load testing scripts
│   └── reports/      # Performance test reports
├── contracts/        # Smart contract benchmarks (see contracts/predict-iq/benches/)
├── config/           # Performance test configurations
└── scripts/          # Utility scripts for running tests
```

## Quick Start

```bash
# Install dependencies
cd performance
npm install

# Run backend load tests
npm run test:load

# Run stress tests
npm run test:stress

# Generate performance report
npm run report
```

## Performance Targets

### Backend API
- Response time (p95): < 200ms
- Response time (p99): < 500ms
- Throughput: > 1000 req/s
- Error rate: < 0.1%

### Load Scenarios
- Normal load: 100 concurrent users
- Peak load: 1000 concurrent users
- Stress test: Progressive load until failure

### Database
- Query time (p95): < 50ms
- Connection pool utilization: < 80%
- Cache hit rate: > 80%

## Tools

- **k6**: Load testing and performance benchmarking
- **Prometheus**: Metrics collection
- **Custom scripts**: Performance regression detection

## CI/CD Integration

Performance tests run automatically on:
- Pull requests (smoke tests)
- Main branch commits (full suite)
- Nightly builds (extended stress tests)

See `.github/workflows/performance.yml` for CI configuration.
