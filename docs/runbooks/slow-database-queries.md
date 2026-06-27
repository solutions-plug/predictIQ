# Slow Database Queries Runbook

## Alert Meaning
Database p95 query time has exceeded 50ms for 5 minutes.

## Impact
- Slower API responses
- Increased database load
- Potential connection pool exhaustion

## Investigation Steps

1. **Check slow query log**
   ```sql
   SELECT * FROM mysql.slow_log ORDER BY start_time DESC LIMIT 20;
   ```

2. **Identify slow queries**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,rate(db_query_duration_seconds_bucket[5m])) by (query)'
   ```

3. **Check query plans**
   - Use EXPLAIN for identified slow queries
   - Check for missing indexes
   - Review join strategies

4. **Check database health**
   - Connection pool utilization
   - Lock contention
   - Disk I/O

## Remediation

### Immediate Actions
1. Identify and analyze slow queries
2. Check for missing indexes
3. Review query execution plans

### Short-term
1. Add indexes for frequently queried columns
2. Optimize query logic
3. Implement query result caching
4. Consider query rewriting

### Long-term
1. Implement query performance monitoring
2. Add database performance tests
3. Establish query performance SLOs
4. Regular index maintenance
