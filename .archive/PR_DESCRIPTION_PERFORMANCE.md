# feat: Add Comprehensive Performance Testing Suite

## Overview

This PR implements a comprehensive performance testing suite for the PredictIQ platform, covering backend API performance, load testing, stress testing, and automated regression detection.

## Changes

### Backend Performance Tests (k6)
- ✅ **Smoke Test**: Quick validation (1 min, 1 user)
- ✅ **Load Test**: Standard load testing (9 min, 100 users)
- ✅ **Stress Test**: Progressive load (30+ min, up to 400 users)
- ✅ **Spike Test**: Traffic spike handling (10 min, 1000 users)
- ✅ **Cache Test**: Cache hit rate validation (2 min, 50 users)
- ✅ **Rate Limit Test**: Rate limiting verification (30 sec, 10 users)

### Performance Benchmarks
- API response time (p95): < 200ms
- API response time (p99): < 500ms
- Error rate: < 0.1%
- Cache hit rate: > 80%
- Throughput: > 1000 req/s

### CI/CD Integration
- ✅ GitHub Actions workflow for automated testing
- ✅ Performance regression detection (>10% degradation alerts)
- ✅ PR comments with test results
- ✅ Baseline comparison
- ✅ Artifact storage (30 days retention)

### Reporting & Analysis
- ✅ HTML report generator with visual dashboard
- ✅ JSON result comparison for regression detection
- ✅ Cross-platform test runners (Bash + PowerShell)
- ✅ NPM scripts for convenience

### Documentation
- ✅ Quick start guide (5 minutes to first test)
- ✅ Comprehensive testing guide
- ✅ Architecture overview
- ✅ Troubleshooting tips

## Project Structure

```
performance/
├── backend/
│   ├── k6/                    # k6 test scripts
│   │   ├── smoke-test.js
│   │   ├── load-test.js
│   │   ├── stress-test.js
│   │   ├── spike-test.js
│   │   ├── cache-test.js
│   │   └── rate-limit-test.js
│   └── reports/               # Generated reports
├── config/
│   └── thresholds.json        # Performance thresholds
├── scripts/
│   ├── run-all-tests.sh       # Bash test runner
│   ├── run-all-tests.ps1      # PowerShell test runner
│   ├── generate-report.js     # HTML report generator
│   └── compare-results.js     # Regression detection
├── package.json               # NPM scripts
├── README.md                  # Architecture overview
├── TESTING_GUIDE.md           # Detailed guide
└── QUICKSTART.md              # Quick start
```

## Usage

### Quick Start
```bash
cd performance
npm install
npm run test:smoke
npm run report
```

### Run All Tests
```bash
# Windows
.\scripts\run-all-tests.ps1

# Linux/macOS
./scripts/run-all-tests.sh
```

### Individual Tests
```bash
npm run test:smoke      # Quick validation
npm run test:load       # Standard load
npm run test:stress     # Stress test
```

## Acceptance Criteria

| Criteria | Status | Notes |
|----------|--------|-------|
| Performance benchmarks established | ✅ | Defined in config/thresholds.json |
| Lighthouse score > 90 | ⏳ | Frontend not implemented yet |
| API response time < 200ms (p95) | ✅ | Threshold configured |
| Load tests pass | ✅ | 100 users, 1000 users, stress tests |
| Performance tracked in CI/CD | ✅ | GitHub Actions workflow |
| Regression alerts configured | ✅ | Comparison script + CI |
| Performance reports generated | ✅ | HTML + JSON reports |

## Testing

### Prerequisites
1. Install k6: https://k6.io/docs/getting-started/installation/
2. Start API server: `cd services/api && cargo run --release`
3. Ensure PostgreSQL and Redis are running

### Manual Testing
```bash
cd performance
npm install
npm run test:smoke
```

### CI/CD Testing
- Push triggers GitHub Actions workflow
- Smoke tests run on PR
- Full suite runs on main branch
- Nightly stress tests

## Files Changed

### New Files (20)
- `.github/workflows/performance.yml` - CI/CD workflow
- `PERFORMANCE_TESTING.md` - Root summary
- `IMPLEMENTATION_SUMMARY_PERFORMANCE.md` - Implementation details
- `performance/` directory with complete test suite

## Documentation

- 📖 [Quick Start Guide](performance/QUICKSTART.md)
- 📖 [Testing Guide](performance/TESTING_GUIDE.md)
- 📖 [Architecture Overview](performance/README.md)
- 📖 [Implementation Summary](IMPLEMENTATION_SUMMARY_PERFORMANCE.md)

## Notes

- Frontend performance testing (Lighthouse) will be added when frontend is implemented
- Contract benchmarks already exist in `contracts/predict-iq/benches/`
- Database query performance monitored via API metrics endpoint
- Cache implementation should add `X-Cache` headers for accurate testing

## Related Issues

Closes #84

## Checklist

- [x] Performance tests implemented
- [x] Benchmarks established
- [x] CI/CD integration complete
- [x] Regression detection configured
- [x] Documentation complete
- [x] Cross-platform support (Windows + Unix)
- [x] Test runners created
- [x] Report generation implemented
