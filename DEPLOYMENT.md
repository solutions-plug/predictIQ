# PredictIQ Deployment and Operations Documentation

> **Issue**: [#88 - Create Deployment and Operations Documentation](https://github.com/solutions-plug/predictIQ/issues/88)

This document provides comprehensive deployment procedures, infrastructure setup, and operational guidance for the PredictIQ platform.

---

## Table of Contents

1. [Infrastructure Overview](#infrastructure-overview)
2. [Deployment Documentation](#deployment-documentation)
3. [Monitoring and Alerting](#monitoring-and-alerting)
4. [Operational Runbooks](#operational-runbooks)
5. [Security Procedures](#security-procedures)
6. [Performance Optimization](#performance-optimization)
7. [Cost Optimization](#cost-optimization)

---

## Infrastructure Overview

### Architecture Diagram

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                            PredictIQ Infrastructure                          │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                           CDN / WAF Layer                             │   │
│  │                    (Cloudflare - DDoS Protection)                    │   │
│  └──────────────────────────────┬───────────────────────────────────────┘   │
│                                 │                                             │
│                                 ▼                                             │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                          Load Balancer                                │   │
│  │                    (AWS ALB / Cloudflare Load Balancer)              │   │
│  └──────────────────────────────┬───────────────────────────────────────┘   │
│                                 │                                             │
│         ┌────────────────────────┼────────────────────────┐                  │
│         ▼                        ▼                        ▼                  │
│  ┌─────────────┐         ┌─────────────┐         ┌─────────────┐           │
│  │  API Pod 1  │         │  API Pod 2   │         │  API Pod N   │           │
│  │  (Rust/Axum)│         │  (Rust/Axum) │         │  (Rust/Axum)│           │
│  └──────┬──────┘         └──────┬──────┘         └──────┬──────┘           │
│         │                        │                        │                    │
│         └────────────────────────┼────────────────────────┘                  │
│                                  ▼                                            │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                        Internal Services                              │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐│   │
│  │  │  PostgreSQL │  │    Redis     │  │   RabbitMQ  │  │  Prometheus ││   │
│  │  │  (Primary)  │  │   (Cache)    │  │    (Queue)  │  │  (Metrics)  ││   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘│   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
│  ┌──────────────────────────────────────────────────────────────────────┐   │
│  │                       External Integrations                          │   │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐│   │
│  │  │   Stellar   │  │    Soroban   │  │    Pyth     │  │  SendGrid   ││   │
│  │  │  Network    │  │   Contracts  │  │   Oracle    │  │   (Email)   ││   │
│  │  └─────────────┘  └─────────────┘  └─────────────┘  └─────────────┘│   │
│  └──────────────────────────────────────────────────────────────────────┘   │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Service Dependencies

| Service | Type | Purpose | Dependencies |
|---------|------|---------|--------------|
| **API Service** | Core | HTTP API handling | PostgreSQL, Redis, Stellar |
| **PostgreSQL** | Database | Primary data store | None |
| **Redis** | Cache | Session cache, rate limiting | None |
| **RabbitMQ** | Message Queue | Async job processing | None |
| **Prometheus** | Metrics | Metrics collection | None |
| **Grafana** | Visualization | Dashboard | Prometheus |
| **Cloudflare** | CDN/WAF | DDoS, SSL, CDN | None |
| **Stellar/Soroban** | Blockchain | Smart contract execution | None |
| **Pyth Oracle** | Oracle | Price feeds | Soroban |
| **SendGrid** | Email | Email delivery | None |

### Network Topology

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                              Network Segments                                │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                           Public Subnet                                │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                   │ │
│  │  │   Cloudflare│  │ Load Balancer│  │   NAT GW   │                   │ │
│  │  │   (WAF/CDN) │  │    (ALB)     │  │            │                   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                   │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                      │                                       │
│                                      ▼                                       │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                          Private Subnet                                 │ │
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐                   │ │
│  │  │  API Server │  │  API Server │  │  API Server │                   │ │
│  │  │   (ECS/EKS) │  │   (ECS/EKS)  │  │   (ECS/EKS)  │                   │ │
│  │  └─────────────┘  └─────────────┘  └─────────────┘                   │ │
│  │                                                                    │ │
│  │  ┌─────────────┐  ┌─────────────┐                                   │ │
│  │  │  PostgreSQL │  │    Redis    │                                   │ │
│  │  │   (RDS)     │  │  (ElastiCache)│                                  │ │
│  │  └─────────────┘  └─────────────┘                                   │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                      │                                       │
│                                      ▼                                       │
│  ┌────────────────────────────────────────────────────────────────────────┐ │
│  │                          Database Subnet                               │ │
│  │  ┌─────────────┐  ┌─────────────┐                                    │ │
│  │  │  PostgreSQL │  │   Backup    │                                    │ │
│  │  │  (Replicas) │  │   Storage    │                                    │ │
│  │  └─────────────┘  └─────────────┘                                    │ │
│  └────────────────────────────────────────────────────────────────────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## Deployment Documentation

### CI/CD Pipeline Explanation

#### Pipeline Stages

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                           CI/CD Pipeline Flow                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐ │
│  │  Code   │───►│  Build  │───►│  Test   │───►│ Staging │───►│Production│ │
│  │  Push   │    │         │    │         │    │ Deploy  │    │  Deploy  │ │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘    └─────────┘ │
│       │              │              │              │              │        │
│       ▼              ▼              ▼              ▼              ▼        │
│  ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐    ┌─────────┐ │
│  │ GitHub  │    │  Cargo  │    │  Cargo  │    │  Docker │    │  Blue/  │ │
│  │ Actions │    │  Build  │    │  Test   │    │  Build  │    │  Green   │ │
│  └─────────┘    └─────────┘    └─────────┘    └─────────┘    └─────────┘ │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

#### GitHub Actions Workflow

```yaml
# .github/workflows/deploy.yml
name: Deploy

on:
  push:
    branches: [main]
  pull_request:
    branches: [main]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Run tests
        run: cargo test --all
      - name: Run clippy
        run: cargo clippy --all
      - name: Security audit
        run: cargo audit

  build:
    needs: test
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Build release
        run: cargo build --release
      - name: Build Docker image
        run: docker build -t predictiq-api:${{ github.sha }} .

  deploy-staging:
    needs: build
    runs-on: ubuntu-latest
    environment: staging
    steps:
      - name: Deploy to staging
        run: |
          kubectl set image deployment/api \
            api=predictiq-api:${{ github.sha }}

  deploy-production:
    needs: deploy-staging
    runs-on: ubuntu-latest
    environment: production
    steps:
      - name: Deploy to production
        run: |
          kubectl rollout status deployment/api
          kubectl set image deployment/api \
            api=predictiq-api:${{ github.sha }}
```

### Deployment Steps

#### Prerequisites

| Requirement | Version | Purpose |
|-------------|---------|---------|
| Rust | 1.75+ | API compilation |
| Docker | 24+ | Containerization |
| Kubernetes | 1.28+ | Orchestration |
| PostgreSQL | 15+ | Database |
| Redis | 7+ | Caching |

#### Step 1: Clone and Configure

```bash
# Clone repository
git clone https://github.com/solutions-plug/predictIQ.git
cd predictIQ

# Copy environment template
cp services/api/.env.example services/api/.env

# Edit configuration
nano services/api/.env
```

#### Step 2: Database Setup

```bash
# Run migrations
cd services/api
cargo run --release -- migrate

# Or run manually
psql $DATABASE_URL -f database/migrations/001_enable_pgcrypto.sql
psql $DATABASE_URL -f database/migrations/002_create_newsletter_subscriptions.sql
# ... etc
```

#### Step 3: Build Application

```bash
# Build release binary
cargo build --release

# Build Docker image
docker build -t predictiq-api:latest .
```

#### Step 4: Deploy to Kubernetes

```bash
# Apply Kubernetes manifests
kubectl apply -f k8s/

# Verify deployment
kubectl get pods -l app=predictiq-api
kubectl logs -l app=predictiq-api

# Check health
curl http://api.predictiq.com/health
```

### Rollback Procedures

#### Quick Rollback (Kubernetes)

```bash
# Rollback to previous revision
kubectl rollout undo deployment/predictiq-api

# Rollback to specific revision
kubectl rollout undo deployment/predictiq-api --to-revision=3

# Check rollback status
kubectl rollout status deployment/predictiq-api
```

#### Database Rollback

```bash
# If migration needs rollback
psql $DATABASE_URL -f database/migrations/downgrades/XXX_rollback.sql

# Or revert manually (extreme cases)
psql $DATABASE_URL -c "DROP TABLE IF EXISTS table_name;"
```

#### Full Rollback Checklist

- [ ] Revert Docker image tag
- [ ] Restore database state if needed
- [ ] Verify DNS routing
- [ ] Check alert acknowledgments
- [ ] Notify stakeholders

### Environment Configuration

#### Development

```bash
# .env.development
RUST_LOG=debug
API_BIND_ADDR=127.0.0.1:8080
BLOCKCHAIN_NETWORK=standalone
DATABASE_URL=postgres://postgres:postgres@localhost/predictiq_dev
REDIS_URL=redis://localhost:6379
```

#### Staging

```bash
# .env.staging
RUST_LOG=info
API_BIND_ADDR=0.0.0.0:8080
BLOCKCHAIN_NETWORK=testnet
DATABASE_URL=postgres://user:pass@staging-db.predictiq.internal/predictiq_staging
REDIS_URL=redis://staging-redis.predictiq.internal:6379
ADMIN_WHITELIST_IPS=10.0.0.0/8
```

#### Production

```bash
# .env.production
RUST_LOG=warn
API_BIND_ADDR=0.0.0.0:8080
BLOCKCHAIN_NETWORK=mainnet
DATABASE_URL=postgres://user:prod-secure-password@prod-db.predictiq.internal/predictiq_prod
REDIS_URL=redis://prod-redis.predictiq.internal:6379
ADMIN_WHITELIST_IPS=10.0.0.0/8,YourOfficeIP
API_KEYS=production-api-key-1,production-api-key-2
REQUEST_SIGNING_SECRET=your-production-hmac-secret
```

### Secrets Management

#### Using AWS Secrets Manager

```bash
# Store secret
aws secretsmanager create-secret \
  --name predictiq/production/api-keys \
  --secret-string '{"api_key":"your-key-here"}'

# Retrieve in application
# Configure AWS SDK to automatically fetch secrets
export AWS_SECRETS_MANAGER=true
```

#### Using HashiCorp Vault

```bash
# Store secret
vault kv put predictiq/prod/api-keys api_key="your-key-here"

# Enable Kubernetes auth
vault auth enable kubernetes

# Create policy
vault policy write predictiq-prod - <<EOF
path "secret/data/predictiq/*" {
  capabilities = ["read"]
}
EOF
```

#### Secret Rotation

```bash
# Rotate API keys quarterly
# 1. Generate new key
openssl rand -hex 32

# 2. Add to application (rolling update)
kubectl set env deployment/predictiq-api \
  API_KEYS="new-key,old-key-1,old-key-2"

# 3. After verification, remove old key
kubectl set env deployment/predictiq-api \
  API_KEYS="new-key,old-key-1"

# 4. Final cleanup
kubectl set env deployment/predictiq-api \
  API_KEYS="new-key"
```

---

## Monitoring and Alerting

### Metrics to Monitor

#### Application Metrics

| Metric | Type | Description | Query |
|--------|------|-------------|-------|
| `http_requests_total` | Counter | Total HTTP requests | `sum(rate(http_requests_total[5m]))` |
| `http_request_duration_seconds` | Histogram | Request latency | `histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))` |
| `active_connections` | Gauge | Active connections | `predictiq_active_connections` |
| `rate_limit_hits_total` | Counter | Rate limit rejections | `sum(rate(rate_limit_hits_total[5m]))` |

#### Business Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `markets_created_total` | Counter | Total markets created |
| `bets_placed_total` | Counter | Total bets placed |
| `active_users` | Gauge | Currently active users |
| `volume_total_xlm` | Counter | Total betting volume |

#### Infrastructure Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `cpu_usage` | Gauge | CPU utilization % |
| `memory_usage` | Gauge | Memory utilization % |
| `disk_usage` | Gauge | Disk utilization % |
| `db_connections` | Gauge | PostgreSQL connections |
| `redis_memory` | Gauge | Redis memory usage |

### Alert Thresholds

| Alert | Condition | Severity | Action |
|-------|-----------|----------|--------|
| **High Error Rate** | errors > 5% for 5min | Critical | Page on-call |
| **High Latency** | p99 > 2s for 5min | High | Investigate |
| **Rate Limited** | rate_limit_hits > 100/min | Medium | Check for abuse |
| **High Memory** | memory > 85% for 10min | High | Scale up |
| **High CPU** | cpu > 80% for 10min | High | Scale up |
| **DB Connections** | connections > 80% max | High | Check queries |
| **Disk Full** | disk > 90% | Critical | Clean up logs |

#### Prometheus Alert Rules

```yaml
# prometheus/alerts.yaml
groups:
  - name: predictiq
    rules:
      - alert: HighErrorRate
        expr: rate(http_requests_total{status=~"5.."}[5m]) > 0.05
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High error rate detected"
          
      - alert: HighLatency
        expr: histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m])) > 2
        for: 5m
        labels:
          severity: high
        annotations:
          summary: "High request latency"
```

### Dashboard Setup

#### Grafana Dashboard JSON

```json
{
  "title": "PredictIQ - Production Overview",
  "panels": [
    {
      "title": "Request Rate",
      "type": "graph",
      "targets": [
        {
          "expr": "sum(rate(http_requests_total[5m]))",
          "legendFormat": "Requests/sec"
        }
      ]
    },
    {
      "title": "Error Rate",
      "type": "graph",
      "targets": [
        {
          "expr": "sum(rate(http_requests_total{status=~\"5..\"}[5m]))",
          "legendFormat": "5xx errors"
        }
      ]
    },
    {
      "title": "Latency (p95, p99)",
      "type": "graph",
      "targets": [
        {
          "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
          "legendFormat": "p95"
        },
        {
          "expr": "histogram_quantile(0.99, rate(http_request_duration_seconds_bucket[5m]))",
          "legendFormat": "p99"
        }
      ]
    },
    {
      "title": "Active Users",
      "type": "stat",
      "targets": [
        {
          "expr": "predictiq_active_users",
          "legendFormat": "Active Users"
        }
      ]
    }
  ]
}
```

### Log Aggregation

#### Structured Logging

```rust
// Log format in Rust
tracing::info!(
    request_id = %request_id,
    user_id = %user_id,
    endpoint = %endpoint,
    method = %method,
    status = %status,
    duration_ms = %duration.as_millis(),
    "request_completed"
);
```

#### Log Retention

| Log Type | Retention | Storage |
|----------|-----------|---------|
| Application logs | 90 days | CloudWatch/ELK |
| Access logs | 1 year | S3 |
| Security logs | 2 years | S3 |
| Audit logs | 7 years | Cold storage |

#### Query Examples

```bash
# Find all 500 errors in last hour
kubectl logs -l app=predictiq-api --since=1h | grep "500"

# Find specific user requests
kubectl logs -l app=predictiq-api | grep "user_id=abc123"

# Find slow queries
kubectl logs -l app=predictiq-api | grep "slow query"
```

---

## Operational Runbooks

### Incident Response

#### 1. Service Down

```bash
# 1. Check pod status
kubectl get pods -l app=predictiq-api

# 2. Check pod logs
kubectl logs deployment/predictiq-api --tail=100

# 3. Describe pod for events
kubectl describe pod <pod-name>

# 4. Check resource usage
kubectl top pods

# 5. If crash looping, get crash logs
kubectl logs --previous <pod-name>

# 6. Restart if needed
kubectl rollout restart deployment/predictiq-api
```

#### 2. High Latency

```bash
# 1. Check current latency
curl -w "@curl-format.txt" -o /dev/null -s http://api.predictiq.com/health

# 2. Check database connections
psql $DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity;"

# 3. Check slow queries
psql $DATABASE_URL -c "SELECT query, calls, mean_time FROM pg_stat_statements ORDER BY mean_time DESC LIMIT 10;"

# 4. Check Redis
redis-cli info stats | grep -E "instantaneous_ops|keyspace"

# 5. Scale up if needed
kubectl scale deployment predictiq-api --replicas=5
```

#### 3. Database Issues

```bash
# Check connection pool
psql $DATABASE_URL -c "SELECT * FROM pg_pool_status();"

# Check for locks
psql $DATABASE_URL -c "SELECT * FROM pg_locks WHERE NOT granted;"

# Check query performance
EXPLAIN ANALYZE SELECT * FROM users WHERE email = 'user@example.com';

# Kill long-running query
psql $DATABASE_URL -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE query_start < NOW() - INTERVAL '5 minutes';"
```

### Common Issues and Fixes

| Issue | Symptom | Fix |
|-------|---------|-----|
| **OOMKilled** | Pod restarts | Increase memory limit, check for leaks |
| **Connection Timeout** | 504 errors | Scale pods, check DB |
| **Rate Limited** | 429 errors | Check for abuse, adjust limits |
| **Stale Cache** | Wrong data | Flush Redis cache |
| **Migration Lock** | Can't deploy | Kill long transactions |

### Scaling Procedures

#### Horizontal Scaling

```bash
# Scale API pods
kubectl scale deployment predictiq-api --replicas=10

# Or use HPA
kubectl autoscale deployment predictiq-api \
  --min=3 \
  --max=20 \
  --cpu-percent=70

# Check scaling status
kubectl get hpa
```

#### Vertical Scaling

```bash
# Update resource requests
kubectl patch deployment predictiq-api \
  -p '{"spec":{"template":{"spec":{"containers":[{"name":"api","resources":{"limits":{"cpu":"4","memory":"8Gi"}}}]}}}}'
```

### Backup and Recovery

#### Database Backup

```bash
# Daily automated backup (cron)
0 2 * * * pg_dump -Fc -f /backups/predictiq-$(date +\%Y\%m\%d).dump predictiq

# Upload to S3
aws s3 cp /backups/predictiq-$(date +\%Y\%m\%d).dump s3://predictiq-backups/

# Retention: 30 daily, 12 monthly
```

#### Restore from Backup

```bash
# Stop application
kubectl scale deployment predictiq-api --replicas=0

# Drop existing database
psql $DATABASE_URL -c "DROP DATABASE predictiq;"

# Create fresh database
psql $DATABASE_URL -c "CREATE DATABASE predictiq;"

# Restore
pg_restore -d predictiq /backups/predictiq-20240215.dump

# Start application
kubectl scale deployment predictiq-api --replicas=3
```

### Database Migrations

#### Running Migrations

```bash
# Run all pending migrations
cargo run --release -- migrate

# Run specific migration
psql $DATABASE_URL -f database/migrations/008_create_email_tracking.sql

# Verify migration status
psql $DATABASE_URL -c "SELECT * FROM schema_migrations;"
```

#### Safe Migration Process

```bash
# 1. Test on staging first
# 2. Create migration script
cat > database/migrations/009_add_new_field.sql <<EOF
ALTER TABLE users ADD COLUMN new_field VARCHAR(255);
EOF

# 3. Run during low traffic window
# 4. Monitor for errors
# 5. Roll back if needed
psql $DATABASE_URL -c "ALTER TABLE users DROP COLUMN new_field;"
```

---

## Security Procedures

### Access Control

#### Kubernetes RBAC

```yaml
# k8s/rbac.yaml
apiVersion: rbac.authorization.k8s.io/v1
kind: Role
metadata:
  name: predictiq-developer
rules:
  - apiGroups: [""]
    resources: ["pods", "services"]
    verbs: ["get", "list", "watch", "logs"]
  - apiGroups: [""]
    resources: ["pods/exec"]
    verbs: ["create"]
---
apiVersion: rbac.authorization.k8s.io/v1
kind: RoleBinding
metadata:
  name: predictiq-developers
subjects:
  - kind: Group
    name: developers
    apiGroup: rbac.authorization.k8s.io
roleRef:
  kind: Role
  name: predictiq-developer
  apiGroup: rbac.authorization.k8s.io
```

#### AWS IAM

```json
{
  "Version": "2012-10-17",
  "Statement": [
    {
      "Effect": "Allow",
      "Action": [
        "ec2:Describe*",
        "rds:DescribeDBInstances",
        "elasticache:Describe*"
      ],
      "Resource": "*"
    }
  ]
}
```

### Secret Rotation

#### API Keys

```bash
# Quarterly rotation
# 1. Generate new key
NEW_KEY=$(openssl rand -hex 32)

# 2. Add to environment (maintain old key for 24h)
kubectl create secret generic api-keys \
  --from-literal=new-key=$NEW_KEY \
  --dry-run=client -o yaml | kubectl apply -f -

# 3. Deploy with new key
kubectl rollout restart deployment/predictiq-api

# 4. After 24h, remove old key
# Update secret, remove old key
kubectl rollout restart deployment/predictiq-api
```

#### Database Credentials

```bash
# Rotate monthly
aws secretsmanager rotate-secret \
  --secret-id predictiq/prod/database \
  --rotation-lambda-arn arn:aws:lambda:region:account:function:rotate
```

### Security Updates

```bash
# Weekly security updates
# 1. Check for updates
cargo outdated

# 2. Check security advisories
cargo audit

# 3. Update dependencies
cargo update

# 4. Test thoroughly
cargo test --all
cargo clippy --all

# 5. Deploy to staging first
# 6. Deploy to production
```

---

## Performance Optimization

### Application Tuning

#### Rust Optimization

```toml
# Cargo.toml - Release profile
[profile.release]
opt-level = 3
lto = "thin"
codegen-units = 1
panic = "abort"
strip = true
```

#### Connection Pooling

```rust
// Configure PostgreSQL connection pool
let pool = PgPoolOptions::new()
    .max_connections(20)
    .min_connections(5)
    .acquire_timeout(Duration::from_secs(30))
    .idle_timeout(Duration::from_secs(600))
    .max_lifetime(Duration::from_secs(1800))
    .connect(&database_url)
    .await?;
```

#### Caching Strategy

```rust
// Redis caching for API responses
async fn get_cached_markets(
    cache: &RedisCache,
    category: &str,
) -> Result<Vec<Market>, AppError> {
    let cache_key = format!("markets:{}", category);
    
    // Try cache first
    if let Some(cached) = cache.get(&cache_key).await? {
        return Ok(cached);
    }
    
    // Fetch from database
    let markets = db.get_markets(category).await?;
    
    // Cache for 5 minutes
    cache.set(&cache_key, &markets, Duration::from_secs(300)).await?;
    
    Ok(markets)
}
```

### Database Optimization

```sql
-- Add indexes for common queries
CREATE INDEX idx_markets_category ON markets(category);
CREATE INDEX idx_markets_status ON markets(status);
CREATE INDEX idx_bets_user_id ON bets(user_id);
CREATE INDEX idx_bets_market_id ON bets(market_id);

-- Analyze query performance
ANALYZE;
EXPLAIN (ANALYZE, BUFFERS) SELECT * FROM markets WHERE status = 'active';
```

---

## Cost Optimization

### Cost Breakdown

| Service | Monthly Cost (Est) | Optimization Tips |
|---------|-------------------|-------------------|
| **EC2/EKS** | $500-2000 | Use spot instances, right-size |
| **RDS** | $200-500 | Reserved instances |
| **ElastiCache** | $100-300 | Right-size, use Redis cluster |
| **Cloudflare** | $50-200 | Cache more, optimize rules |
| **Data Transfer** | $50-200 | CloudFront, compression |
| **Monitoring** | $50-100 | Adjust retention |

### Optimization Strategies

#### Compute

```bash
# Use spot instances for non-critical workloads
# EKS node groups
eksctl create nodegroup \
  --cluster predictiq \
  --name spot-nodes \
  --instance-types t3.medium \
  --spot
```

#### Database

```bash
# Use reserved instances for predictable load
# 1 year reserved vs on-demand: ~40% savings
aws rds purchase-reserved-db-instance-offering \
  --reserved-db-instance-offering-id offering-id \
  --db-instance-count 1
```

#### Storage

```bash
# Move old logs to cold storage
aws s3 cp s3://predictiq-logs/ s3://predictiq-logs-cold/ \
  --storage-class GLACIER \
  --recursive
```

---

## Appendix

### Quick Reference Commands

```bash
# Deployment
kubectl rollout restart deployment/predictiq-api
kubectl rollout status deployment/predictiq-api
kubectl rollout undo deployment/predictiq-api

# Logs
kubectl logs -f deployment/predictiq-api
kubectl logs --previous <pod>

# Debug
kubectl exec -it <pod> -- /bin/sh
kubectl port-forward svc/predictiq-api 8080:80

# Database
psql $DATABASE_URL -c "SELECT * FROM schema_migrations;"
psql $DATABASE_URL -c "SELECT pg_size_pretty(pg_database_size('predictiq'));"

# Redis
redis-cli INFO stats
redis-cli KEYS "*" | head -20

# Health
curl http://localhost:8080/health
curl http://localhost:8080/metrics
```

### Environment Variables Reference

| Variable | Required | Description |
|----------|----------|-------------|
| `DATABASE_URL` | Yes | PostgreSQL connection string |
| `REDIS_URL` | Yes | Redis connection string |
| `API_BIND_ADDR` | Yes | Server bind address |
| `BLOCKCHAIN_NETWORK` | Yes | Network (mainnet/testnet) |
| `RUST_LOG` | No | Log level |
| `API_KEYS` | No | Comma-separated API keys |
| `ADMIN_WHITELIST_IPS` | No | Admin IP whitelist |

---

*Last Updated: February 2024*
*Document Owner: DevOps Team*
*Next Review: August 2024*
