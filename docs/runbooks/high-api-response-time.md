# High API Response Time Runbook

## Alert Meaning
The API p95 response time has exceeded 200ms threshold for 5 minutes.

## Impact
- Degraded user experience with slower API responses
- Potential cascading failures if latency continues to increase
- May indicate resource contention or inefficient queries

## Investigation Steps

1. **Check current metrics**
   ```bash
   curl http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[5m]))
   ```

2. **Identify affected endpoints**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,rate(http_request_duration_seconds_bucket{endpoint!=""}[5m])) by (endpoint)'
   ```

3. **Check database performance**
   - Query slow query logs
   - Check connection pool utilization
   - Monitor active connections

4. **Check resource utilization**
   - CPU usage on API servers
   - Memory usage and GC pauses
   - Network I/O

5. **Review recent deployments**
   - Check if any code changes were deployed recently
   - Review database schema changes

## Remediation

### Immediate Actions
1. Check if this is a temporary spike or sustained issue
2. If sustained, consider scaling up API instances
3. Review and optimize slow queries

### Short-term
1. Implement query caching if not already in place
2. Add database indexes for frequently queried fields
3. Review API endpoint implementations for inefficiencies

### Long-term
1. Implement performance testing in CI/CD
2. Set up performance regression alerts
3. Establish performance SLOs and budgets
