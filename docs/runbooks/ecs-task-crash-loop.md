# ECS Task Crash Loop Runbook

## Alert

**Name:** `ECSTaskCrashLoop`
**Severity:** critical
**Detection:** ECS service `runningCount` stays below `desiredCount` for more
than 3 minutes because tasks exit immediately after launch.
**Dashboard:** Grafana → *PredictIQ Services* → *ECS Tasks*

## Impact

Depends on which service is crash-looping:
- `predictiq-api` — full API outage (see also: [api-outage.md](./api-outage.md))
- `predictiq-indexer` — blockchain event ingestion stops
- `predictiq-email-worker` — email delivery queues up (see: [email-queue-backup.md](./email-queue-backup.md))

## Immediate Mitigation (< 5 minutes)

1. Identify which service is affected:
   ```bash
   aws ecs list-services --cluster predictiq-prod --output text | xargs -I{} \
     aws ecs describe-services --cluster predictiq-prod --services {} \
     --query 'services[?runningCount < desiredCount].[serviceName,runningCount,desiredCount]'
   ```
2. Describe stopped tasks to get the exit code and reason:
   ```bash
   SERVICE=predictiq-api  # replace with affected service
   TASK_ARN=$(aws ecs list-tasks --cluster predictiq-prod \
                --service-name $SERVICE --desired-status STOPPED \
                --query 'taskArns[0]' --output text)
   aws ecs describe-tasks --cluster predictiq-prod --tasks $TASK_ARN \
     --query 'tasks[0].containers[0].{exit:exitCode,reason:reason,status:lastStatus}'
   ```
3. Check recent logs for the fatal error:
   ```bash
   aws logs tail /ecs/$SERVICE --since 15m | tail -100
   ```

## Common Causes and Fixes

### Exit code 1 — application panic / unhandled error at startup
- Check logs for `FATAL`, `panic`, or `error` at process start.
- Common culprits: missing environment variables, bad secret ARN, schema
  migration failure.
- Fix: correct the env/secrets and redeploy.

### Exit code 137 — OOM kill
- The task ran out of memory.
- Fix: increase the task `memory` reservation, or identify and fix a memory
  leak, then redeploy.

### Exit code 139 — segfault (native crash)
- Rare in Go/Rust services. Check for a recent native dependency change.
- Roll back the task definition to the last known-good revision.

### Container health-check failure (ECS stops after `healthCheckGracePeriodSeconds`)
- The container started but failed its health check (e.g., `/health` endpoint
  not responding in time).
- Check if the service needs more time to initialize; increase
  `healthCheckGracePeriodSeconds` as a short-term measure.

### Bad task definition / secret injection failure
- If `reason` contains `CannotPullContainerError` or `ResourceInitializationError`,
  the container image pull or secret injection failed.
- Verify the ECR image tag exists and IAM permissions for Secrets Manager are
  correct.

## Rolling Back a Deployment

```bash
# List recent task definition revisions
aws ecs list-task-definitions --family-prefix predictiq-api --sort DESC | head -5

# Update service to the previous revision
aws ecs update-service \
  --cluster predictiq-prod \
  --service predictiq-api \
  --task-definition predictiq-api:<PREVIOUS_REVISION>
```

## Escalation

- **< 5 min:** On-call engineer diagnoses exit code and attempts quick fix or
  rollback.
- **5–15 min:** If the root cause is unclear, page the service owner
  (PagerDuty: `predictiq-<service>-owner`).
- **> 15 min with no fix:** Declare incident; engage platform lead.

## Post-Incident Steps

1. Verify the service stabilised (`runningCount == desiredCount` for 5+ min).
2. Capture the root cause in the incident tracker.
3. Add a startup probe or improve health-check timeouts if the crash was caused
   by a slow initialisation.
4. Update this runbook with new findings.
