# High Error Rate Runbook

## Alert Meaning
The API error rate (5xx responses) has exceeded 0.1% for 5 minutes.

## Impact
- Users experiencing failures
- Potential data loss or inconsistency
- Service reliability degradation

## Investigation Steps

1. **Check error rate by endpoint**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=rate(http_requests_total{status=~"5.."}[5m]) by (endpoint)'
   ```

2. **Review error logs**
   ```bash
   kubectl logs -l app=predictiq-api --tail=1000 | grep ERROR
   ```

3. **Check specific error types**
   - 500: Internal Server Error
   - 502: Bad Gateway
   - 503: Service Unavailable
   - 504: Gateway Timeout

4. **Check dependencies**
   - Database connectivity
   - Cache availability
   - External service health

## Remediation

### Immediate Actions
1. Check application logs for error patterns
2. Verify database connectivity
3. Check cache service status
4. Review recent deployments

### Short-term
1. Increase logging verbosity if needed
2. Implement circuit breakers
3. Add retry logic with exponential backoff
4. Scale up instances if resource-constrained

### Long-term
1. Implement comprehensive error tracking
2. Add error budget monitoring
3. Establish error rate SLOs
4. Improve error handling and recovery
