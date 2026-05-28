# Critical Database Connection Pool Utilization Runbook

## Alert Meaning
Database connection pool utilization has exceeded 95% critical threshold for 2 minutes.

## Impact
- Imminent connection pool exhaustion
- New requests will fail
- Service outage risk

## Investigation Steps

1. **Immediate assessment**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=db_connections_active / db_connections_max'
   ```

2. **Check active connections**
   ```sql
   SHOW PROCESSLIST;
   ```

3. **Identify blocking queries**
   ```sql
   SELECT * FROM information_schema.processlist WHERE state != 'Sleep';
   ```

## Remediation

### Immediate Actions
1. **Page on-call DBA** - Critical issue
2. Increase connection pool size immediately
3. Kill idle connections
4. Kill long-running queries if safe
5. Consider read-only mode

### Emergency Actions
1. Implement connection throttling
2. Redirect traffic to backup database
3. Scale database resources
4. Implement circuit breakers

### Post-Incident
1. Root cause analysis
2. Optimize connection usage
3. Implement connection monitoring
4. Add connection pool auto-scaling
