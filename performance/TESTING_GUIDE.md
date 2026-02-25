# Performance Testing Guide

Complete guide for running and analyzing performance tests for PredictIQ.

## Prerequisites

### Backend Testing
- k6 installed ([installation guide](https://k6.io/docs/getting-started/installation/))
- API server running locally or accessible endpoint
- PostgreSQL and Redis running

### Contract Testing
- Rust toolchain installed
- Soroban CLI installed
- Contract built in release mode

## Running Tests

### Quick Start

```bash
# Navigate to performance directory
cd performance

# Install dependencies
npm install

# Run all backend tests
npm run test:all

# Generate HTML report
npm run report
```

### Individual Tests

#### Smoke Test (1 minute, 1 user)
Quick validation that all endpoints are working:
```bash
npm run test:smoke
```

#### Load Test (9 minutes, up to 100 users)
Standard load testing with realistic user patterns:
```bash
npm run test:load
```

#### Stress Test (30+ minutes, up to 400 users)
Progressive load increase to find breaking points:
```bash
npm run test:stress
```

#### Spike Test (10 minutes, sudden spike to 1000 users)
Tests system recovery from sudden traffic spikes:
```bash
npm run test:spike
```

#### Cache Test (2 minutes, 50 users)
Validates cache hit rates and performance:
```bash
k6 run backend/k6/cache-test.js
```

#### Rate Limit Test (30 seconds, 10 users)
Verifies rate limiting is working correctly:
```bash
k6 run backend/k6/rate-limit-test.js
```

### Custom Configuration

Set environment variables to customize tests:

```bash
# Custom API endpoint
export API_URL=https://api.predictiq.example.com

# Run load test
npm run test:load
```

## Understanding Results

### Key Metrics

#### Response Time
- **avg**: Average response time across all requests
- **p(95)**: 95th percentile - 95% of requests faster than this
- **p(99)**: 99th percentile - 99% of requests faster than this
- **max**: Slowest request

Target: p(95) < 200ms, p(99) < 500ms

#### Error Rate
Percentage of failed requests (non-2xx status codes)

Target: < 0.1%

#### Throughput
Requests per second the system can handle

Target: > 1000 req/s

#### Cache Hit Rate
Percentage of requests served from cache

Target: > 80%

### Reading Reports

After running tests, check the reports directory:

```bash
ls -la backend/reports/
```

Files generated:
- `*-summary.json`: Raw test data
- `performance-report.html`: Visual report (open in browser)

### Performance Thresholds

Defined in `config/thresholds.json`:

```json
{
  "backend": {
    "response_time": {
      "p95": 200,
      "p99": 500
    },
    "error_rate": {
      "max": 0.001
    }
  }
}
```

## Regression Testing

Compare current performance against baseline:

```bash
# Save current results as baseline
cp backend/reports/load-test-summary.json backend/reports/baseline-load-test-summary.json

# Make changes to code...

# Run tests again
npm run test:load

# Compare results
npm run compare
```

The comparison script will:
- Show side-by-side metrics
- Calculate percentage changes
- Flag regressions > 10%
- Exit with error code if regression detected

## Contract Benchmarks

Smart contract gas benchmarks are in `contracts/predict-iq/benches/`:

```bash
cd contracts/predict-iq

# Run benchmarks
cargo bench --bench gas_benchmark

# View results
cat benches/gas_benchmark.sh
```

## CI/CD Integration

Performance tests run automatically in GitHub Actions:

- **Pull Requests**: Smoke tests only
- **Main branch**: Full test suite
- **Nightly**: Extended stress tests

See `.github/workflows/performance.yml` for configuration.

### Viewing CI Results

1. Go to Actions tab in GitHub
2. Select "Performance Tests" workflow
3. Download artifacts for detailed reports

## Troubleshooting

### API Not Responding

```bash
# Check if API is running
curl http://localhost:8080/health

# Start API server
cd services/api
cargo run --release
```

### k6 Not Found

```bash
# Install k6 (macOS)
brew install k6

# Install k6 (Linux)
sudo apt-get install k6

# Install k6 (Windows)
choco install k6
```

### High Error Rates

Check:
1. Database connection
2. Redis connection
3. API logs for errors
4. Rate limiting configuration

### Slow Response Times

Investigate:
1. Database query performance
2. Cache hit rates
3. Network latency
4. Resource utilization (CPU, memory)

## Best Practices

### Before Testing

1. Use dedicated test environment
2. Ensure database has realistic data volume
3. Warm up caches with preliminary requests
4. Monitor system resources

### During Testing

1. Don't run tests on production
2. Monitor system metrics (CPU, memory, disk I/O)
3. Watch for error logs
4. Note any anomalies

### After Testing

1. Review all metrics, not just response time
2. Compare against previous baselines
3. Investigate any regressions
4. Document findings

## Advanced Usage

### Custom Test Scenarios

Create new test files in `backend/k6/`:

```javascript
import http from 'k6/http';
import { check } from 'k6';

export const options = {
  vus: 10,
  duration: '30s',
};

export default function () {
  const res = http.get('http://localhost:8080/api/v1/custom-endpoint');
  check(res, {
    'status is 200': (r) => r.status === 200,
  });
}
```

### Distributed Testing

For higher load, use k6 cloud or distributed execution:

```bash
k6 cloud backend/k6/load-test.js
```

### Continuous Monitoring

Set up Prometheus + Grafana for ongoing performance monitoring:

1. API exposes metrics at `/metrics`
2. Configure Prometheus to scrape endpoint
3. Create Grafana dashboards
4. Set up alerts for threshold violations

## Performance Optimization Tips

### Backend
- Enable response caching
- Optimize database queries
- Use connection pooling
- Implement rate limiting
- Enable compression

### Database
- Add appropriate indexes
- Optimize slow queries
- Use read replicas
- Configure connection pool size

### Caching
- Cache frequently accessed data
- Set appropriate TTLs
- Use cache warming strategies
- Monitor cache hit rates

## Support

For questions or issues:
- Check existing test results in CI
- Review API logs
- Consult team documentation
- Open an issue with test results attached
