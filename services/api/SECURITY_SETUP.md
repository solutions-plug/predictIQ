# Security Setup Guide

## Quick Start

### 1. Generate Security Keys

```bash
# Generate API keys
openssl rand -hex 32

# Generate request signing secret
openssl rand -base64 64
```

### 2. Configure Environment Variables

Copy `.env.example` to `.env` and update the security settings:

```bash
cp .env.example .env
```

Edit `.env`:
```bash
# Add your generated keys
API_KEYS=your-generated-api-key-1,your-generated-api-key-2
REQUEST_SIGNING_SECRET=your-generated-signing-secret

# Add whitelisted IPs for admin access
ADMIN_WHITELIST_IPS=127.0.0.1,your-office-ip
```

### 3. Build and Run

```bash
cargo build --release
cargo run --release
```

## Testing Security Features

### Test Rate Limiting

```bash
# Test global rate limit (100 req/min)
./test_rate_limit.sh

# Test newsletter rate limit (5 req/hour)
for i in {1..10}; do
  curl -X POST http://localhost:8080/api/v1/newsletter/subscribe \
    -H "Content-Type: application/json" \
    -d '{"email":"test'$i'@example.com"}'
  echo ""
done
```

### Test Security Headers

```bash
curl -I http://localhost:8080/health | grep -E "(content-security-policy|x-frame-options|x-content-type-options)"
```

### Test Input Validation

```bash
# SQL injection attempt (should be blocked)
curl "http://localhost:8080/api/content?page=1' OR '1'='1"

# Path traversal attempt (should be blocked)
curl "http://localhost:8080/api/../../../etc/passwd"

# XSS attempt (should be blocked)
curl "http://localhost:8080/api/content?search=<script>alert('xss')</script>"
```

### Test API Key Authentication

```bash
# Without API key (should return 401)
curl -X POST http://localhost:8080/api/markets/1/resolve

# With valid API key (should succeed)
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: your-api-key"

# With invalid API key (should return 401)
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: invalid-key"
```

### Test IP Whitelisting

```bash
# From whitelisted IP (should succeed)
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: your-api-key" \
  -H "X-Real-IP: 127.0.0.1"

# From non-whitelisted IP (should return 403)
curl -X POST http://localhost:8080/api/markets/1/resolve \
  -H "X-API-Key: your-api-key" \
  -H "X-Real-IP: 1.2.3.4"
```

## Production Deployment

### 1. Infrastructure-Level Protection

#### Cloudflare Setup
1. Add your domain to Cloudflare
2. Enable DDoS protection
3. Configure WAF rules
4. Set up rate limiting rules
5. Enable Bot Fight Mode

#### AWS Shield (if using AWS)
1. Enable AWS Shield Standard (free)
2. Consider AWS Shield Advanced for critical applications
3. Configure AWS WAF rules
4. Set up CloudWatch alarms

#### Nginx Configuration
```nginx
# /etc/nginx/nginx.conf

http {
    # Rate limiting zones
    limit_req_zone $binary_remote_addr zone=api:10m rate=100r/m;
    limit_req_zone $binary_remote_addr zone=newsletter:10m rate=5r/h;
    limit_conn_zone $binary_remote_addr zone=addr:10m;

    # Request size limits
    client_max_body_size 1m;
    client_body_timeout 10s;
    client_header_timeout 10s;

    server {
        listen 443 ssl http2;
        server_name api.predictiq.com;

        # SSL configuration
        ssl_certificate /path/to/cert.pem;
        ssl_certificate_key /path/to/key.pem;
        ssl_protocols TLSv1.2 TLSv1.3;
        ssl_ciphers HIGH:!aNULL:!MD5;

        # Security headers (backup layer)
        add_header X-Frame-Options "DENY" always;
        add_header X-Content-Type-Options "nosniff" always;
        add_header X-XSS-Protection "1; mode=block" always;

        # Rate limiting
        limit_req zone=api burst=20 nodelay;
        limit_conn addr 10;

        location /api/v1/newsletter/ {
            limit_req zone=newsletter burst=2 nodelay;
            proxy_pass http://localhost:8080;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        }

        location / {
            proxy_pass http://localhost:8080;
            proxy_set_header X-Real-IP $remote_addr;
            proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
            proxy_set_header Host $host;
        }
    }
}
```

### 2. Environment Configuration

Production `.env`:
```bash
# Use strong, unique keys
API_KEYS=prod-key-1,prod-key-2
REQUEST_SIGNING_SECRET=prod-signing-secret

# Whitelist only necessary IPs
ADMIN_WHITELIST_IPS=office-ip-1,office-ip-2,ci-cd-ip

# Enable production logging
RUST_LOG=warn,predictiq_api=info

# Use production database
DATABASE_URL=postgres://user:pass@prod-db:5432/predictiq

# Use production Redis
REDIS_URL=redis://prod-redis:6379
```

### 3. Monitoring Setup

#### Prometheus Metrics
```yaml
# prometheus.yml
scrape_configs:
  - job_name: 'predictiq-api'
    static_configs:
      - targets: ['localhost:8080']
    metrics_path: '/metrics'
```

#### Grafana Dashboard
Import the provided dashboard or create alerts for:
- Rate limit violations
- Authentication failures
- Error rates
- Response times

#### Log Monitoring
```bash
# Set up log aggregation (e.g., ELK stack, Datadog)
# Monitor for:
# - SQL injection attempts
# - XSS attempts
# - Rate limit violations
# - Authentication failures
```

### 4. Security Checklist

- [ ] Generate strong API keys
- [ ] Configure IP whitelist
- [ ] Set up HTTPS/TLS
- [ ] Enable Cloudflare/WAF
- [ ] Configure Nginx rate limiting
- [ ] Set up monitoring and alerts
- [ ] Enable log aggregation
- [ ] Test all security features
- [ ] Document incident response plan
- [ ] Schedule regular security audits
- [ ] Keep dependencies updated
- [ ] Implement backup and recovery
- [ ] Set up security scanning (e.g., Dependabot)

## Maintenance

### Regular Tasks

#### Weekly
- Review rate limit violations
- Check authentication failure logs
- Monitor error rates

#### Monthly
- Rotate API keys
- Review IP whitelist
- Update dependencies
- Security audit

#### Quarterly
- Penetration testing
- Security training
- Incident response drill
- Disaster recovery test

### Key Rotation

```bash
# Generate new keys
NEW_API_KEY=$(openssl rand -hex 32)
NEW_SIGNING_SECRET=$(openssl rand -base64 64)

# Update environment
# Add new keys alongside old ones
API_KEYS=old-key-1,old-key-2,new-key-1

# Update clients to use new keys
# After migration, remove old keys
API_KEYS=new-key-1,new-key-2

# Restart service
systemctl restart predictiq-api
```

## Incident Response

### Security Incident Detected

1. **Immediate Actions**
   - Enable emergency rate limiting
   - Block suspicious IPs
   - Rotate compromised keys
   - Enable additional logging

2. **Investigation**
   - Review logs
   - Identify attack vector
   - Assess damage
   - Document findings

3. **Remediation**
   - Patch vulnerabilities
   - Update security rules
   - Notify affected users
   - Implement additional controls

4. **Post-Incident**
   - Conduct post-mortem
   - Update procedures
   - Improve monitoring
   - Train team

### Emergency Contacts
- Security Team: security@predictiq.com
- On-Call Engineer: oncall@predictiq.com
- Management: management@predictiq.com

## Additional Resources

- [OWASP Top 10](https://owasp.org/www-project-top-ten/)
- [CWE/SANS Top 25](https://cwe.mitre.org/top25/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [Axum Security Best Practices](https://docs.rs/axum/latest/axum/)

## Support

For security questions or to report vulnerabilities:
- Email: security@predictiq.com
- PGP Key: [link to public key]

**Please do not disclose security vulnerabilities publicly.**
