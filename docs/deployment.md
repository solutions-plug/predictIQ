# Deployment Guide

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
