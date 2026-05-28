# Low Cache Hit Rate Runbook

## Alert Meaning
Cache hit rate has dropped below 80% for 10 minutes.

## Impact
- Increased database load
- Slower response times
- Higher operational costs

## Investigation Steps

1. **Check cache metrics**
   ```bash
   curl 'http://prometheus:9090/api/v1/query?query=rate(cache_hits_total[5m]) / (rate(cache_hits_total[5m]) + rate(cache_misses_total[5m]))'
   ```

2. **Check cache size and evictions**
   - Monitor Redis memory usage
   - Check eviction policy
   - Review key expiration rates

3. **Identify problematic keys**
   - Check for cache stampedes
   - Review key access patterns
   - Check for inefficient caching

## Remediation

### Immediate Actions
1. Increase cache size if memory allows
2. Review cache eviction policy
3. Check for cache key collisions

### Short-term
1. Optimize cache key design
2. Adjust TTL values
3. Implement cache warming for hot keys
4. Add cache statistics monitoring

### Long-term
1. Implement multi-level caching
2. Use cache-aside pattern more effectively
3. Add cache performance SLOs
4. Implement cache coherence strategies
