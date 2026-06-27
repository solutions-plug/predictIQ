# Significant Performance Degradation Runbook

## Alert Meaning
Response time has degraded by more than 5% compared to 24 hours ago (alert threshold).

## Impact
- Noticeable performance degradation
- Potential SLO impact if trend continues
- May indicate emerging issues

## Investigation Steps

1. **Check performance trend**
   ```bash
   curl 'http://prometheus:9090/api/v1/query_range?query=histogram_quantile(0.95,rate(http_request_duration_seconds_bucket[1h]))&start=<24h-ago>&end=<now>&step=1h'
   ```

2. **Identify trend direction**
   - Is degradation continuing?
   - Is it stabilizing?
   - Are there spikes?

3. **Check for correlations**
   - Database performance
   - Cache hit rates
   - Resource utilization

4. **Review recent changes**
   - Code deployments
   - Infrastructure changes
   - Data volume changes

## Remediation

### Immediate Actions
1. Monitor trend closely
2. Prepare for escalation if continues
3. Identify potential causes

### Short-term
1. Optimize identified bottlenecks
2. Review code changes
3. Check resource utilization
4. Consider preventive scaling

### Long-term
1. Implement continuous performance monitoring
2. Add performance regression tests
3. Establish performance budgets
4. Regular optimization reviews
