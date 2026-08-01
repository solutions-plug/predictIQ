# API Outage Runbook

## Alert

**Name:** `APIOutage`
**Severity:** critical
**Detection:** `up{job="predictiq-api"} == 0` for 2 minutes, or HTTP health-check
returning non-2xx for 2 minutes.
**Dashboard:** Grafana → *PredictIQ Services* → *API Health*

## Impact

All clients (frontend, third-party integrations, blockchain indexer) are unable
to reach the API. Bet placements, market queries, and payouts are unavailable.

## Immediate Mitigation (< 5 minutes)

1. Check ECS service status:
   ```bash
   aws ecs describe-services \
     --cluster predictiq-prod \
     --services predictiq-api \
     --query 'services[0].{status:status,running:runningCount,desired:desiredCount}'
   ```
2. If `runningCount == 0`, force a new deployment:
   ```bash
   aws ecs update-service \
     --cluster predictiq-prod \
     --service predictiq-api \
     --force-new-deployment
   ```
3. Check the ALB target group health:
   ```bash
   aws elbv2 describe-target-health \
     --target-group-arn <TARGET_GROUP_ARN>
   ```

## Investigation Steps

1. **Tail recent logs:**
   ```bash
   aws logs tail /ecs/predictiq-api --follow --since 10m
   ```
2. **Check for OOM kills or exit codes:**
   ```bash
   aws ecs describe-tasks \
     --cluster predictiq-prod \
     --tasks $(aws ecs list-tasks --cluster predictiq-prod --service predictiq-api \
               --desired-status STOPPED --query 'taskArns[0]' --output text) \
     --query 'tasks[0].containers[0].{exitCode:exitCode,reason:reason}'
   ```
3. **Verify database reachability** from a running task or bastion:
   ```bash
   psql $DATABASE_URL -c 'SELECT 1'
   ```
4. **Check Redis:**
   ```bash
   redis-cli -u $REDIS_URL ping
   ```
5. **Review recent deployments** — check ECS deployment history and roll back
   if the outage correlates with a new task definition.

## Escalation

- **< 5 min:** On-call engineer attempts auto-remediation above.
- **5–15 min:** Page the service owner (PagerDuty: `predictiq-api-owner`).
- **> 15 min:** Declare incident, pull in platform lead and CTO.

## Post-Incident Steps

1. Write a post-mortem within 48 hours.
2. Capture the root cause in the incident tracker.
3. Add or tune alert thresholds if detection was slow.
4. Update this runbook with any new remediation steps discovered.
