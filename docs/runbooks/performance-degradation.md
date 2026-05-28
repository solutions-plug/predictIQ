# Performance Degradation Runbook

## Alert Meaning
Response time has degraded by more than 10% compared to 24 hours ago.

## Impact
- Degraded user experience
- Potential SLO violation
- May indicate resource constraints or code issues

## Investigation Steps

1. **Check current performance**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[1h]))'
   ```

2. **Compare with baseline**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[1h] offset 24h))'
   ```

3. **Identify affected endpoints**
   - Check by endpoint
   - Check by method
   - Check by status code

4. **Check for recent changes**
   - Review deployments
   - Check database changes
   - Review infrastructure changes

## Remediation

### Immediate Actions
1. Analyze performance trends
2. Identify affected endpoints
3. Check for recent deployments

### Short-term
1. Optimize identified bottlenecks
2. Review query performance
3. Check resource utilization
4. Consider rollback if recent deployment

### Long-term
1. Implement performance testing
2. Add performance regression tests
3. Establish performance SLOs
4. Regular optimization reviews
