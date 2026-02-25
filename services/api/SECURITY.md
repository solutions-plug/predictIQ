# API Security Implementation

## Overview

This document describes the comprehensive security measures implemented in the PredictIQ API to protect against abuse, attacks, and unauthorized access.

## Rate Limiting

### Global Rate Limiting
- **Limit**: 100 requests per minute per IP
- **Scope**: All API endpoints
- **Implementation**: IP-based tracking with sliding window

### Endpoint-Specific Rate Limiting

#### Newsletter Endpoints
- **Limit**: 5 requests per hour per IP
- **Endpoints**:
  - `POST /api/v1/newsletter/subscribe`
  - `GET /api/v1/newsletter/confirm`
  - `DELETE /api/v1/newsletter/unsubscribe`
  - `GET /api/v1/newsletter/gdpr/export`
  - `DELETE /api/v1/newsletter/gdpr/delete`

#### Contact Endpoints
- **Limit**: 3 requests per hour per IP
- **Endpoints**: Contact form submissions (when implemented)

#### Analytics Endpoints
- **Limit**: 1000 requests per minute per session
- **Tracking**: Session ID (via `X-Session-ID` header) or IP fallback

#### Admin Endpoints
- **Limit**: 30 requests per minute per IP
- **Endpoints**: All `/api/admin/*` routes

## Security Headers

All responses include the following security headers:

### Content-Security-Policy (CSP)
```
default-src 'self'; 
script-src 'self' 'unsafe-inline'; 
style-src 'self' 'unsafe-inline'; 
img-src 'self' data: https:; 
font-src 'self' data:; 
connect-src 'self'; 
frame-ancestors 'none';
```

### X-Frame-Options
```
DENY
```
Prevents clickjacking attacks by disallowing the page to be embedded in frames.

### X-Content-Type-Options
```
nosniff
```
Prevents MIME type sniffing.

### X-XSS-Protection
```
1; mode=block
```
Enables browser XSS filtering.

### Strict-Transport-Security (HSTS)
```
max-age=31536000; includeSubDomains
```
Forces HTTPS connections for one year.

### Referrer-Policy
```
strict-origin-when-cross-origin
```
Controls referrer information sent with requests.

### Permissions-Policy
```
geolocation=(), microphone=(), camera=()
```
Disables unnecessary browser features.

## Input Validation & Sanitization

### Request Validation
- Query string length limited to 2048 characters
- Path traversal detection (`..`, `//`)
- SQL injection pattern detection
- XSS pattern detection

### Input Sanitization
- Email normalization and validation
- String sanitization (control character removal)
- Numeric ID validation
- Maximum length enforcement

### Content-Type Validation
- POST/PUT/PATCH requests require valid Content-Type header
- Allowed types: `application/json`, `application/x-www-form-urlencoded`, `multipart/form-data`

### Request Size Validation
- Maximum request body size: 1MB
- Enforced via Content-Length header check

## SQL Injection Protection

### Parameterized Queries
All database queries use SQLx with parameterized statements, preventing SQL injection.

### Pattern Detection
Additional layer of protection detects common SQL injection patterns:
- `' or '1'='1`
- `'; drop table`
- `union select`
- `exec(`, `execute(`

## XSS Protection

### Output Encoding
All user-generated content is properly encoded before rendering.

### CSP Headers
Content Security Policy headers restrict script execution sources.

### Input Sanitization
Control characters and script tags are filtered from user input.

## CSRF Protection

### State-Changing Operations
All state-changing operations (POST, PUT, DELETE) require:
- Valid Content-Type header
- Request size validation
- Origin validation (via CORS)

### Admin Operations
Additional protection via:
- API key authentication
- IP whitelisting
- Request signing (optional)

## API Key Authentication

### Admin Endpoints
Protected endpoints require API key authentication via `X-API-Key` header.

### Configuration
```bash
export API_KEYS="key1,key2,key3"
```

### Usage
```bash
curl -H "X-API-Key: your-api-key" https://api.predictiq.com/api/admin/...
```

## IP Whitelisting

### Admin Endpoints
Admin operations restricted to whitelisted IP addresses.

### Configuration
```bash
export ADMIN_WHITELIST_IPS="192.168.1.1,10.0.0.1"
```

### IP Extraction
Supports multiple headers for proxy/load balancer scenarios:
- `X-Forwarded-For`
- `X-Real-IP`
- Direct connection IP

## Request Signing

### Sensitive Operations
Optional HMAC-SHA256 request signing for critical operations.

### Configuration
```bash
export REQUEST_SIGNING_SECRET="your-secret-key"
```

### Implementation
```rust
use crate::security::signing;

let signature = signing::generate_signature(payload, secret);
let valid = signing::verify_signature(payload, signature, secret);
```

## DDoS Protection

### Application Layer
- Rate limiting (multiple tiers)
- Request size limits
- Connection limits (via reverse proxy)

### Infrastructure Layer
Recommended external services:
- **Cloudflare**: DDoS protection, WAF, rate limiting
- **AWS Shield**: DDoS protection for AWS-hosted services
- **Nginx**: Connection limiting, request buffering

### Configuration Example (Nginx)
```nginx
limit_req_zone $binary_remote_addr zone=api:10m rate=100r/m;
limit_conn_zone $binary_remote_addr zone=addr:10m;

server {
    limit_req zone=api burst=20 nodelay;
    limit_conn addr 10;
    client_body_timeout 10s;
    client_header_timeout 10s;
}
```

## Disposable Email Detection

Newsletter subscriptions block common disposable email domains:
- mailinator.com
- tempmail.com
- guerrillamail.com

## Environment Variables

### Required
```bash
DATABASE_URL=postgres://user:pass@localhost/predictiq
REDIS_URL=redis://localhost:6379
```

### Security (Optional)
```bash
API_KEYS=key1,key2,key3
ADMIN_WHITELIST_IPS=192.168.1.1,10.0.0.1
REQUEST_SIGNING_SECRET=your-secret-key
```

## Monitoring & Logging

### Security Events
All security-related events are logged:
- Rate limit violations
- Authentication failures
- Invalid input attempts
- SQL injection attempts

### Metrics
Prometheus metrics track:
- Request rates by endpoint
- Rate limit hits
- Authentication failures
- Error rates

## Testing

### Rate Limiting
```bash
# Test global rate limit
for i in {1..150}; do curl http://localhost:8080/health; done

# Test newsletter rate limit
for i in {1..10}; do 
  curl -X POST http://localhost:8080/api/v1/newsletter/subscribe \
    -H "Content-Type: application/json" \
    -d '{"email":"test@example.com"}'; 
done
```

### Security Headers
```bash
curl -I http://localhost:8080/health
```

### Input Validation
```bash
# Test SQL injection detection
curl "http://localhost:8080/api/content?page=1' OR '1'='1"

# Test path traversal
curl "http://localhost:8080/api/../../../etc/passwd"
```

### API Key Authentication
```bash
# Without API key (should fail)
curl -X POST http://localhost:8080/api/markets/1/resolve

# With API key (should succeed)
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: your-api-key"
```

## Best Practices

1. **Always use HTTPS in production**
2. **Rotate API keys regularly**
3. **Monitor rate limit violations**
4. **Keep dependencies updated**
5. **Use strong secrets for signing**
6. **Enable all security headers**
7. **Implement proper logging**
8. **Regular security audits**
9. **Use infrastructure-level DDoS protection**
10. **Implement proper error handling (don't leak info)**

## Compliance

### GDPR
- Newsletter GDPR export endpoint
- Newsletter GDPR delete endpoint
- Data minimization
- Consent tracking

### Security Standards
- OWASP Top 10 protection
- CWE/SANS Top 25 mitigation
- Industry best practices

## Future Enhancements

- [ ] Implement CAPTCHA for high-risk endpoints
- [ ] Add geolocation-based rate limiting
- [ ] Implement account lockout after failed attempts
- [ ] Add honeypot fields for bot detection
- [ ] Implement advanced bot detection
- [ ] Add security audit logging
- [ ] Implement automated security scanning
- [ ] Add intrusion detection system (IDS)

## Support

For security issues, please contact: security@predictiq.com

**Do not disclose security vulnerabilities publicly.**
