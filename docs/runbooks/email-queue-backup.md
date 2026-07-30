# Email Queue Backup Runbook

## Alert

**Name:** `EmailQueueBackup`
**Severity:** warning (→ critical if queue depth > 1 000 for > 10 min)
**Detection:** `email_queue_depth > 100` for 5 minutes.
**Dashboard:** Grafana → *PredictIQ Services* → *Email Queue*

## Impact

- Users do not receive bet confirmation, market resolution, or registration
  emails in a timely manner.
- If the queue grows unboundedly, messages older than the dead-letter TTL are
  dropped permanently.

## Immediate Mitigation (< 5 minutes)

1. Check the queue depth:
   ```bash
   aws sqs get-queue-attributes \
     --queue-url $EMAIL_QUEUE_URL \
     --attribute-names ApproximateNumberOfMessages \
                       ApproximateNumberOfMessagesNotVisible
   ```
2. Check the dead-letter queue for recent failures:
   ```bash
   aws sqs get-queue-attributes \
     --queue-url $EMAIL_DLQ_URL \
     --attribute-names ApproximateNumberOfMessages
   ```
3. Check the email worker logs:
   ```bash
   aws logs tail /ecs/predictiq-email-worker --follow --since 10m
   ```
4. If the worker is crash-looping, force a redeployment:
   ```bash
   aws ecs update-service \
     --cluster predictiq-prod \
     --service predictiq-email-worker \
     --force-new-deployment
   ```

## Investigation Steps

1. **Identify whether the queue is growing or draining:**
   - Poll `ApproximateNumberOfMessages` every 60 s for 5 minutes.
   - If growing, the worker is not consuming fast enough or is failing.
2. **Check for provider errors** (e.g., SendGrid or SES rate limiting):
   ```bash
   aws logs tail /ecs/predictiq-email-worker --since 30m | grep -i "429\|rate limit\|quota"
   ```
3. **Inspect DLQ messages** for recurring error patterns:
   ```bash
   aws sqs receive-message --queue-url $EMAIL_DLQ_URL --max-number-of-messages 10
   ```
4. **Check SES sending limits** in the AWS console: SES → Account dashboard →
   Sending statistics.

## Escalation

- **< 5 min:** On-call engineer restarts the worker.
- **5–15 min:** If provider rate-limiting is confirmed, engage the provider's
  support and consider pausing non-critical email sends.
- **> 15 min, DLQ depth > 500:** Page the platform lead; consider bulk-replaying
  DLQ messages after fixing the root cause.

## Post-Incident Steps

1. Replay the DLQ after the root cause is fixed:
   ```bash
   # Use AWS SQS DLQ Redrive or a script to move messages back to the main queue.
   aws sqs start-message-move-task \
     --source-arn $(aws sqs get-queue-attributes --queue-url $EMAIL_DLQ_URL \
                    --attribute-names QueueArn --query Attributes.QueueArn --output text)
   ```
2. Review and increase the email worker's concurrency or auto-scaling rules if
   the backup was caused by a traffic spike.
3. Update this runbook with new findings.
