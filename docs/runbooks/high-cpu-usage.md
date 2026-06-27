# High CPU Usage Runbook

## Alert Meaning
CPU usage has exceeded 80% for 10 minutes.

## Impact
- Reduced system responsiveness
- Potential service degradation
- Risk of timeouts

## Investigation Steps

1. **Check CPU usage**
   ```bash
   top -b -n 1 | head -20
   ```

2. **Identify CPU consumers**
   ```bash
   ps aux --sort=-%cpu | head -20
   ```

3. **Check for runaway processes**
   - Review application logs
   - Check for infinite loops
   - Review recent deployments

4. **Check system load**
   ```bash
   uptime
   ```

## Remediation

### Immediate Actions
1. Identify CPU-intensive processes
2. Scale up CPU if possible
3. Restart services if needed
4. Check for runaway processes

### Short-term
1. Optimize CPU usage
2. Implement CPU limits
3. Add CPU monitoring
4. Review code for inefficiencies

### Long-term
1. Implement performance profiling
2. Add performance tests to CI/CD
3. Establish CPU usage SLOs
4. Regular optimization reviews
