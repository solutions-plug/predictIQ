# Service Down Runbook

## Alert Meaning
A PredictIQ service has been down for more than 1 minute.

## Impact
- Service unavailable to users
- Potential data loss
- Revenue impact

## Investigation Steps

1. **Check service status**
   ```bash
   kubectl get pods -l app=predictiq-api
   ```

2. **Check service logs**
   ```bash
   kubectl logs -l app=predictiq-api --tail=100
   ```

3. **Check recent events**
   ```bash
   kubectl describe pod <pod-name>
   ```

4. **Check infrastructure**
   - Node status
   - Resource availability
   - Network connectivity

## Remediation

### Immediate Actions
1. **Page on-call engineer** - Critical issue
2. Check pod status and restart if needed
3. Check node health
4. Review recent deployments

### Emergency Actions
1. Rollback recent deployment if applicable
2. Scale up replicas
3. Check for resource constraints
4. Failover to backup if available

### Post-Incident
1. Root cause analysis
2. Implement monitoring improvements
3. Add health checks
4. Improve deployment process
