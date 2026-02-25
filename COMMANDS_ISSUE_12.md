# Commands for Issue #12: Rate Limiting and Security

## Git Commands

### Push Branch
```bash
git push -u origin features/issue-12-rate-limiting-and-security
```

### Create PR (GitHub CLI)
```bash
gh pr create \
  --base develop \
  --head features/issue-12-rate-limiting-and-security \
  --title "feat: Implement Rate Limiting and Security Measures (Issue #12)" \
  --body-file PR_TEMPLATE_ISSUE_12.md \
  --label "backend,security,rate-limiting,high-priority"
```

### Alternative: Create PR via Web
```
https://github.com/YOUR_USERNAME/predictIQ/compare/develop...features/issue-12-rate-limiting-and-security
```

## Build & Test Commands

### Build
```bash
cd services/api
cargo build --release
```

### Run Tests
```bash
# Unit tests
cargo test security_tests

# All tests
cargo test

# Integration tests
./test_rate_limit.sh
```

### Run Server
```bash
cargo run --release
```

## Setup Commands

### Generate Security Keys
```bash
# API Key
openssl rand -hex 32

# Signing Secret
openssl rand -base64 64
```

### Configure Environment
```bash
# Copy example
cp .env.example .env

# Edit configuration
nano .env  # or your preferred editor
```

## Testing Commands

### Test Rate Limiting
```bash
# Global rate limit (100 req/min)
for i in {1..110}; do curl http://localhost:8080/health; done

# Newsletter rate limit (5 req/hour)
for i in {1..10}; do 
  curl -X POST http://localhost:8080/api/v1/newsletter/subscribe \
    -H "Content-Type: application/json" \
    -d '{"email":"test'$i'@example.com"}'; 
done
```

### Test Security Headers
```bash
curl -I http://localhost:8080/health
```

### Test Input Validation
```bash
# SQL injection attempt
curl "http://localhost:8080/api/content?page=1' OR '1'='1"

# Path traversal attempt
curl "http://localhost:8080/api/../../../etc/passwd"

# XSS attempt
curl "http://localhost:8080/api/content?search=<script>alert('xss')</script>"
```

### Test API Key Authentication
```bash
# Without API key (should fail)
curl -X POST http://localhost:8080/api/markets/1/resolve

# With valid API key (should succeed)
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: your-api-key"

# With invalid API key (should fail)
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: invalid-key"
```

### Test IP Whitelisting
```bash
# From whitelisted IP
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: your-api-key" \
  -H "X-Real-IP: 127.0.0.1"

# From non-whitelisted IP
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: your-api-key" \
  -H "X-Real-IP: 1.2.3.4"
```

### Run Full Test Suite
```bash
./test_rate_limit.sh
```

## Monitoring Commands

### Check Metrics
```bash
curl http://localhost:8080/metrics
```

### Check Health
```bash
curl http://localhost:8080/health
```

### View Logs
```bash
# If using systemd
journalctl -u predictiq-api -f

# If running directly
# Logs will appear in terminal
```

## Deployment Commands

### Production Build
```bash
cargo build --release --target x86_64-unknown-linux-gnu
```

### Run in Production
```bash
# Set environment variables
export API_KEYS="prod-key-1,prod-key-2"
export ADMIN_WHITELIST_IPS="office-ip-1,office-ip-2"
export REQUEST_SIGNING_SECRET="prod-secret"

# Run
./target/release/predictiq-api
```

### Docker (if applicable)
```bash
# Build
docker build -t predictiq-api:latest .

# Run
docker run -d \
  --name predictiq-api \
  -p 8080:8080 \
  -e API_KEYS="key1,key2" \
  -e ADMIN_WHITELIST_IPS="127.0.0.1" \
  predictiq-api:latest
```

## Maintenance Commands

### Update Dependencies
```bash
cargo update
cargo audit
```

### Check for Security Issues
```bash
cargo audit
cargo clippy -- -D warnings
```

### Format Code
```bash
cargo fmt
```

### Clean Build
```bash
cargo clean
cargo build --release
```

## Quick Reference

### Environment Variables
```bash
# Required
DATABASE_URL=postgres://user:pass@localhost/predictiq
REDIS_URL=redis://localhost:6379

# Security
API_KEYS=key1,key2,key3
ADMIN_WHITELIST_IPS=127.0.0.1,192.168.1.100
REQUEST_SIGNING_SECRET=secret-key
```

### Rate Limits
- Global: 100 req/min per IP
- Newsletter: 5 req/hour per IP
- Contact: 3 req/hour per IP
- Analytics: 1000 req/min per session
- Admin: 30 req/min per IP

### Security Headers
- Content-Security-Policy
- X-Frame-Options: DENY
- X-Content-Type-Options: nosniff
- X-XSS-Protection: 1; mode=block
- Strict-Transport-Security
- Referrer-Policy
- Permissions-Policy

## Troubleshooting Commands

### Check if Server is Running
```bash
curl http://localhost:8080/health
```

### Check Port Usage
```bash
# Linux/Mac
lsof -i :8080

# Windows
netstat -ano | findstr :8080
```

### Check Environment Variables
```bash
env | grep -E "(API_KEYS|ADMIN_WHITELIST|DATABASE_URL|REDIS_URL)"
```

### Test Database Connection
```bash
psql $DATABASE_URL -c "SELECT 1;"
```

### Test Redis Connection
```bash
redis-cli -u $REDIS_URL ping
```

## Documentation Commands

### View Documentation
```bash
# Security documentation
cat services/api/SECURITY.md

# Setup guide
cat services/api/SECURITY_SETUP.md

# Quick reference
cat QUICK_REFERENCE_ISSUE_12.md

# Implementation details
cat IMPLEMENTATION_ISSUE_12.md
```

### Generate Rust Docs
```bash
cargo doc --open
```

## All-in-One Test Script

```bash
#!/bin/bash
# Complete test suite

echo "Building..."
cargo build --release

echo "Running unit tests..."
cargo test security_tests

echo "Running integration tests..."
./test_rate_limit.sh

echo "Checking security headers..."
curl -I http://localhost:8080/health | grep -E "(content-security-policy|x-frame-options)"

echo "Testing rate limiting..."
for i in {1..110}; do 
  curl -s -o /dev/null -w "%{http_code}\n" http://localhost:8080/health
done | sort | uniq -c

echo "All tests complete!"
```

## Summary

### To Push and Create PR:
```bash
git push -u origin features/issue-12-rate-limiting-and-security
gh pr create --base develop --head features/issue-12-rate-limiting-and-security \
  --title "feat: Implement Rate Limiting and Security Measures (Issue #12)" \
  --body-file PR_TEMPLATE_ISSUE_12.md
```

### To Test Locally:
```bash
cd services/api
cargo test
./test_rate_limit.sh
```

### To Deploy:
```bash
cargo build --release
# Set environment variables
./target/release/predictiq-api
```

---

**All commands ready for Issue #12 implementation!** 🚀
