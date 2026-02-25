# Performance Testing Quick Start

Get started with performance testing in 5 minutes.

## Prerequisites

1. **Install k6** (load testing tool)

   **Windows:**
   ```powershell
   choco install k6
   ```
   Or download from: https://k6.io/docs/getting-started/installation/

   **macOS:**
   ```bash
   brew install k6
   ```

   **Linux:**
   ```bash
   sudo apt-get install k6
   ```

2. **Start the API server**
   ```bash
   cd services/api
   cargo run --release
   ```

## Run Your First Test

```bash
# Navigate to performance directory
cd performance

# Install Node dependencies
npm install

# Run smoke test (1 minute)
npm run test:smoke
```

## View Results

Results are saved in `backend/reports/`:

```bash
# Generate HTML report
npm run report

# Open the report (Windows)
start backend/reports/performance-report.html

# Open the report (macOS)
open backend/reports/performance-report.html

# Open the report (Linux)
xdg-open backend/reports/performance-report.html
```

## Run All Tests

```bash
# Run complete test suite (30+ minutes)
npm run test:all

# Or use PowerShell script (Windows)
.\scripts\run-all-tests.ps1

# Or use Bash script (Linux/macOS)
./scripts/run-all-tests.sh
```

## What Gets Tested

- ✅ API response times (p95 < 200ms)
- ✅ Error rates (< 0.1%)
- ✅ Load handling (100 concurrent users)
- ✅ Stress testing (up to 400 users)
- ✅ Cache performance (> 80% hit rate)
- ✅ Rate limiting

## Next Steps

- Read [TESTING_GUIDE.md](./TESTING_GUIDE.md) for detailed documentation
- Check [README.md](./README.md) for architecture overview
- Review performance thresholds in `config/thresholds.json`
- Set up CI/CD with `.github/workflows/performance.yml`

## Troubleshooting

**API not responding?**
```bash
# Check if API is running
curl http://localhost:8080/health
```

**k6 command not found?**
- Reinstall k6 following the installation instructions above
- Restart your terminal after installation

**Tests failing?**
- Ensure PostgreSQL and Redis are running
- Check API logs for errors
- Verify database migrations are applied
