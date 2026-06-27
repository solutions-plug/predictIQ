# Database Schema Documentation

This service uses PostgreSQL. Schema and seed scripts are in:

- `services/api/database/migrations/`
- `services/api/database/seeds/`

## Tables

- `newsletter_subscribers` — email opt-in list with double-opt-in confirmation
- `contact_form_submissions`
- `waitlist_entries`
- `analytics_events`
- `content_management`
- `audit_logs` — general audit trail (UUID primary key)
- `audit_log` — append-only admin-operation audit log (bigserial primary key)
- `email_jobs` — async email queue tracking
- `markets` — on-chain market mirror

## Migration Files

| # | File | Description |
|---|------|-------------|
| 000 | `000_create_schema_migrations.sql` | Migration tracking table |
| 001 | `001_enable_pgcrypto.sql` | Enable pgcrypto extension |
| 002 | `002_create_newsletter_subscriptions.sql` | `newsletter_subscribers` table |
| 003 | `003_create_contact_form_submissions.sql` | `contact_form_submissions` table |
| 004 | `004_create_waitlist_entries.sql` | `waitlist_entries` table |
| 005 | `005_create_content_management.sql` | `content_management` table |
| 006 | `006_create_analytics_events.sql` | `analytics_events` table |
| 007 | `007_create_audit_logs.sql` | `audit_logs` table (UUID PK) |
| 008 | `008_create_email_tracking.sql` | Email jobs, events, suppressions, templates, analytics |
| 009 | `009_add_newsletter_indexes.sql` | Performance indexes on `newsletter_subscribers` |
| 010a | `010_add_soft_delete_newsletter.sql` | Adds `deleted_at` to `newsletter_subscribers` |
| 010b | `010_create_audit_log.sql` | Append-only `audit_log` table (bigserial PK) |
| 011 | `011_create_markets.sql` | `markets` table |
| 012 | `012_add_performance_indexes.sql` | Composite indexes on `markets` and `content` (promoted from `sql/`) |

> **Note:** Two migration files share the `010_` prefix. Apply them in lexicographic
> order (`010_add_soft_delete_newsletter.sql` before `010_create_audit_log.sql`).

## Apply Migrations

Run from the workspace root:

```bash
for f in services/api/database/migrations/*.sql; do
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$f"
done
```

Or use the provided script:

```bash
bash services/api/scripts/run_migrations.sh
```

## Rollback Procedure

Each migration has a corresponding rollback (down) script in
`services/api/database/migrations/rollbacks/`. Rollback scripts reverse the
exact changes made by their paired migration.

### Rollback scripts

| Migration | Rollback script |
|-----------|----------------|
| `000_create_schema_migrations.sql` | `rollbacks/000_create_schema_migrations_down.sql` |
| `001_enable_pgcrypto.sql` | `rollbacks/001_enable_pgcrypto_down.sql` |
| `002_create_newsletter_subscriptions.sql` | `rollbacks/002_create_newsletter_subscriptions_down.sql` |
| `003_create_contact_form_submissions.sql` | `rollbacks/003_create_contact_form_submissions_down.sql` |
| `004_create_waitlist_entries.sql` | `rollbacks/004_create_waitlist_entries_down.sql` |
| `005_create_content_management.sql` | `rollbacks/005_create_content_management_down.sql` |
| `006_create_analytics_events.sql` | `rollbacks/006_create_analytics_events_down.sql` |
| `007_create_audit_logs.sql` | `rollbacks/007_create_audit_logs_down.sql` |
| `008_create_email_tracking.sql` | `rollbacks/008_create_email_tracking_down.sql` |
| `009_add_newsletter_indexes.sql` | `rollbacks/009_add_newsletter_indexes_down.sql` |
| `010_add_soft_delete_newsletter.sql` | `rollbacks/010_add_soft_delete_newsletter_down.sql` |
| `010_create_audit_log.sql` | `rollbacks/010_create_audit_log_down.sql` |
| `011_create_markets.sql` | `rollbacks/011_create_markets_down.sql` |

### Rolling back a single migration

```bash
# 1. Apply the down script
psql "$DATABASE_URL" -v ON_ERROR_STOP=1 \
  -f services/api/database/migrations/rollbacks/<version>_down.sql

# 2. Remove the version record from the tracking table
psql "$DATABASE_URL" -c \
  "DELETE FROM schema_migrations WHERE version = '<version>';"

# 3. Verify — the version should now appear as pending
bash services/api/scripts/run_migrations.sh --status
```

### Rolling back multiple migrations

Apply rollback scripts in **reverse order** (highest version first):

```bash
for f in $(ls services/api/database/migrations/rollbacks/*_down.sql | sort -r); do
  echo "Rolling back: $f"
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$f"
done
```

### Dependency order for rollbacks

When rolling back multiple migrations, respect foreign-key dependencies:

1. `011` → `010b` → `010a` → `009` → `008` → `007` → `006` → `005` → `004` → `003` → `002` → `001` → `000`

`analytics_events` (006) references `content_management` (005), and
`audit_logs` (007) references several earlier tables — roll them back before
their dependencies.

### Emergency: drop and recreate schema

Only in **development or staging** when data loss is acceptable:

```bash
psql "$DATABASE_URL" -c "DROP SCHEMA public CASCADE; CREATE SCHEMA public;"
bash services/api/scripts/run_migrations.sh
```

## sql/ Directory

`services/api/sql/` contains **query templates and ad-hoc reference SQL** — not schema migrations.

| File | Purpose |
|---|---|
| `performance_indexes.sql` | Source for the indexes now in `012_add_performance_indexes.sql`. Kept as a reference; do not apply manually. |
| `newsletter_schema.sql` | Early draft of the `newsletter_subscribers` schema. Superseded by `002_create_newsletter_subscriptions.sql`. Do not apply manually. |

> **Rule:** No schema-altering SQL should be applied from `sql/` directly. All schema changes must go through a numbered migration in `database/migrations/`.

## Connection Pool Configuration

Pool sizing and timeouts are fully env-configurable — no code changes needed for different deployment sizes.

| Variable | Default | Description |
|---|---|---|
| `DB_POOL_MIN_CONNECTIONS` | `5` | Minimum idle connections kept open |
| `DB_POOL_MAX_CONNECTIONS` | `25` | Maximum concurrent connections |
| `DB_POOL_ACQUIRE_TIMEOUT_SECS` | `5` | Seconds to wait for a free connection before error |
| `DB_POOL_IDLE_TIMEOUT_SECS` | _(sqlx default)_ | Seconds before idle connections are reaped (0 = disabled) |
| `DB_POOL_MAX_LIFETIME_SECS` | _(sqlx default)_ | Max lifetime of a connection in seconds (0 = disabled) |
| `DB_QUERY_TIMEOUT_SECS` | `30` | Per-query execution timeout; queries exceeding this return an error |

**Sizing guidance:**
- Small / dev: `DB_POOL_MIN_CONNECTIONS=2 DB_POOL_MAX_CONNECTIONS=5`
- Medium: `DB_POOL_MIN_CONNECTIONS=5 DB_POOL_MAX_CONNECTIONS=25` (default)
- Large / high-traffic: `DB_POOL_MIN_CONNECTIONS=10 DB_POOL_MAX_CONNECTIONS=100`

Pool metrics are exposed on the `/metrics` Prometheus endpoint under the `db_pool_*` family.

## Apply Migrations

Run from the workspace root:

```bash
for f in services/api/database/migrations/*.sql; do
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$f"
done
```

Or use the provided script:

```bash
bash services/api/scripts/run_migrations.sh
```

## Seeding

```bash
for f in services/api/database/seeds/*.sql; do
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$f"
done
```

## Backup Strategy

- Daily logical backups with `pg_dump`, 30-day retention.
- Weekly full snapshot, 90-day retention.
- Quarterly restore drills in staging.
- Encrypt backup storage at rest.

## Data Retention Policy

- `analytics_events`: 13 months raw, then archive/aggregate.
- `audit_logs` / `audit_log`: 24 months minimum for compliance.
- `contact_form_submissions`: 12 months unless legal hold.
- `newsletter_subscribers` / `waitlist_entries`: retain active records; hard-delete on GDPR request.

## Notes

- UUID primary keys via `gen_random_uuid()` (most tables); `audit_log` uses `BIGSERIAL`.
- All tables include `created_at` / `updated_at` timestamps.
- Soft deletes via `deleted_at` in `content_management`, `audit_logs`, and `newsletter_subscribers`.
- Indexes on high-frequency query fields (`email`, `status`, `created_at`).
