# Critical API Response Time Runbook

## Alert Meaning
The API p99 response time has exceeded 500ms critical threshold for 2 minutes.

## Impact
- Severe user experience degradation
- Potential request timeouts
- Risk of cascading failures across dependent services

## Investigation Steps

1. **Immediate assessment**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.99,rate(http_request_duration_seconds_bucket[5m]))'
   ```

2. **Check for errors**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=rate(http_requests_total{status=~"5.."}[5m])'
   ```

3. **Identify bottlenecks**
   - Check database query times
   - Review external service calls
   - Check cache hit rates

4. **Check infrastructure**
   - CPU and memory on all API instances
   - Network latency
   - Disk I/O

## Remediation

### Immediate Actions
1. **Page on-call engineer** - This is a critical issue
2. **Scale up API instances** if CPU/memory is high
3. **Check for stuck connections** in database
4. **Review recent changes** - rollback if necessary

### Emergency Actions
1. Enable circuit breakers for external services
2. Reduce cache TTL to force fresh data
3. Temporarily disable non-critical features
4. Redirect traffic to backup region if available

### Post-Incident
1. Conduct root cause analysis
2. Implement permanent fix
3. Add performance tests to prevent recurrence
