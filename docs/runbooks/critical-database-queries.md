# Critical Database Queries Runbook

## Alert Meaning
Database p99 query time has exceeded 100ms critical threshold for 2 minutes.

## Impact
- Severe API latency
- Risk of connection pool exhaustion
- Potential cascading failures

## Investigation Steps

1. **Immediate query analysis**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.99,rate(db_query_duration_seconds_bucket[5m]))'
   ```

2. **Check for locks**
   ```sql
   SHOW PROCESSLIST;
   SHOW OPEN TABLES WHERE In_use > 0;
   ```

3. **Check connection pool**
   - Active connections
   - Waiting connections
   - Connection timeout rate

4. **Review recent changes**
   - Schema changes
   - Index changes
   - Data volume changes

## Remediation

### Immediate Actions
1. **Page on-call DBA** - This is critical
2. Kill long-running queries if safe
3. Check for table locks
4. Review connection pool settings

### Emergency Actions
1. Increase connection pool size
2. Implement query timeout
3. Redirect traffic if possible
4. Consider read replicas

### Post-Incident
1. Conduct root cause analysis
2. Optimize identified queries
3. Add query performance tests
4. Implement query monitoring
