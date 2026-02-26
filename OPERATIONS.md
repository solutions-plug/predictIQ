# PredictIQ Operations Runbook

> **Issue**: [#88 - Create Deployment and Operations Documentation](https://github.com/solutions-plug/predictIQ/issues/88)

This document provides operational runbooks, procedures, and checklists for the PredictIQ platform.

---

## Table of Contents

1. [Operational Checklists](#operational-checklists)
2. [Incident Response Procedures](#incident-response-procedures)
3. [Common Issues and Resolutions](#common-issues-and-resolutions)
4. [Runbook Templates](#runbook-templates)

---

## Operational Checklists

### Pre-Deployment Checklist

- [ ] All tests passing (`cargo test --all`)
- [ ] Code passes clippy (`cargo clippy --all`)
- [ ] Security audit passes (`cargo audit`)
- [ ] Migration scripts tested
- [ ] Database backup verified
- [ ] Environment variables configured
- [ ] Secrets properly set
- [ ] Health check endpoint working
- [ ] Staging deployment verified
- [ ] Rollback plan documented
- [ ] Stakeholders notified
- [ ] Monitoring alerts acknowledged

### Post-Deployment Checklist

- [ ] Health check passing
- [ ] No new errors in logs
- [ ] Response times normal
- [ ] Database connections healthy
- [ ] Cache hit rates normal
- [ ] Rate limiting正常工作
- [ ] All endpoints responding
- [ ] Critical metrics in normal range
- [ ] Stakeholders notified of success
- [ ] Deployment logged

### Daily Operations Checklist

- [ ] Review dashboard for anomalies
- [ ] Check error rates
- [ ] Review slow queries
- [ ] Verify backup completion
- [ ] Check disk space
- [ ] Review security logs
- [ ] Monitor API response times
- [ ] Check database connection pool
- [ ] Review rate limit violations
- [ ] Verify monitoring alerts

### Weekly Operations Checklist

- [ ] Review weekly performance report
- [ ] Analyze trend data
- [ ] Check for security updates
- [ ] Rotate logs
- [ ] Review and update documentation
- [ ] Check dependency updates
- [ ] Review access logs for anomalies
- [ ] Test backup restoration
- [ ] Review capacity planning
- [ ] Update runbooks if needed

### Monthly Operations Checklist

- [ ] Security audit review
- [ ] Performance tuning review
- [ ] Cost analysis
- [ ] Dependency audit
- [ ] Access review
- [ ] Disaster recovery test
- [ ] Capacity planning review
- [ ] Compliance review

---

## Incident Response Procedures

### Severity Levels

| Level | Description | Response Time | Examples |
|-------|-------------|---------------|----------|
| **P1 - Critical** | Service down, data loss | 15 minutes | Database unavailable, all API down |
| **P2 - High** | Major feature broken | 1 hour | Payments failing, can't place bets |
| **P3 - Medium** | Minor feature broken | 4 hours | Dashboard slow, some errors |
| **P4 - Low** | Cosmetic issue | 24 hours | UI glitch, non-critical error |

### Incident Response Flow

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                         Incident Response Flow                               │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌─────────┐                                                               │
│  │ Detect  │◄─── Monitoring Alert                                          │
│  └────┬────┘     User Report    Internal Discovery                        │
│       │                                                                    │
│       ▼                                                                    │
│  ┌─────────┐                                                               │
│  │ Assess  │◄─── Determine Severity                                        │
│  └────┬────┘     Identify Impact                                          │
│       │                                                                    │
│       ▼                                                                    │
│  ┌─────────┐                                                               │
│  │ Respond │◄─── Execute Runbook                                          │
│  └────┬────┘     Communicate Status                                      │
│       │                                                                    │
│       ▼                                                                    │
│  ┌─────────┐                                                               │
│  │Resolve  │◄─── Fix Applied                                             │
│  └────┬────┘     Verification                                             │
│       │                                                                    │
│       ▼                                                                    │
│  ┌─────────┐                                                               │
│  │ Review  │◄─── Post-Mortem                                               │
│  └─────────┘     Update Runbooks                                          │
│                                                                           │
└─────────────────────────────────────────────────────────────────────────────┘
```

### Communication Templates

#### Initial Alert

```
🚨 INCIDENT ALERT

Severity: [P1/P2/P3/P4]
Title: [Brief description]
Status: Investigating
Impact: [User impact description]
Affected: [Systems/services affected]

Actions:
- [ ] Investigating root cause
- [ ] Notified: [list]

Next update in: 30 minutes
```

#### Status Update

```
📊 INCIDENT UPDATE

Status: [Investigating/Identified/Monitoring/Resolved]
Title: [Brief description]

What happened: [Description]
What we're doing: [Current actions]
ETA: [If known]

Next update in: [Time]
```

#### Resolution

```
✅ INCIDENT RESOLVED

Title: [Brief description]
Duration: [Start time] - [End time]
Severity: [P1/P2/P3/P4]

Root cause: [Description]
Fix applied: [Description]
Lessons learned: [Any lessons]

Post-mortem: [Link if available]
```

---

## Common Issues and Resolutions

### Issue: High CPU Usage

**Symptoms:**
- API response times increase
- Pods may restart
- CPU metrics > 80%

**Diagnosis:**
```bash
# Check pod CPU usage
kubectl top pods

# Check which process
kubectl exec -it <pod> -- top

# Check for infinite loops in logs
kubectl logs <pod> | grep -i "loop"
```

**Resolution:**
```bash
# Scale up temporarily
kubectl scale deployment predictiq-api --replicas=10

# Check for bad queries
psql $DATABASE_URL -c "SELECT query, calls FROM pg_stat_statements ORDER BY total_time DESC LIMIT 5;"
```

**Prevention:**
- Set CPU limits
- Optimize slow queries
- Use connection pooling

---

### Issue: Out of Memory (OOM)

**Symptoms:**
- Pods in CrashLoopBackOff
- OOMKilled in events
- Memory usage > 90%

**Diagnosis:**
```bash
# Check pod events
kubectl describe pod <pod> | grep -A 5 "Last State"

# Check memory limits
kubectl get pod <pod> -o jsonpath='{.spec.containers[0].resources}'
```

**Resolution:**
```bash
# Increase memory limit
kubectl patch deployment predictiq-api \
  -p '{"spec":{"template":{"spec":{"containers":[{"name":"api","resources":{"limits":{"memory":"4Gi"}}}]}}}}'

# Restart pods
kubectl rollout restart deployment/predictiq-api
```

**Prevention:**
- Set appropriate memory limits
- Fix memory leaks
- Monitor memory usage

---

### Issue: Database Connection Exhaustion

**Symptoms:**
- "Too many connections" errors
- 503 Service Unavailable
- Database CPU at 100%

**Diagnosis:**
```bash
# Check current connections
psql $DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity;"

# Check for idle connections
psql $DATABASE_URL -c "SELECT count(*) FROM pg_stat_activity WHERE state = 'idle';"

# Find longest running queries
psql $DATABASE_URL -c "SELECT pid, query, duration FROM pg_stat_activity WHERE state != 'idle' ORDER BY duration DESC;"
```

**Resolution:**
```bash
# Kill idle connections
psql $DATABASE_URL -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE state = 'idle' AND query_start < NOW() - INTERVAL '10 minutes';"

# Reduce connection pool size in app
# Update PgPoolOptions max_connections
```

**Prevention:**
- Set appropriate pool size
- Implement connection timeouts
- Use connection pooler (PgBouncer)

---

### Issue: Rate Limit Errors

**Symptoms:**
- 429 Too Many Requests
- Users complaining
- Rate limit metric spike

**Diagnosis:**
```bash
# Check rate limit metrics
curl http://localhost:8080/metrics | grep rate_limit

# Check for attack patterns
kubectl logs | grep "rate limit exceeded"
```

**Resolution:**
```bash
# Temporarily increase limits
kubectl set env deployment/predictiq-api \
  RATE_LIMIT_GLOBAL=200

# Or add IP to whitelist
kubectl set env deployment/predictiq-api \
  ADMIN_WHITELIST_IPS="existing,new-ip"
```

**Prevention:**
- Implement better caching
- Add CAPTCHA for suspicious activity
- Use Cloudflare rate limiting

---

### Issue: Slow Queries

**Symptoms:**
- High database CPU
- Slow API responses
- Query time > 1s

**Diagnosis:**
```bash
# Enable query timing
psql $DATABASE_URL -c "SELECT pg_stat_statements.track = 'all';"

# Find slowest queries
psql $DATABASE_URL -c "SELECT query, calls, mean_time, total_time FROM pg_stat_statements ORDER BY mean_time DESC LIMIT 10;"

# Analyze specific query
EXPLAIN ANALYZE <slow_query>;
```

**Resolution:**
```bash
# Add index
CREATE INDEX idx_table_column ON table(column);

# Or kill long query
psql $DATABASE_URL -c "SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE query_start < NOW() - INTERVAL '1 minute';"
```

**Prevention:**
- Regular index maintenance
- Query review process
- Slow query logging

---

### Issue: Cache Miss Storm

**Symptoms:**
- Database CPU spikes
- High latency
- Cache hit rate < 50%

**Diagnosis:**
```bash
# Check Redis info
redis-cli INFO stats | grep keyspace

# Check cache hit rate
redis-cli INFO stats | grep hit_rate
```

**Resolution:**
```bash
# Warm cache
curl -X POST http://localhost:8080/api/admin/cache/warm

# Or manually populate
redis-cli SET "markets:active" "$(curl http://api/markets?status=active)"
```

**Prevention:**
- Implement cache warming
- Use cache-aside pattern
- Set appropriate TTLs

---

### Issue: SSL/TLS Certificate Expired

**Symptoms:**
- "Certificate expired" errors
- Users can't access site
- SSL check fails

**Diagnosis:**
```bash
# Check certificate
curl -vI https://predictiq.com

# Check with openssl
openssl s_client -connect predictiq.com:443 -showcerts
```

**Resolution:**
```bash
# Renew via Cloudflare
# Or manually with certbot
certbot renew --dry-run

# Force renewal
certbot renew --force-renewal
```

**Prevention:**
- Enable auto-renewal
- Set calendar reminders
- Monitor SSL expiry

---

### Issue: Disk Space Full

**Symptoms:**
- Write failures
- Pods crashing
- "No space left" errors

**Diagnosis:**
```bash
# Check disk usage
kubectl exec -it <pod> -- df -h

# Find large files
kubectl exec -it <pod> -- find /var/log -type f -size +100M
```

**Resolution:**
```bash
# Clean old logs
kubectl exec -it <pod> -- truncate -s 0 /var/log/*.log

# Clean Docker
docker system prune -a

# Expand volume (AWS)
aws ec2 modify-volume --volume-id vol-xxx --size 200
```

**Prevention:**
- Set up log rotation
- Monitor disk usage
- Use lifecycle policies

---

## Runbook Templates

### Template: Service Deployment Runbook

```markdown
# Service Deployment Runbook: [Service Name]

## Pre-Deployment
- [ ] Changes reviewed and approved
- [ ] Tests passing
- [ ] Database migrations ready
- [ ] Rollback plan documented
- [ ] Stakeholders notified

## Deployment Steps
1. Run database migrations:
   ```bash
   cargo run --release -- migrate
   ```

2. Build Docker image:
   ```bash
   docker build -t predictiq-api:$VERSION .
   ```

3. Deploy to staging:
   ```bash
   kubectl set image deployment/predictiq-api api=predictiq-api:$VERSION
   ```

4. Verify staging:
   ```bash
   curl https://staging.predictiq.com/health
   ```

5. Deploy to production:
   ```bash
   kubectl set image deployment/predictiq-api api=predictiq-api:$VERSION
   ```

## Post-Deployment
- [ ] Health check passing
- [ ] No errors in logs
- [ ] Metrics normal
- [ ] Stakeholders notified

## Rollback Steps
1. Rollback deployment:
   ```bash
   kubectl rollout undo deployment/predictiq-api
   ```

2. Revert database if needed:
   ```bash
   psql $DATABASE -f migrations/downgrade.sql
   ```

## Monitoring
- Dashboard: [Link]
- Alerts: [Link]
- Logs: [Link]
```

---

### Template: Database Migration Runbook

```markdown
# Database Migration Runbook: [Migration Name]

## Pre-Migration
- [ ] Backup created
- [ ] Tested on staging
- [ ] Estimated downtime: [X] minutes
- [ ] Rollback plan ready

## Migration Steps
1. Create backup:
   ```bash
   pg_dump -Fc predictiq > backup_$(date +%Y%m%d).dump
   ```

2. Run migration:
   ```bash
   psql $DATABASE -f migrations/$(migration_name).sql
   ```

3. Verify migration:
   ```bash
   psql $DATABASE -c "SELECT * FROM schema_migrations;"
   ```

## Post-Migration
- [ ] Application tests passing
- [ ] No errors in logs
- [ ] Performance normal

## Rollback Steps
1. Restore from backup:
   ```bash
   dropdb predictiq
   createdb predictiq
   pg_restore -d predictiq backup_$(date +%Y%m%d).dump
   ```

## Monitoring
- Query performance: [Link]
- Error rate: [Link]
```

---

### Template: Incident Response Runbook

```markdown
# Incident Response Runbook: [Incident Type]

## Detection
- **Alert Source:** [Monitoring tool]
- **Trigger:** [What caused the alert]

## Impact
- **Users Affected:** [Number/percentage]
- **Severity:** [P1/P2/P3/P4]
- **Duration:** [Estimated]

## Diagnosis
1. Check service status:
   ```bash
   kubectl get pods -l app=predictiq-api
   ```

2. Check logs:
   ```bash
   kubectl logs -l app=predictiq-api --tail=100
   ```

3. Check metrics:
   ```bash
   # Look at [specific metrics]
   ```

## Resolution
1. [Step 1]
2. [Step 2]
3. [Step 3]

## Verification
- [ ] Service healthy
- [ ] No errors
- [ ] Metrics normal

## Post-Incident
- [ ] Post-mortem scheduled
- [ ] Runbook updated
- [ ] Prevention measures identified
```

---

### Template: Scaling Runbook

```markdown
# Scaling Runbook: [Service]

## Horizontal Scaling
1. Check current replicas:
   ```bash
   kubectl get deployment predictiq-api
   ```

2. Scale up:
   ```bash
   kubectl scale deployment predictiq-api --replicas=10
   ```

3. Verify:
   ```bash
   kubectl get pods -l app=predictiq-api
   ```

## Vertical Scaling
1. Check current resources:
   ```bash
   kubectl get pods -o jsonpath='{.items[*].spec.containers[0].resources}'
   ```

2. Update resources:
   ```bash
   kubectl patch deployment predictiq-api -p '{"spec":{"template":{"spec":{"containers":[{"name":"api","resources":{"limits":{"cpu":"4","memory":"8Gi"}}}]}}}}'
   ```

3. Verify:
   ```bash
   kubectl rollout status deployment/predictiq-api
   ```

## Auto-Scaling (HPA)
1. Enable HPA:
   ```bash
   kubectl autoscale deployment predictiq-api --min=3 --max=20 --cpu-percent=70
   ```

2. Monitor:
   ```bash
   kubectl get hpa
   ```

## Scale Down
1. Scale down after peak:
   ```bash
   kubectl scale deployment predictiq-api --replicas=3
   ```
```

---

### Template: Security Incident Runbook

```markdown
# Security Incident Runbook: [Incident Type]

## Severity Assessment
- [ ] P1 - Critical (Data breach, active attack)
- [ ] P2 - High (Potential breach, vulnerability)
- [ ] P3 - Medium (Security policy violation)
- [ ] P4 - Low (Minor security issue)

## Immediate Actions
1. **Contain:**
   ```bash
   # Isolate affected systems
   kubectl scale deployment predictiq-api --replicas=0
   ```

2. **Preserve:**
   ```bash
   # Save logs
   kubectl logs -l app=predictiq-api > logs/incident_$(date +%Y%m%d).log
   
   # Save state
   pg_dump predictiq > backup_$(date +%Y%m%d).sql
   ```

3. **Notify:**
   - Security team
   - Legal/compliance
   - Management

## Investigation
1. Review access logs
2. Check for unauthorized access
3. Identify attack vector
4. Assess data exposure

## Remediation
1. [Remediation step 1]
2. [Remediation step 2]
3. [Remediation step 3]

## Recovery
1. Restore service
2. Verify integrity
3. Monitor for recurrence

## Post-Incident
- [ ] Complete incident report
- [ ] Update security policies
- [ ] Implement prevention measures
- [ ] Schedule security review
```

---

### Template: Backup and Recovery Runbook

```markdown
# Backup and Recovery Runbook

## Backup Verification
1. Check last backup:
   ```bash
   ls -la /backups/
   ```

2. Verify backup size:
   ```bash
   du -h /backups/predictiq_latest.dump
   ```

3. Test backup integrity:
   ```bash
   pg_restore --list /backups/predictiq_latest.dump
   ```

## Restore from Backup
1. Stop application:
   ```bash
   kubectl scale deployment predictiq-api --replicas=0
   ```

2. Drop existing database:
   ```bash
   psql $DATABASE -c "DROP DATABASE predictiq;"
   ```

3. Create database:
   ```bash
   psql $DATABASE -c "CREATE DATABASE predictiq;"
   ```

4. Restore:
   ```bash
   pg_restore -d predictiq /backups/predictiq_latest.dump
   ```

5. Start application:
   ```bash
   kubectl scale deployment predictiq-api --replicas=3
   ```

6. Verify:
   ```bash
   curl https://predictiq.com/health
   ```

## Disaster Recovery
- RTO: 4 hours
- RPO: 1 hour
- Procedure: [Link to full DR plan]
```

---

## Quick Reference

### Emergency Contacts

| Role | Contact | Phone |
|------|---------|-------|
| On-Call Engineer | [Name] | [Phone] |
| DevOps Lead | [Name] | [Phone] |
| Security Lead | [Name] | [Phone] |
| Engineering Manager | [Name] | [Phone] |

### Critical URLs

| Service | URL |
|---------|-----|
| Production API | https://api.predictiq.com |
| Staging API | https://staging.predictiq.com |
| Grafana | https://grafana.predictiq.com |
| PagerDuty | https://predictiq.pagerduty.com |
| AWS Console | https://console.aws.amazon.com |

### Critical Commands

```bash
# Get help quickly
alias k='kubectl'
alias kgp='kubectl get pods'
alias kgs='kubectl get services'
alias klf='kubectl logs -f'
alias kdp='kubectl describe pod'

# Quick status
alias status='kubectl get pods,svc,deployment'

# Database
alias psql-prod='psql $DATABASE_URL'
alias backup='pg_dump -Fc predictiq > backup_$(date +%Y%m%d).dump'
```

---

*Last Updated: February 2024*
*Document Owner: DevOps Team*
*Next Review: August 2024*
