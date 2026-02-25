# Quick Reference: Security Implementation (Issue #12)

## Quick Start

### 1. Setup Environment
```bash
cd services/api
cp .env.example .env
```

### 2. Generate Keys
```bash
# API Key
openssl rand -hex 32

# Signing Secret
openssl rand -base64 64
```

### 3. Configure .env
```bash
API_KEYS=your-generated-key
ADMIN_WHITELIST_IPS=127.0.0.1
REQUEST_SIGNING_SECRET=your-generated-secret
```

### 4. Build & Run
```bash
cargo build --release
cargo run --release
```

### 5. Test
```bash
./test_rate_limit.sh
```

## Rate Limits Quick Reference

| Endpoint | Limit | Window | Key |
|----------|-------|--------|-----|
| Global | 100 req | 1 min | IP |
| Newsletter | 5 req | 1 hour | IP |
| Contact | 3 req | 1 hour | IP |
| Analytics | 1000 req | 1 min | Session/IP |
| Admin | 30 req | 1 min | IP |

## Security Headers

```
Content-Security-Policy: default-src 'self'; ...
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: geolocation=(), microphone=(), camera=()
```

## API Endpoints

### Public (Rate Limited)
- `GET /health`
- `GET /metrics`
- `GET /api/blockchain/*`
- `GET /api/statistics`
- `GET /api/markets/featured`
- `GET /api/content`

### Newsletter (5 req/hour)
- `POST /api/v1/newsletter/subscribe`
- `GET /api/v1/newsletter/confirm`
- `DELETE /api/v1/newsletter/unsubscribe`
- `GET /api/v1/newsletter/gdpr/export`
- `DELETE /api/v1/newsletter/gdpr/delete`

### Admin (API Key + IP Whitelist)
- `POST /api/markets/:id/resolve`

## Testing Commands

### Rate Limiting
```bash
# Global
for i in {1..110}; do curl http://localhost:8080/health; done

# Newsletter
for i in {1..10}; do 
  curl -X POST http://localhost:8080/api/v1/newsletter/subscribe \
    -H "Content-Type: application/json" \
    -d '{"email":"test'$i'@example.com"}'; 
done
```

### Security Headers
```bash
curl -I http://localhost:8080/health
```

### Input Validation
```bash
# SQL Injection
curl "http://localhost:8080/api/content?page=1' OR '1'='1"

# Path Traversal
curl "http://localhost:8080/api/../../../etc/passwd"
```

### Authentication
```bash
# No key (401)
curl -X POST http://localhost:8080/api/markets/1/resolve

# Valid key
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: your-key"
```

## Environment Variables

### Required
```bash
DATABASE_URL=postgres://user:pass@localhost/predictiq
REDIS_URL=redis://localhost:6379
```

### Security (Optional)
```bash
API_KEYS=key1,key2,key3
ADMIN_WHITELIST_IPS=127.0.0.1,192.168.1.100
REQUEST_SIGNING_SECRET=secret
```

## File Structure

```
services/api/
├── src/
│   ├── security.rs          # Core security (rate limiting, auth)
│   ├── rate_limit.rs        # Endpoint-specific rate limits
│   ├── validation.rs        # Input validation middleware
│   ├── main.rs              # Updated with security layers
│   └── config.rs            # Security config
├── tests/
│   └── security_tests.rs    # Unit tests
├── SECURITY.md              # Full documentation
├── SECURITY_SETUP.md        # Setup guide
├── .env.example             # Config template
└── test_rate_limit.sh       # Test script
```

## Common Issues

### Rate Limit Not Working
- Check if rate limiter is initialized in `main.rs`
- Verify middleware is applied to routes
- Check IP extraction (proxy headers)

### Security Headers Missing
- Verify `security_headers_middleware` is applied
- Check middleware order in `main.rs`

### API Key Auth Failing
- Verify `API_KEYS` environment variable is set
- Check header name: `X-API-Key`
- Ensure middleware is applied to admin routes

### IP Whitelist Blocking
- Verify `ADMIN_WHITELIST_IPS` is set correctly
- Check IP extraction (proxy headers)
- Test with `127.0.0.1` first

## Monitoring

### Prometheus Metrics
```bash
curl http://localhost:8080/metrics
```

### Key Metrics
- `http_requests_total` - Total requests
- `http_request_duration_seconds` - Response times
- Rate limit violations (in logs)
- Authentication failures (in logs)

## Production Deployment

### Pre-Deployment
1. Generate production keys
2. Configure IP whitelist
3. Set up HTTPS/TLS
4. Enable Cloudflare/WAF
5. Configure Nginx

### Post-Deployment
1. Verify security headers
2. Test rate limiting
3. Test authentication
4. Monitor metrics
5. Check logs

## Support

- **Documentation**: `SECURITY.md`, `SECURITY_SETUP.md`
- **Tests**: `./test_rate_limit.sh`
- **Implementation**: `IMPLEMENTATION_ISSUE_12.md`

## Quick Commands

```bash
# Build
cargo build --release

# Test
cargo test security_tests

# Run
cargo run --release

# Test security
./test_rate_limit.sh

# Check headers
curl -I http://localhost:8080/health

# Generate key
openssl rand -hex 32
```
