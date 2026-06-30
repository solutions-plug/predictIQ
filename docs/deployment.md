# PredictIQ — Production Deployment Guide

This guide covers every step from provisioning infrastructure from scratch to running day-to-day deployments, database migrations, rollbacks, and environment variable management.

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [First-time Infrastructure Provisioning](#2-first-time-infrastructure-provisioning)
3. [Environment Variable Management](#3-environment-variable-management)
4. [Normal Deployment Workflow](#4-normal-deployment-workflow)
5. [Database Migration Process](#5-database-migration-process)
6. [Rollback Procedure](#6-rollback-procedure)
7. [Validation Checklist](#7-validation-checklist)

---

## 1. Prerequisites

Install and configure the following tools before proceeding.

| Tool | Version | Install |
|---|---|---|
| AWS CLI | ≥ 2.x | `brew install awscli` / [aws.amazon.com/cli](https://aws.amazon.com/cli/) |
| Terraform | ≥ 1.5.0 | `brew install terraform` |
| Docker | ≥ 24 | [docs.docker.com](https://docs.docker.com/get-docker/) |
| Rust / Cargo | stable | `curl https://sh.rustup.rs -sSf \| sh` |
| psql (PostgreSQL client) | ≥ 14 | `brew install libpq` |
| GitHub CLI (`gh`) | ≥ 2.x | `brew install gh` |

### AWS credentials

Configure an AWS profile with permission to assume the deployment IAM roles:

```bash
aws configure --profile predictiq
# Enter: Access Key ID, Secret Access Key, us-east-1, json
```

Then verify access:

```bash
aws sts get-caller-identity --profile predictiq
```

### Required GitHub repository secrets

The deployment workflow reads the following secrets. Set them under **Settings → Secrets → Actions** in the repository:

| Secret | Description |
|---|---|
| `AWS_ROLE_DEV` | IAM role ARN for the dev environment |
| `AWS_ROLE_STAGING` | IAM role ARN for the staging environment |
| `AWS_ROLE_PROD` | IAM role ARN for the production environment |

---

## 2. First-time Infrastructure Provisioning

Run this section **once** per environment. Skip it for subsequent deployments.

### Step 1 — Bootstrap Terraform remote state

The bootstrap script creates the S3 bucket and DynamoDB table that Terraform uses for remote state storage and locking.

```bash
cd infrastructure/terraform

# Bootstrap for production (repeat with dev/staging as needed)
./bootstrap.sh us-east-1 prod
```

Expected output: bucket name, DynamoDB table name, and confirmation that versioning and encryption are enabled.

### Step 2 — Initialise Terraform

```bash
cd infrastructure/terraform
terraform init -backend-config=environments/production/backend.hcl
```

### Step 3 — Plan and review

```bash
terraform plan -var-file="environments/production/terraform.tfvars"
```

Review the plan output carefully. Confirm:
- A new VPC with public and private subnets
- An RDS PostgreSQL instance in the private subnet
- An ElastiCache Redis cluster in the private subnet
- An ECS cluster + Fargate service for the API
- An Application Load Balancer in the public subnet
- A CloudWatch monitoring module

### Step 4 — Apply

```bash
terraform apply -var-file="environments/production/terraform.tfvars"
```

Confirm with `yes` when prompted. The first apply takes approximately 15–25 minutes.

### Step 5 — Retrieve outputs

```bash
terraform output
```

Note the following values — you will need them for environment variable configuration:

| Output | Usage |
|---|---|
| `database_url` | `DATABASE_URL` for the API service |
| `redis_url` | `REDIS_URL` for the API service |
| `alb_dns_name` | Configure DNS CNAME to this value |
| `ecs_cluster_name` | Used for ECS deployment commands |
| `ecs_service_name` | Used for ECS deployment commands |

### Step 6 — Run database migrations (first time)

Connect to the RDS instance via the bastion or AWS RDS Proxy (see [Database Migration Process](#5-database-migration-process)).

```bash
DATABASE_URL="postgres://user:password@<rds-endpoint>:5432/predictiq" \
  bash services/api/scripts/run_migrations.sh
```

### Step 7 — Configure AWS Secrets Manager

Store secrets that the ECS task reads at runtime:

```bash
# Create the production secret bundle
aws secretsmanager create-secret \
  --name predictiq/prod/api \
  --secret-string '{
    "DATABASE_URL": "postgres://...",
    "REDIS_URL": "rediss://...",
    "SENDGRID_API_KEY": "SG.xxx",
    "HMAC_KEY": "<32-byte-hex>",
    "UNSUBSCRIBE_SIGNING_SECRET": "<secret>",
    "PREDICTIQ_CONTRACT_ID": "<stellar-contract-id>",
    "API_KEYS": "<key1>,<key2>"
  }' \
  --profile predictiq
```

---

## 3. Environment Variable Management

The API service is configured entirely via environment variables. Variables are stored in AWS Secrets Manager and injected into ECS task definitions at deploy time.

### Required variables (production)

| Variable | Description | Example |
|---|---|---|
| `DATABASE_URL` | PostgreSQL connection string | `postgres://user:pass@host:5432/predictiq` |
| `REDIS_URL` | Redis connection string | `rediss://host:6379` |
| `HMAC_KEY` | 32-byte hex secret for request signing | `deadbeef...` |
| `API_KEYS` | Comma-separated admin API keys | `key1,key2` |
| `SENDGRID_API_KEY` | SendGrid API key | `SG.xxx` |
| `FROM_EMAIL` | Sender address for transactional email | `noreply@predictiq.com` |
| `BASE_URL` | Public base URL of the API | `https://api.predictiq.com` |
| `UNSUBSCRIBE_SIGNING_SECRET` | HMAC secret for unsubscribe tokens | `<random-32-chars>` |
| `PREDICTIQ_CONTRACT_ID` | Stellar/Soroban contract address | `C...` |
| `BLOCKCHAIN_RPC_URL` | Stellar Horizon / Soroban RPC endpoint | `https://soroban-testnet.stellar.org` |
| `STELLAR_NETWORK_PASSPHRASE` | Stellar network passphrase | `Test SDF Network ; September 2015` |

### Optional / tuning variables

| Variable | Default | Description |
|---|---|---|
| `API_BIND_ADDR` | `0.0.0.0:8080` | Bind address |
| `CORS_ALLOWED_ORIGINS` | *(empty — blocks cross-origin)* | Comma-separated origins |
| `DB_POOL_MIN_CONNECTIONS` | `2` | Minimum DB pool connections |
| `DB_POOL_MAX_CONNECTIONS` | `10` | Maximum DB pool connections |
| `FEATURED_LIMIT` | `10` | Max featured markets returned |
| `NEWSLETTER_TOKEN_TTL_SECS` | `86400` | Confirmation token expiry |
| `OTLP_ENDPOINT` | *(none)* | OpenTelemetry collector endpoint |
| `TRACE_SAMPLE_RATE` | `0.1` | Fraction of requests traced (0–1) |
| `SENDGRID_WEBHOOK_SECRET` | *(none)* | SendGrid webhook signature secret |
| `ADMIN_WHITELIST_IPS` | *(none — admin routes unrestricted)* | Comma-separated CIDR allowlist |

### Updating a secret

```bash
aws secretsmanager update-secret \
  --secret-id predictiq/prod/api \
  --secret-string '{"SENDGRID_API_KEY": "SG.new-value", ...}' \
  --profile predictiq
```

After updating secrets, force a new ECS deployment to pick up the changes (see [Step 4](#step-4--force-ecs-deployment) below).

---

## 4. Normal Deployment Workflow

A normal deployment pushes a new container image and triggers a rolling ECS update.

### Step 1 — Merge to main

All deployments start with merging a PR to `main`. GitHub Actions (`.github/workflows/docker.yml`) automatically builds and pushes a tagged image to GHCR:

```
ghcr.io/<org>/predictiq:<git-sha>
ghcr.io/<org>/predictiq:main
ghcr.io/<org>/predictiq:latest
```

### Step 2 — Update the image URI in Terraform (if pinning a specific SHA)

Edit `infrastructure/terraform/environments/production/terraform.tfvars`:

```hcl
api_image_uri = "ghcr.io/<org>/predictiq:<new-sha>"
```

Commit and push. The Terraform deployment workflow applies automatically on push to `main`.

### Step 3 — Monitor the ECS deployment

```bash
# Watch the rolling deployment
aws ecs describe-services \
  --cluster predictiq-prod \
  --services predictiq-api \
  --profile predictiq \
  --query 'services[0].deployments'

# Stream ECS task logs
aws logs tail /ecs/predictiq-prod/api --follow --profile predictiq
```

### Step 4 — Force ECS deployment (for config-only changes)

If you changed secrets but not the image, force a redeployment:

```bash
aws ecs update-service \
  --cluster predictiq-prod \
  --service predictiq-api \
  --force-new-deployment \
  --profile predictiq
```

### Step 5 — Verify the deployment

```bash
curl -s https://api.predictiq.com/health | jq .
```

Expected response:
```json
{
  "status": "ok",
  "redis": { "circuit_breaker": "closed" },
  "db": { "status": "ok" }
}
```

---

## 5. Database Migration Process

Database migrations are SQL files in `services/api/database/migrations/`. Each migration has a corresponding rollback script in `services/api/database/migrations/rollbacks/`.

### Pre-Migration Database Snapshot

Before any deployment that includes database migrations, a snapshot of the RDS instance must be taken. This provides a rollback point if a migration causes data loss or corruption.

#### Taking a Snapshot (AWS RDS)

```bash
aws rds create-db-snapshot \
  --db-instance-identifier <your-db-instance-id> \
  --db-snapshot-identifier "pre-migration-$(date +%Y%m%d%H%M%S)" \
  --region us-east-1
```

Wait for the snapshot to complete before proceeding:

```bash
aws rds wait db-snapshot-completed \
  --db-snapshot-identifier "pre-migration-<timestamp>" \
  --region us-east-1
```

If the `wait` command exits with a non-zero status, **do not proceed with the deployment**. Investigate the snapshot failure before continuing.

#### CI/CD Snapshot Step

In the deployment pipeline, add a snapshot step **before** the migration step and gate migration execution on its success:

```yaml
- name: Take pre-migration RDS snapshot
  run: |
    SNAPSHOT_ID="pre-migration-${GITHUB_SHA:0:8}-$(date +%Y%m%d%H%M%S)"
    aws rds create-db-snapshot \
      --db-instance-identifier "$RDS_INSTANCE_ID" \
      --db-snapshot-identifier "$SNAPSHOT_ID" \
      --region "$AWS_REGION"
    echo "SNAPSHOT_ID=$SNAPSHOT_ID" >> "$GITHUB_ENV"

- name: Wait for snapshot to complete
  run: |
    aws rds wait db-snapshot-completed \
      --db-snapshot-identifier "$SNAPSHOT_ID" \
      --region "$AWS_REGION"

- name: Run migrations
  run: ./scripts/run-migrations.sh
  env:
    DATABASE_URL: ${{ secrets.DATABASE_URL }}
```

### Applying migrations

Migrations must be applied **before** deploying a new API version that requires them.

```bash
# Export the production database URL (get from Secrets Manager)
export DATABASE_URL=$(aws secretsmanager get-secret-value \
  --secret-id predictiq/prod/api \
  --profile predictiq \
  --query 'SecretString' \
  --output text | python3 -c "import json,sys; print(json.load(sys.stdin)['DATABASE_URL'])")

# Apply all pending migrations
bash services/api/scripts/run_migrations.sh
```

To apply a single migration manually:

```bash
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
  -f services/api/database/migrations/011_create_markets.sql
```

### Dry-Run Mode

The migration runner supports a dry-run mode that prints the SQL for all pending migrations without executing anything. Use this to audit pending changes before a production deployment.

```bash
MIGRATE_DRY_RUN=true ./your-api-binary
```

Output example:

```
-- [DRY RUN] migration 008 (008_create_email_tracking)
CREATE TABLE IF NOT EXISTS email_tracking ( ... );

[DRY RUN] migration run complete — no changes applied (pending=1)
```

Dry-run mode is safe to run against production databases — it only reads the `schema_migrations` table to determine which migrations are pending.

### Checking migration status

```bash
psql "$DATABASE_URL" -c "SELECT version, applied_at FROM schema_migrations ORDER BY version;"
```

### Writing a new migration

1. Create `services/api/database/migrations/NNN_description.sql`
2. Create `services/api/database/migrations/rollbacks/NNN_description_down.sql` (required — CI validates this)
3. Test the migration and rollback locally against a dev database
4. Include both files in the PR

### Snapshot Retention

Pre-migration snapshots are kept for **30 days**. Clean up old snapshots periodically:

```bash
aws rds describe-db-snapshots \
  --snapshot-type manual \
  --query "DBSnapshots[?SnapshotCreateTime<='$(date -d '30 days ago' --utc +%Y-%m-%dT%H:%M:%SZ)'].DBSnapshotIdentifier" \
  --output text | xargs -I {} aws rds delete-db-snapshot --db-snapshot-identifier {}
```

---

## 6. Rollback Procedure

### Rolling back the application (code only)

```bash
# Option A — revert the merge commit and push; CI redeploys the previous image
git revert <merge-commit-sha>
git push origin main

# Option B — force-redeploy a previous image without a code change
aws ecs update-service \
  --cluster predictiq-prod \
  --service predictiq-api \
  --task-definition predictiq-api:<previous-revision> \
  --force-new-deployment \
  --profile predictiq
```

### Rolling back a database migration

Each migration has a paired rollback script. Apply in reverse order (highest version first):

```bash
# Roll back migration 012
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
  -f services/api/database/migrations/rollbacks/012_add_performance_indexes_down.sql

# Remove the version record so the migration is treated as pending again
psql "$DATABASE_URL" -c "DELETE FROM schema_migrations WHERE version = '012';"
```

If a migration caused data corruption, you can restore from the pre-migration snapshot:

1. Identify the snapshot taken before the failed deployment:
   ```bash
   aws rds describe-db-snapshots \
     --query "DBSnapshots[?starts_with(DBSnapshotIdentifier,'pre-migration')]|sort_by(@,&SnapshotCreateTime)[-5:]" \
     --region us-east-1
   ```

2. Restore the snapshot to a new instance for validation:
   ```bash
   aws rds restore-db-instance-from-db-snapshot \
     --db-instance-identifier <restore-instance-id> \
     --db-snapshot-identifier <snapshot-id> \
     --region us-east-1
   ```

3. After validating the restore, update DNS/connection strings to point to the restored instance, or swap the identifier if using RDS renaming.

4. Delete the failed instance once traffic has been moved.

For more detail, see [`services/api/database/migrations/ROLLBACK.md`](../services/api/database/migrations/ROLLBACK.md).

### Rolling back infrastructure

See [`infrastructure/ROLLBACK.md`](../infrastructure/ROLLBACK.md) for full procedures including Terraform state restoration.

Quick path — revert via git:

```bash
git revert <terraform-commit-sha>
git push origin main
# GitHub Actions applies the reverted Terraform plan automatically
```

### Rollback timeline targets

| Scope | Target time |
|---|---|
| Application (ECS rolling update) | < 5 minutes |
| Config-only (Secrets Manager + force redeploy) | < 3 minutes |
| Database migration rollback | 5–15 minutes (depends on data volume) |
| Full infrastructure rollback (Terraform) | 15–30 minutes |

---

## 7. Validation Checklist

Run these checks after every production deployment:

```bash
# 1. Health endpoint
curl -sf https://api.predictiq.com/health | jq .

# 2. Statistics (exercises DB + cache)
curl -sf https://api.predictiq.com/api/v1/statistics | jq .

# 3. Featured markets (exercises blockchain + cache)
curl -sf https://api.predictiq.com/api/v1/markets/featured | jq .

# 4. Blockchain health
curl -sf https://api.predictiq.com/api/v1/blockchain/health | jq .

# 5. CloudWatch alarms — confirm no alarms are in ALARM state
aws cloudwatch describe-alarms \
  --state-value ALARM \
  --profile predictiq \
  --query 'MetricAlarms[].AlarmName'

# 6. ECS service stabilized
aws ecs describe-services \
  --cluster predictiq-prod \
  --services predictiq-api \
  --profile predictiq \
  --query 'services[0].{desired:desiredCount,running:runningCount,pending:pendingCount}'
```

All six checks should pass within 5 minutes of the deployment completing.

---

## References

- `services/api/src/migrations.rs` — migration runner implementation
- `services/api/database/migrations/` — SQL migration files
- AWS docs: [Creating a DB snapshot](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_CreateSnapshot.html)
- AWS docs: [Restoring from a DB snapshot](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_RestoreFromSnapshot.html)

---

> **Validate this guide:** After any significant change to the deployment process, have a team member who was not involved in the change follow it end-to-end in the staging environment and record the outcome here.
>
> | Date | Follower | Environment | Issues found | PR |
> |---|---|---|---|---|
> | *(not yet validated)* | | | | |
