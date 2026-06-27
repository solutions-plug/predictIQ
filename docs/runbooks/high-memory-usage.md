# High Memory Usage Runbook

## Alert Meaning
Memory usage has exceeded 90% for 5 minutes.

## Impact
- Risk of OOM (Out of Memory) errors
- Service degradation
- Potential crashes

## Investigation Steps

1. **Check memory usage**
   ```bash
   free -h
   ```

2. **Identify memory consumers**
   ```bash
   ps aux --sort=-%mem | head -20
   ```

3. **Check for memory leaks**
   - Review application logs
   - Check for growing memory usage over time

4. **Check cache size**
   - Redis memory usage
   - Application cache size

## Remediation

### Immediate Actions
1. Identify and kill unnecessary processes
2. Clear caches if safe
3. Scale up memory if possible
4. Restart services if needed

### Short-term
1. Optimize memory usage
2. Implement memory limits
3. Add memory monitoring
4. Review cache policies

### Long-term
1. Implement memory profiling
2. Add memory tests to CI/CD
3. Establish memory SLOs
4. Regular optimization reviews
