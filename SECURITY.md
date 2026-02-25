# PredictIQ Security and Compliance Documentation

> **Issue**: [#86 - Create Security and Compliance Documentation](https://github.com/solutions-plug/predictIQ/issues/86)

This document describes the comprehensive security measures, compliance requirements, and best practices implemented in the PredictIQ project.

---

## Table of Contents

1. [Security Architecture Overview](#security-architecture-overview)
2. [Authentication and Authorization](#authentication-and-authorization)
3. [Data Encryption](#data-encryption)
4. [API Security Measures](#api-security-measures)
5. [Vulnerability Management](#vulnerability-management)
6. [Compliance Documentation](#compliance-documentation)
7. [Security Incident Response Plan](#security-incident-response-plan)
8. [Security Checklist for Developers](#security-checklist-for-developers)
9. [Third-Party Service Security Review](#third-party-service-security-review)
10. [Audit Logs and Monitoring](#audit-logs-and-monitoring)
11. [Backup and Disaster Recovery](#backup-and-disaster-recovery)

---

## Security Architecture Overview

### System Components

```
┌─────────────────────────────────────────────────────────────────┐
│                        PredictIQ Platform                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │   Frontend   │    │    API       │    │  Blockchain  │       │
│  │   (Web App)  │◄──►│   Service    │◄──►│  (Soroban)   │       │
│  └──────────────┘    └──────────────┘    └──────────────┘       │
│         │                   │                   │                │
│         ▼                   ▼                   ▼                │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐       │
│  │  CDN/WAF     │    │   Database   │    │   Oracles    │       │
│  │ (Cloudflare) │    │  (PostgreSQL)│    │  (Pyth)      │       │
│  └──────────────┘    └──────────────┘    └──────────────┘       │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### Security Layers

| Layer | Protection Mechanisms |
|-------|----------------------|
| **Perimeter** | WAF, DDoS protection, IP whitelisting, CORS |
| **Application** | Rate limiting, Input validation, CSRF protection, Security headers |
| **Data** | Encryption at rest (AES-256), Encryption in transit (TLS 1.3) |
| **Infrastructure** | VPC isolation, Private subnets, IAM roles |
| **Monitoring** | Real-time alerts, Audit logging, Anomaly detection |

### Trust Boundary Model

```
┌────────────────────────┐     ┌────────────────────────┐
│    Untrusted Zone     │     │    Trusted Zone        │
│                        │     │                        │
│  - Public Internet    │     │  - VPC Private Network │
│  - User Browsers      │◄───►│  - Internal Services   │
│  - External APIs      │     │  - Database            │
│                        │     │  - Admin Console       │
└────────────────────────┘     └────────────────────────┘
```

---

## Authentication and Authorization

### API Authentication

#### API Key Authentication

All admin endpoints require API key authentication via the `X-API-Key` header:

```bash
# Example request
curl -X POST https://api.predictiq.com/api/admin/markets/resolve \
  -H "X-API-Key: your-api-key" \
  -H "Content-Type: application/json" \
  -d '{"market_id": "123", "outcome": "1"}'
```

**Configuration:**
```bash
# Environment variable (comma-separated for multiple keys)
export API_KEYS="key1,key2,key3"

# Rotate keys regularly
export API_KEYS="newkey1,key2,key3"  # Remove compromised keys immediately
```

#### Request Signing (HMAC-SHA256)

Optional HMAC-SHA256 signing for sensitive operations:

```rust
use crate::security::signing;

// Generate signature
let signature = signing::generate_signature(payload, secret);

// Verify signature
let valid = signing::verify_signature(payload, signature, secret);
```

### Blockchain Authentication

The smart contract uses Soroban authentication:

- **Admin functions**: Require guardian multisig (3-of-5 threshold)
- **Market creation**: Requires staking deposit
- **Betting**: Requires valid token balance
- **Resolution**: Requires oracle signature or guardian vote

### Role-Based Access Control (RBAC)

| Role | Permissions |
|------|-------------|
| `user` | Place bets, view markets, claim winnings |
| `creator` | Create markets, manage own markets |
| `guardian` | Vote on disputes, pause/resume markets |
| `admin` | Full system configuration, emergency controls |

---

## Data Encryption

### Encryption in Transit

All data in transit is encrypted using TLS 1.3:

| Protocol | Status | Notes |
|----------|--------|-------|
| TLS 1.3 | ✅ Required | Minimum supported version |
| TLS 1.2 | ⚠️ Deprecated | Only for legacy compatibility |
| HTTP | ❌ Disabled | Redirects to HTTPS |

**Configuration:**
```nginx
# HSTS Header (Strict-Transport-Security)
add_header Strict-Transport-Security "max-age=31536000; includeSubDomains; preload" always;
```

### Encryption at Rest

| Data Type | Encryption | Algorithm |
|-----------|------------|-----------|
| Database | ✅ Enabled | AES-256-GCM |
| Backups | ✅ Enabled | AES-256-GCM |
| Environment Variables | ✅ Encrypted | Via secrets manager |
| Log Files | ✅ Enabled | AES-256 (optional) |

**PostgreSQL Configuration:**
```sql
-- Enable transparent data encryption (TDE)
ALTER SYSTEM SET ssl = on;
ALTER SYSTEM SET ssl_cert_file = '/path/to/server.crt';
ALTER SYSTEM SET ssl_key_file = '/path/to/server.key';
```

**Application Secrets:**
```bash
# Use secrets manager (AWS Secrets Manager, HashiCorp Vault, etc.)
export DATABASE_URL="postgres://user:@host/db"  # Password from secrets manager
export REDIS_PASSWORD="from-secrets-manager"
```

### Key Management

| Aspect | Implementation |
|--------|----------------|
| Key Rotation | 90-day rotation cycle |
| Key Storage | Hardware Security Module (HSM) or cloud KMS |
| Key Backup | Encrypted backups stored in separate geographic region |
| Key Revocation | Immediate revocation process for compromised keys |

---

## API Security Measures

### Rate Limiting

#### Global Rate Limiting
- **Limit**: 100 requests per minute per IP
- **Algorithm**: Sliding window
- **Scope**: All API endpoints

#### Endpoint-Specific Limits

| Endpoint Category | Limit | Window |
|------------------|-------|--------|
| Newsletter subscribe | 5 requests | Hourly |
| Contact form | 3 requests | Hourly |
| Admin endpoints | 30 requests | Per minute |
| Analytics | 1000 requests | Per minute |

#### Rate Limit Response Headers
```
X-RateLimit-Limit: 100
X-RateLimit-Remaining: 95
X-RateLimit-Reset: 1640000000
```

#### DDoS Protection (Infrastructure Layer)

Recommended external services:
- **Cloudflare**: DDoS protection, WAF, rate limiting
- **AWS Shield**: DDoS protection for AWS-hosted services
- **Nginx**: Connection limiting, request buffering

```nginx
# Nginx rate limiting configuration
limit_req_zone $binary_remote_addr zone=api:10m rate=100r/m;
limit_conn_zone $binary_remote_addr zone=addr:10m;

server {
    limit_req zone=api burst=20 nodelay;
    limit_conn addr 10;
}
```

### Input Validation

| Validation Type | Implementation |
|-----------------|----------------|
| Query string length | Max 2048 characters |
| Path traversal | Detection of `..`, `//` |
| SQL injection | Pattern detection + parameterized queries |
| XSS | Pattern detection + output encoding |
| Email | RFC 5322 validation + disposable domain blocking |
| Content-Type | Required for POST/PUT/PATCH |
| Request size | Max 1MB body size |

**Validation Code:**
```rust
// From services/api/src/validation.rs
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    // RFC 5322 validation
    // Disposable domain blocking
}

pub fn sanitize_input(input: &str) -> String {
    // Remove control characters
    // Escape HTML entities
}
```

### CORS Configuration

```rust
// CORS configuration in main.rs
Cors::new()
    .allowed_origins(["https://predictiq.com", "https://www.predictiq.com"])
    .allowed_methods(["GET", "POST", "PUT", "DELETE"])
    .allowed_headers(["Content-Type", "Authorization", "X-API-Key"])
    .expose_headers(["X-RateLimit-Limit", "X-RateLimit-Remaining"])
    .max_age(3600)
    .allow_credentials(true)
```

| Setting | Value |
|---------|-------|
| Allowed Origins | `predictiq.com` (production) |
| Allowed Methods | GET, POST, PUT, DELETE |
| Allowed Headers | Content-Type, Authorization, X-API-Key |
| Max Age | 1 hour |
| Credentials | Supported |

### Security Headers

All responses include the following security headers:

```http
Content-Security-Policy: default-src 'self'; script-src 'self' 'unsafe-inline'; ...
X-Frame-Options: DENY
X-Content-Type-Options: nosniff
X-XSS-Protection: 1; mode=block
Strict-Transport-Security: max-age=31536000; includeSubDomains
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: geolocation=(), microphone=(), camera=()
```

| Header | Purpose |
|--------|---------|
| `Content-Security-Policy` | Prevents XSS and injection attacks |
| `X-Frame-Options` | Prevents clickjacking |
| `X-Content-Type-Options` | Prevents MIME sniffing |
| `X-XSS-Protection` | XSS filter (legacy browsers) |
| `Strict-Transport-Security` | Forces HTTPS |
| `Referrer-Policy` | Controls referrer information |
| `Permissions-Policy` | Disables unnecessary browser features |

---

## Vulnerability Management

### Dependency Scanning

#### Automated Scanning

| Tool | Purpose | Frequency |
|------|---------|-----------|
| `cargo audit` | Rust dependency vulnerabilities | Every build |
| GitHub Dependabot | Dependency updates | Daily |
| Snyk | Full SCA | Weekly |
| Trivy | Container scanning | Every deployment |

```bash
# Run cargo audit
cargo audit

# GitHub Actions workflow
name: Security Scan
on: [push, pull_request]
jobs:
  security:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run cargo audit
        run: cargo audit
```

#### Vulnerability Categories

| Severity | Response Time |
|----------|---------------|
| Critical (CVSS 9-10) | 24 hours |
| High (CVSS 7-8.9) | 7 days |
| Medium (CVSS 4-6.9) | 30 days |
| Low (CVSS < 4) | Next release |

### Security Updates Process

#### Patch Management

1. **Monitoring**: Subscribe to security mailing lists (RustSec, CVE)
2. **Assessment**: Evaluate CVSS score and impact
3. **Testing**: Test patches in staging environment
4. **Deployment**: Deploy to production with rollback plan
5. **Verification**: Confirm fix and monitor for issues

```bash
# Update dependencies
cargo update

# Review changes
cargo outdated

# Check for security advisories
cargo audit
```

### Penetration Testing

| Type | Frequency | Provider |
|------|-----------|----------|
| Automated scanning | Weekly | Internal + Snyk |
| Manual penetration testing | Quarterly | Third-party |
| Code review | Per PR | Maintainers |
| Architecture review | Annually | Security team |

**Scope of Pen Tests:**
- API endpoints (OWASP API Security Top 10)
- Authentication mechanisms
- Smart contract audit
- Infrastructure configuration

---

## Compliance Documentation

### GDPR Compliance

#### Data Processing Principles

| Principle | Implementation |
|-----------|----------------|
| Lawfulness | Explicit consent for all processing |
| Purpose Limitation | Data used only for stated purposes |
| Data Minimization | Only necessary data collected |
| Accuracy | User-correctable data |
| Storage Limitation | Automatic deletion after retention period |
| Integrity & Confidentiality | Encryption and access controls |

#### User Rights

| Right | Endpoint |
|-------|----------|
| Access | `GET /api/v1/newsletter/gdpr/export` |
| Rectification | Via account settings |
| Erasure | `DELETE /api/v1/newsletter/gdpr/delete` |
| Data Portability | JSON export available |
| Withdraw Consent | Unsubscribe link in all emails |

#### Data Retention

| Data Type | Retention Period | Legal Basis |
|-----------|------------------|-------------|
| Account data | Until deletion request | Contract fulfillment |
| Betting history | 7 years | Legal obligation |
| Marketing consent | Until withdrawn | Consent |
| Server logs | 90 days | Legitimate interest |
| Analytics | 2 years | Legitimate interest |

#### DPO Contact

> **Data Protection Officer**: dpo@predictiq.com

---

### Cookie Policy

#### Types of Cookies Used

| Cookie Type | Purpose | Duration |
|-------------|---------|----------|
| `session_id` | Authentication | 24 hours |
| `csrf_token` | CSRF protection | Session |
| `preferences` | User settings | 1 year |
| `_ga` | Analytics | 2 years |
| `_gid` | Analytics | 24 hours |

#### Cookie Management

Users can manage cookies via:
- Browser settings
- Cookie consent banner on first visit
- `DELETE /api/v1/user/cookies` endpoint

```javascript
// Cookie consent implementation
const COOKIE_POLICY = {
  essential: ['session_id', 'csrf_token'],
  analytics: ['_ga', '_gid'],
  marketing: []
};
```

---

### Privacy Policy

#### Information We Collect

| Category | Data Points |
|----------|--------------|
| Account | Email, username, wallet address |
| Usage | Page views, interactions, device info |
| Financial | Betting history, transaction records |
| Technical | IP address, browser, device |

#### How We Use Data

- Provide and improve services
- Process transactions
- Comply with legal obligations
- Communicate with users
- Prevent fraud and abuse

#### Data Sharing

| Recipient | Purpose | Legal Basis |
|-----------|---------|-------------|
| Blockchain | Transaction processing | Contract fulfillment |
| Payment providers | Financial transactions | Contract fulfillment |
| Oracle services | Market resolution | Contract fulfillment |
| Legal authorities | Compliance | Legal obligation |

---

### Terms of Service

#### Acceptance

By using PredictIQ, you agree to:
- Be at least 18 years of age
- Comply with all applicable laws
- Not engage in fraud or manipulation
- Accept market outcomes

#### User Responsibilities

- Maintain secure account credentials
- Report security issues promptly
- Not exploit vulnerabilities
- Not interfere with other users

#### Limitation of Liability

PredictIQ is not liable for:
- Market outcomes
- Financial losses from betting
- Third-party service failures
- Force majeure events

---

## Security Incident Response Plan

### Incident Classification

| Severity | Definition | Examples | Response Time |
|----------|------------|----------|---------------|
| **P1 - Critical** | Data breach, funds at risk | Database breach, smart contract exploit | 1 hour |
| **P2 - High** | Service disruption | DDoS attack, malware | 4 hours |
| **P3 - Medium** | Security policy violation | Unauthorized access attempt | 24 hours |
| **P4 - Low** | Minor security issue | Failed login attempts | 7 days |

### Incident Response Team (IRT)

| Role | Name | Contact |
|------|------|---------|
| Lead | Security Lead | security@predictiq.com |
| Technical | DevOps Lead | ops@predictiq.com |
| Legal | General Counsel | legal@predictiq.com |
| Communications | PR Manager | pr@predictiq.com |

### Response Procedure

```
┌──────────────────────────────────────────────────────────────────┐
│                     Incident Response Flow                        │
├──────────────────────────────────────────────────────────────────┤
│                                                                   │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐      │
│  │  Detect │───►│ Triage  │───►│ Contain │───►│ Eradicate│     │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘      │
│       │                                            │              │
│       ▼                                            ▼              │
│  ┌─────────┐                               ┌─────────┐         │
│  │  Alert  │                               │ Recover │         │
│  └─────────┘                               └─────────┘         │
│       │                                            │              │
│       ▼                                            ▼              │
│  ┌─────────────────────────────────────────────────────┐        │
│  │              Post-Incident Review                    │        │
│  └─────────────────────────────────────────────────────┘        │
│                                                                   │
└──────────────────────────────────────────────────────────────────┘
```

#### Step 1: Detection
- Automated alerts from monitoring systems
- User reports via security@predictiq.com
- Third-party vulnerability reports

#### Step 2: Triage
- Confirm incident validity
- Classify severity
- Assign incident owner

#### Step 3: Containment
- Isolate affected systems
- Preserve evidence
- Prevent lateral movement

#### Step 4: Eradication
- Remove threat
- Patch vulnerabilities
- Verify clean state

#### Step 5: Recovery
- Restore services
- Verify functionality
- Monitor for recurrence

#### Step 6: Post-Incident Review
- Document timeline
- Identify root cause
- Implement improvements
- Update documentation

### Communication Plan

| Audience | Method | Timing |
|----------|--------|--------|
| Internal team | Slack #security-incidents | Immediate |
| Affected users | Email + Website notice | Within 72 hours |
| Regulatory bodies | Direct contact | As required |
| Public | Blog post (if significant) | After remediation |

### Bug Bounty Program

| Severity | Bounty |
|----------|--------|
| Critical | $5,000 - $50,000 |
| High | $1,000 - $5,000 |
| Medium | $250 - $1,000 |
| Low | $50 - $250 |

**Scope:**
- predictiq.com
- API endpoints
- Smart contracts

**Contact:** bugbounty@predictiq.com

---

## Security Checklist for Developers

### Pre-Development

- [ ] Complete security awareness training
- [ ] Review threat model for feature
- [ ] Identify sensitive data handled
- [ ] Define authentication requirements
- [ ] Plan input validation strategy

### During Development

#### Authentication
- [ ] Use established authentication libraries
- [ ] Never hardcode credentials
- [ ] Implement proper session management
- [ ] Use secure password hashing (Argon2id)
- [ ] Implement MFA for admin accounts

#### Input Validation
- [ ] Validate all inputs on server-side
- [ ] Use parameterized queries
- [ ] Sanitize outputs
- [ ] Implement rate limiting

#### Data Protection
- [ ] Encrypt sensitive data at rest
- [ ] Use TLS for data in transit
- [ ] Never log sensitive data
- [ ] Implement proper key management

#### Code Quality
- [ ] Use `cargo clippy` for linting
- [ ] Run `cargo audit` for vulnerabilities
- [ ] Write unit tests for security controls
- [ ] Review dependencies for known vulnerabilities

### Pre-Deployment

- [ ] Security code review completed
- [ ] Penetration testing (if high-risk)
- [ ] Secrets rotated
- [ ] Logging and monitoring enabled
- [ ] Backup verified
- [ ] Rollback plan documented

### Post-Deployment

- [ ] Monitor error logs
- [ ] Watch for anomalies
- [ ] Verify security headers
- [ ] Test rate limiting
- [ ] Confirm backup restoration works

### Secure Coding Guidelines

```rust
// ✅ GOOD: Parameterized query
let result = sqlx::query("SELECT * FROM users WHERE id = ?")
    .bind(user_id)
    .fetch_one(&pool)
    .await?;

// ❌ BAD: String concatenation (SQL injection)
let query = format!("SELECT * FROM users WHERE id = {}", user_id);
```

```rust
// ✅ GOOD: Constant-time comparison
use subtle::ConstantTimeEq;
if secret.ct_eq(&input).unwrap_u8() == 1 {
    // authenticated
}

// ❌ BAD: Timing attack vulnerable
if secret == input {
    // vulnerable to timing attack
}
```

---

## Third-Party Service Security Review

### Service Risk Matrix

| Service | Purpose | Risk Level | Data Sensitivity |
|---------|---------|------------|-------------------|
| PostgreSQL | Database | Low | High |
| Redis | Caching | Low | Medium |
| Cloudflare | CDN/WAF | Low | None |
| AWS | Infrastructure | Low | Low |
| Pyth Network | Oracles | Medium | Low |
| SendGrid | Email | Medium | Low |

### Review Criteria

| Criteria | Weight | Assessment Method |
|----------|--------|-------------------|
| Security certifications | 20% | SOC2, ISO 27001 |
| Data handling | 20% | DPA review |
| Encryption practices | 20% | Documentation |
| Access controls | 15% | Audit reports |
| Incident history | 15% | Public records |
| Subprocessor usage | 10% | Subprocessor list |

### Data Processing Agreements (DPA)

All third parties with data access have signed DPAs covering:
- Data processing scope
- Security requirements
- Breach notification
- Data deletion
- Audit rights

### Vendor Assessment Checklist

- [ ] SOC 2 Type II report reviewed
- [ ] Data Processing Agreement signed
- [ ] Security questionnaire completed
- [ ] Penetration test results reviewed
- [ ] Subprocessor list obtained
- [ ] Incident response plan reviewed

---

## Audit Logs and Monitoring

### Audit Log Events

| Category | Events Logged |
|----------|---------------|
| Authentication | Login, logout, failed attempts, password changes |
| Authorization | Access denied, privilege escalation |
| Data | Create, read, update, delete operations |
| Security | Rate limit violations, input validation failures |
| Admin | Configuration changes, deployments |
| Financial | Deposits, withdrawals, bet placement, resolution |

### Log Format

```json
{
  "timestamp": "2024-01-15T10:30:00Z",
  "level": "warn",
  "event": "rate_limit_exceeded",
  "user_id": "user_123",
  "ip": "192.168.1.1",
  "endpoint": "/api/v1/newsletter/subscribe",
  "requests_last_minute": 6,
  "limit": 5
}
```

### Log Retention

| Log Type | Retention | Storage |
|----------|-----------|---------|
| Access logs | 90 days | Hot storage |
| Security logs | 1 year | Warm storage |
| Audit logs | 7 years | Cold storage |
| Financial logs | 7 years | Cold storage |

### Monitoring Alerts

| Alert | Condition | Severity |
|-------|-----------|----------|
| Failed logins | >10/minute | High |
| Rate limit hits | >100/minute | Medium |
| Error rate | >5% | High |
| Latency | >500ms p99 | Medium |
| CPU usage | >80% | Medium |
| Memory usage | >90% | Critical |

### Tools

| Purpose | Tool |
|---------|------|
| Log aggregation | AWS CloudWatch / ELK |
| Metrics | Prometheus + Grafana |
| Tracing | Jaeger |
| Alerting | PagerDuty |
| SIEM | AWS GuardDuty |

---

## Backup and Disaster Recovery

### Backup Strategy

#### Database Backups

| Type | Frequency | Retention | Location |
|------|-----------|-----------|----------|
| Full | Daily | 30 days | Off-site |
| Incremental | Hourly | 7 days | Off-site |
| WAL archiving | Continuous | 7 days | Off-site |

```bash
# PostgreSQL backup script
pg_dump -Fc -f backup.dump predictiq
aws s3 cp backup.dump s3://predictiq-backups/
```

#### Application Backups

| Component | Frequency | Method |
|-----------|-----------|--------|
| Code | Every commit | Git |
| Config | On change | Version control |
| Secrets | Rotation event | Secrets manager |
| Media | Daily | Object storage |

### Disaster Recovery

#### Recovery Objectives

| Metric | Target |
|--------|--------|
| RTO (Recovery Time Objective) | 4 hours |
| RPO (Recovery Point Objective) | 1 hour |

#### Recovery Procedures

1. **Database Failure**
   - Promote read replica to primary
   - Update DNS
   - Verify functionality
   - Duration: ~30 minutes

2. **Application Failure**
   - Deploy from last healthy state
   - Restore config from secrets manager
   - Verify health checks
   - Duration: ~1 hour

3. **Full Region Failure**
   - Deploy to backup region
   - Restore from cross-region backup
   - Update DNS
   - Duration: ~2-4 hours

#### DR Testing

| Test | Frequency |
|------|-----------|
| Backup restoration | Monthly |
| Failover drill | Quarterly |
| Full DR simulation | Annually |

---

## Vulnerability Disclosure Process

### Reporting

Submit vulnerabilities securely:

1. **Email**: security@predictiq.com
2. **Encrypted**: PGP key available on website
3. **Bug Bounty**: For eligible vulnerabilities

### What to Include

- Description of vulnerability
- Steps to reproduce
- Potential impact
- Suggested remediation (optional)

### Response Timeline

| Phase | Timeline |
|-------|----------|
| Initial acknowledgment | 24 hours |
| Severity assessment | 7 days |
| Fix timeline | Per severity table |
| Public disclosure | After fix + 90 days |

---

## Contact

| Purpose | Contact |
|---------|---------|
| Security issues | security@predictiq.com |
| Bug bounty | bugbounty@predictiq.com |
| Privacy concerns | privacy@predictiq.com |
| Data Protection Officer | dpo@predictiq.com |
| General inquiries | support@predictiq.com |

**PGP Key**: Available at https://predictiq.com/security/pgp.txt

---

*Last Updated: February 2024*
*Document Owner: Security Team*
*Next Review: August 2024*
