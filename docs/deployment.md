# Deployment Guide

## ECS Rolling Deploy Strategy

PredictIQ uses ECS Fargate with a **zero-downtime rolling deploy** strategy.

### Configuration

| Setting | Value | Rationale |
|---|---|---|
| `deployment_minimum_healthy_percent` | 100 | Never reduce capacity below 100 % during a deploy |
| `deployment_maximum_percent` | 200 | Allow new tasks to start before old tasks are drained |
| `deployment_circuit_breaker.enable` | true | Automatically detect a failed deploy |
| `deployment_circuit_breaker.rollback` | true | Automatically roll back to the previous task definition on failure |

### Deploy Sequence

1. **Run database migrations** before deploying new tasks.  
   Migrations must be backward-compatible with the currently running task version so both versions can co-exist during the rollover window.

2. **ECS starts new tasks** (up to `maximum_percent` total capacity) using the new task definition.

3. **ALB health checks** confirm the new tasks are healthy (`/health` returns 200).

4. **ECS drains old tasks** — the load balancer stops routing traffic to them and waits for in-flight requests to complete before terminating.

5. **Circuit breaker monitors** the deploy. If the new tasks fail health checks repeatedly, ECS automatically rolls back to the previous stable task definition and raises a `SERVICE_DEPLOYMENT_FAILED` CloudWatch event.

### Deployment Steps (Manual)

```bash
# 1. Build and push the new image
docker build -t predictiq-api:$VERSION .
docker push $ECR_REPO:$VERSION

# 2. Run migrations against the target environment
DATABASE_URL=$PROD_DATABASE_URL ./services/api/scripts/run_migrations.sh

# 3. Update the ECS service (GitHub Actions does this automatically on merge to main)
aws ecs update-service \
  --cluster predictiq-prod \
  --service predictiq-prod-api \
  --task-definition predictiq-prod-api:$NEW_REVISION \
  --region us-east-1

# 4. Wait for the deploy to stabilise
aws ecs wait services-stable \
  --cluster predictiq-prod \
  --services predictiq-prod-api \
  --region us-east-1
```

### Rollback (Manual)

If automatic rollback did not trigger:

```bash
# Find the last known-good task definition revision
aws ecs describe-services \
  --cluster predictiq-prod \
  --services predictiq-prod-api \
  --query 'services[0].deployments'

# Force rollback to a specific revision
aws ecs update-service \
  --cluster predictiq-prod \
  --service predictiq-prod-api \
  --task-definition predictiq-prod-api:$PREVIOUS_REVISION \
  --region us-east-1
```

### AWS CodeDeploy Blue-Green — Assessment

A full blue-green deployment (via AWS CodeDeploy) would eliminate even the brief overlap window of rolling deploys by switching traffic at the ALB listener level in one atomic step.

**Warranted when:**
- Migrations are not backward-compatible and cannot be made so.
- The team needs a well-defined, instant traffic cutover with a single-click rollback.
- Compliance requirements mandate zero request failures during deploy.

**Current recommendation:** The rolling strategy with `minimum_healthy_percent = 100` already delivers zero-downtime deploys for backward-compatible migrations. CodeDeploy blue-green adds significant operational complexity (separate target groups, CodeDeploy application/deployment group resources, lifecycle hook Lambdas). Given that PredictIQ migrations are currently written to be additive and backward-compatible, the rolling strategy is sufficient. Revisit this decision if a migration arises that requires a hard cutover.

## Pre-Migration Database Snapshot

Before any deployment that includes database migrations, a snapshot of the RDS instance must be taken. This provides a rollback point if a migration causes data loss or corruption.

### Taking a Snapshot (AWS RDS)

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

### CI/CD Snapshot Step

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

## Dry-Run Mode

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

## Rolling Back a Migration

> Only roll back if the new migration caused an error. Rollback discards data written after the snapshot was taken.

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

## Snapshot Retention

Pre-migration snapshots are kept for **30 days**. Clean up old snapshots periodically:

```bash
aws rds describe-db-snapshots \
  --snapshot-type manual \
  --query "DBSnapshots[?SnapshotCreateTime<='$(date -d '30 days ago' --utc +%Y-%m-%dT%H:%M:%SZ)'].DBSnapshotIdentifier" \
  --output text | xargs -I {} aws rds delete-db-snapshot --db-snapshot-identifier {}
```

## References

- `services/api/src/migrations.rs` — migration runner implementation
- `services/api/database/migrations/` — SQL migration files
- AWS docs: [Creating a DB snapshot](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_CreateSnapshot.html)
- AWS docs: [Restoring from a DB snapshot](https://docs.aws.amazon.com/AmazonRDS/latest/UserGuide/USER_RestoreFromSnapshot.html)
