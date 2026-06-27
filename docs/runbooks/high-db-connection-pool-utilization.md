# High Database Connection Pool Utilization Runbook

## Alert Meaning
Database connection pool utilization has exceeded 80% for 5 minutes.

## Impact
- Risk of connection exhaustion
- New requests may fail to acquire connections
- Potential service degradation

## Investigation Steps

1. **Check connection pool status**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=db_connections_active / db_connections_max'
   ```

2. **Identify connection consumers**
   ```sql
   SELECT user, COUNT(*) FROM information_schema.processlist GROUP BY user;
   ```

3. **Check for idle connections**
   ```sql
   SELECT * FROM information_schema.processlist WHERE command = 'Sleep';
   ```

4. **Review query performance**
   - Check for slow queries holding connections
   - Look for transaction locks

## Remediation

### Immediate Actions
1. Increase connection pool size
2. Kill idle connections
3. Review and optimize slow queries
4. Check for connection leaks

### Short-term
1. Implement connection pooling
2. Add connection timeout
3. Optimize query performance
4. Implement circuit breakers

### Long-term
1. Monitor connection pool metrics
2. Establish connection pool SLOs
3. Implement connection pool auto-scaling
4. Regular performance tuning
