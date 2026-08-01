# SendGrid API Key Rotation Runbook

## Overview

The SendGrid API key (`SENDGRID_API_KEY`) is stored in AWS Secrets Manager and
injected into the ECS task at deploy time via the `SENDGRID_API_KEY` secret.
A companion secret (`SENDGRID_KEY_ROTATED_AT`) records the date (YYYY-MM-DD)
the key was last rotated. The application reads this at startup and emits a
`SECURITY WARNING` log if the key is **90 days or older**.

Rotate the key proactively every 90 days, or immediately if compromise is
suspected.

---

## When to Rotate

| Trigger | Action |
|---------|--------|
| Startup warning: key ≥ 90 days old | Scheduled rotation (follow steps below) |
| Suspected key compromise or leak | Emergency rotation (follow steps below + revoke old key first) |
| Staff offboarding (key holder) | Rotate within 24 hours |
| Security audit finding | Rotate within the remediation SLA |

---

## Rotation Steps

### 1. Generate a new API key in SendGrid

1. Log into [SendGrid](https://app.sendgrid.com) with your organisation account.
2. Navigate to **Settings → API Keys → Create API Key**.
3. Set the permissions to **Restricted Access**:
   - **Mail Send** → Full Access
   - All others → No Access
4. Copy the new key value (it is only shown once).

### 2. Update the secret in AWS Secrets Manager

```bash
# Set these for your environment (prod / staging / dev)
ENVIRONMENT=prod
NEW_KEY="SG.xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
ROTATION_DATE=$(date +%Y-%m-%d)

# Update the API key secret
aws secretsmanager put-secret-value \
  --secret-id "predictiq/${ENVIRONMENT}/sendgrid-api-key" \
  --secret-string "${NEW_KEY}"

# Update the rotation-date metadata secret
aws secretsmanager put-secret-value \
  --secret-id "predictiq/${ENVIRONMENT}/sendgrid-key-rotated-at" \
  --secret-string "${ROTATION_DATE}"
```

### 3. Trigger a rolling ECS deployment to pick up the new secret

ECS injects secrets at task launch time, so a new deployment is required:

```bash
# Force a new deployment (no image change needed)
aws ecs update-service \
  --cluster predictiq-${ENVIRONMENT} \
  --service predictiq-${ENVIRONMENT}-api \
  --force-new-deployment
```

Monitor the deployment:

```bash
aws ecs wait services-stable \
  --cluster predictiq-${ENVIRONMENT} \
  --services predictiq-${ENVIRONMENT}-api
echo "Deployment complete"
```

### 4. Verify the new key is active

```bash
# Check startup logs for the age-check info line (should show age_days = 0)
aws logs filter-log-events \
  --log-group-name "/ecs/predictiq-${ENVIRONMENT}" \
  --filter-pattern "SendGrid API key age check passed" \
  --start-time $(date -d '5 minutes ago' +%s000)

# Send a test email via the API (adjust endpoint / auth as needed)
curl -sf -X POST https://api.predictiq.com/api/v1/newsletter/subscribe \
  -H "Content-Type: application/json" \
  -d '{"email": "rotation-test@example.com"}' \
  && echo "Test subscription queued"
```

### 5. Revoke the old key in SendGrid

1. Return to **Settings → API Keys** in SendGrid.
2. Find the old key (by name/date), click the action menu, and select **Delete**.
3. Confirm deletion.

> **Never leave both the old and new key active longer than needed.** Revoke
> the old key as soon as the new deployment is stable.

### 6. Update Terraform state (if managing via Terraform)

If the `sendgrid_api_key` variable is managed in a `.tfvars` file or CI/CD
secret store, update it to the new value so the next `terraform apply` does not
overwrite the secret you just set manually.

```bash
# Example: update the secret in your CI/CD secret store, then run:
terraform apply -var="sendgrid_api_key=${NEW_KEY}" \
                -var="sendgrid_key_rotated_at=${ROTATION_DATE}"
```

---

## Emergency Rotation (Key Compromised)

If you believe the key has been leaked or is actively being misused:

1. **Immediately** delete the compromised key in the SendGrid dashboard
   (step 5 above) **before** generating a new one.
2. Check SendGrid **Activity Feed** for unexpected sends from the compromised key.
3. Follow steps 1–6 above to issue a new key.
4. File a security incident and notify the security team.

---

## Rollback

If the new key does not work (e.g. email delivery failures after deployment):

1. Restore the old secret value from Secrets Manager version history:
   ```bash
   # List versions to find the previous AWSPREVIOUS stage
   aws secretsmanager list-secret-version-ids \
     --secret-id "predictiq/${ENVIRONMENT}/sendgrid-api-key"

   # Restore the previous version (replace VERSION_ID)
   aws secretsmanager update-secret-version-stage \
     --secret-id "predictiq/${ENVIRONMENT}/sendgrid-api-key" \
     --version-stage AWSCURRENT \
     --move-to-version-id <PREVIOUS_VERSION_ID> \
     --remove-from-version-id <CURRENT_VERSION_ID>
   ```
2. Force a new ECS deployment to pick up the restored secret.
3. Investigate why the new key failed (wrong permissions, copy-paste error, etc.).

---

## Automation / Future Improvements

- Enable [Secrets Manager automatic rotation](https://docs.aws.amazon.com/secretsmanager/latest/userguide/rotating-secrets.html)
  with a Lambda rotation function once SendGrid's key API is integrated.
- Add a CloudWatch alarm on the `SECURITY WARNING` log pattern so PagerDuty
  fires automatically when the key exceeds 90 days.
- Consider storing a key fingerprint (SHA-256 of the key prefix) alongside
  `SENDGRID_KEY_ROTATED_AT` to detect out-of-band key changes.

---

## Related

- [SendGrid API Key documentation](https://docs.sendgrid.com/ui/account-and-settings/api-keys)
- `infrastructure/terraform/modules/ecs/main.tf` — Secrets Manager resources
- `services/api/src/config.rs` — `warn_if_sendgrid_key_stale()`
- Security issue: [#893](https://github.com/solutions-plug/predictIQ/issues/893)
