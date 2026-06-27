# Low Throughput Runbook

## Alert Meaning
Request throughput has dropped below 1000 req/s for 10 minutes.

## Impact
- Reduced system capacity
- Potential service degradation
- May indicate upstream issues or traffic routing problems

## Investigation Steps

1. **Check current throughput**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=rate(http_requests_total[5m])'
   ```

2. **Check by endpoint**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=rate(http_requests_total[5m]) by (endpoint)'
   ```

3. **Check for errors**
   - Are requests being rejected?
   - Are there connection timeouts?
   - Check rate limiting status

4. **Check infrastructure**
   - API instance availability
   - Load balancer health
   - Network connectivity

## Remediation

### Immediate Actions
1. Verify API instances are running
2. Check load balancer configuration
3. Verify DNS resolution
4. Check for rate limiting issues

### Investigation
1. Review application logs
2. Check for deployment issues
3. Verify database connectivity
4. Check external service dependencies

### Recovery
1. Restart affected instances if needed
2. Adjust load balancer configuration
3. Scale up if capacity is insufficient
4. Investigate root cause
