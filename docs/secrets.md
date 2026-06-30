# Secrets Management

All application secrets are stored in **AWS Secrets Manager** and injected into
the ECS task at startup via the `secrets` block in the task definition.  No
secret values are stored in plaintext environment variables, Terraform state
outputs, or version control.

---

## Secrets Inventory

| Short name | Secrets Manager path (pattern) | ECS env var | Type | Rotation schedule |
|---|---|---|---|---|
| `database-url` | `predictiq/<env>/database-url` | `DATABASE_URL` | Connection string | Manual (coordinate with RDS password rotation) |
| `redis-url` | `predictiq/<env>/redis-url` | `REDIS_URL` | Connection string | Manual |
| `hmac-key` | `predictiq/<env>/hmac-key` | `HMAC_KEY` | 32+ byte random hex | **Every 90 days** |
| `sendgrid-api-key` | `predictiq/<env>/sendgrid-api-key` | `SENDGRID_API_KEY` | SendGrid API key (`SG.*`) | **Every 180 days** |
| `api-signing-key` | `predictiq/<env>/api-signing-key` | `API_SIGNING_KEY` | 32+ byte random hex | **Every 90 days** |

`<env>` is one of `dev`, `staging`, or `prod`.

### Full Secrets Manager ARN pattern

```
arn:aws:secretsmanager:<region>:<account-id>:secret:predictiq/<env>/<secret-name>-<suffix>
```

---

## Architecture

```
┌─────────────────────────────────────────────────────┐
│  Terraform (infrastructure/terraform/)              │
│  ┌──────────────────────┐                          │
│  │ modules/ecs/main.tf  │  creates/updates secrets │
│  │                      │──────────────────────────┼──► AWS Secrets Manager
│  │  ECS Task Definition │  references ARNs         │
│  └──────────────────────┘                          │
└─────────────────────────────────────────────────────┘
                │  at task startup
                ▼
┌─────────────────────────────────────────────────────┐
│  ECS Fargate Task                                   │
│  ECS agent fetches secrets from Secrets Manager     │
│  and injects them as environment variables          │
│  (DATABASE_URL, REDIS_URL, HMAC_KEY, …)             │
└─────────────────────────────────────────────────────┘
```

The ECS Task **Execution Role** (`predictiq-<env>-ecs-task-execution-role`) has
`secretsmanager:GetSecretValue` permission scoped to only the five ARNs listed
above — no wildcard permissions.

---

## Rotation Procedure

### Automated helper

Use `scripts/rotate-secret.sh` to rotate any secret and trigger an ECS
rolling redeployment in one step:

```bash
# Rotate the HMAC key in production
./scripts/rotate-secret.sh hmac-key "$(openssl rand -hex 32)" prod

# Rotate the SendGrid API key (read from stdin to avoid shell history)
echo "SG.newkey..." | ./scripts/rotate-secret.sh sendgrid-api-key - staging

# Rotate the API signing key in dev
./scripts/rotate-secret.sh api-signing-key "$(openssl rand -hex 32)" dev
```

The script:
1. Validates the secret name against the known inventory.
2. Prompts for confirmation in the `prod` environment.
3. Calls `aws secretsmanager put-secret-value` to store the new value.
4. Calls `aws ecs update-service --force-new-deployment` so running tasks
   are replaced with new ones that pull the updated secret on startup.

### Prerequisites

- AWS CLI v2 with credentials that have:
  - `secretsmanager:PutSecretValue` on the target secret ARN
  - `ecs:UpdateService` on the target ECS service
- `jq` installed

### Manual rotation steps (if not using the script)

1. **Generate** a new secret value:
   ```bash
   openssl rand -hex 32   # for HMAC_KEY / API_SIGNING_KEY
   ```
2. **Store** the new value in Secrets Manager:
   ```bash
   aws secretsmanager put-secret-value \
     --secret-id "predictiq/prod/hmac-key" \
     --secret-string "<new-value>"
   ```
3. **Redeploy** the ECS service to pick up the new secret:
   ```bash
   aws ecs update-service \
     --cluster predictiq-prod \
     --service predictiq-prod-api \
     --force-new-deployment
   ```
4. **Verify** the new tasks are healthy before terminating old ones:
   ```bash
   aws ecs describe-services \
     --cluster predictiq-prod \
     --services predictiq-prod-api
   ```

---

## Rotation Schedule

| Secret | Frequency | Last rotated | Next rotation due |
|---|---|---|---|
| `hmac-key` | 90 days | See audit trail | — |
| `api-signing-key` | 90 days | See audit trail | — |
| `sendgrid-api-key` | 180 days | See audit trail | — |
| `database-url` | On-demand | See audit trail | — |
| `redis-url` | On-demand | See audit trail | — |

Update "Last rotated" and "Next rotation due" after each rotation and commit
the change to this file.

---

## Adding a New Secret

1. Add a `variable "<name>"` block in `infrastructure/terraform/variables.tf`
   with `sensitive = true`.
2. Add `aws_secretsmanager_secret` + `aws_secretsmanager_secret_version`
   resources in `infrastructure/terraform/modules/ecs/main.tf`.
3. Add the new ARN to the `secrets` block of `aws_ecs_task_definition.api`.
4. Add the new ARN to the `Resource` list in
   `aws_iam_role_policy.ecs_task_execution_secrets`.
5. Pass the variable from the root module (`infrastructure/terraform/main.tf`)
   to the ECS module.
6. Update this inventory table.

---

## Auditing Secret Access

All `GetSecretValue` calls are logged in AWS CloudTrail under
`secretsmanager.amazonaws.com` events.  Use the following query in CloudTrail
Insights or Athena to see recent accesses:

```json
{
  "eventSource": "secretsmanager.amazonaws.com",
  "eventName": "GetSecretValue",
  "requestParameters.secretId": "predictiq/prod/*"
}
```

---

## Emergency Revocation

To immediately revoke access to a secret (e.g. after a suspected compromise):

1. Rotate the secret to a new value immediately:
   ```bash
   ./scripts/rotate-secret.sh <secret-name> "$(openssl rand -hex 32)" prod
   ```
2. If the ECS service must be stopped immediately, scale it to 0:
   ```bash
   aws ecs update-service \
     --cluster predictiq-prod \
     --service predictiq-prod-api \
     --desired-count 0
   ```
3. Investigate CloudTrail logs for unauthorized `GetSecretValue` calls.
4. Once the incident is resolved, restore desired count and redeploy.
