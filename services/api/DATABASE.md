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
- `audit_log` — the single audit table; append-only admin-operation audit log (bigserial primary key)
- `email_jobs` — async email queue tracking
- `markets` — on-chain market mirror
- `watched_transactions` — user watch records for on-chain transactions

## Audit Logging

`audit_log` is the only audit table. It is **append-only**: rows are inserted by
admin operations and are never updated or soft-deleted. The former `audit_logs`
table (UUID primary key, soft-deletable) was unused and has been removed; see the
migration note below. GDPR logging tests target `audit_log`.

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
| 007 | `007_create_audit_logs.sql` | Legacy `audit_logs` table (UUID PK) — removed, see migration note |
| 008 | `008_create_email_tracking.sql` | Email jobs, events, suppressions, templates, analytics |
| 009 | `009_add_newsletter_indexes.sql` | Performance indexes on `newsletter_subscribers` |
| 010a | `010_add_soft_delete_newsletter.sql` | Adds `deleted_at` to `newsletter_subscribers` |
| 010b | `010_create_audit_log.sql` | Append-only `audit_log` table (bigserial PK) |
| 011 | `011_create_markets.sql` | `markets` table |
| 012 | `012_add_performance_indexes.sql` | Composite indexes on `markets` and `content` (promoted from `sql/`) |
| 013 | `013_add_email_queue_composite_index.sql` | Composite index for priority-ordered email queue scans |
| 014 | `014_add_email_events_composite_index.sql` | Composite index for ordered email event lookups by job |
| 015 | `015_add_newsletter_cleanup_index.sql` | Partial index for hourly unconfirmed-subscriber cleanup |
| 016 | `016_add_markets_deadline_index.sql` | Partial index for deadline-based active market queries |
| 017 | `017_add_soft_delete_markets.sql` | Adds `deleted_at` to `markets` + cleanup function |
| 018 | `018_add_varchar_constraints.sql` | CHECK constraints on user-supplied TEXT columns |
| 019 | `019_create_watched_transactions.sql` | Canonical `watched_transactions` table (guarded, idempotent) |

> **Note:** Two migration files share the `010_` prefix. Apply them in lexicographic
> order (`010_add_soft_delete_newsletter.sql` before `010_create_audit_log.sql`).

### Migration note: removal of `audit_logs`

The unused `audit_logs` table (created by `007_create_audit_logs.sql`) has been
removed from `schema.sql`. No source code references it. Environments that already
applied migration `007` may still have the table; drop it with:

```sql
DROP TABLE IF EXISTS audit_logs CASCADE;
```

This is safe because nothing reads or writes `audit_logs`; all audit writes go to
`audit_log`. Do not edit migration `007` in place — its checksum is recorded in
`schema_migrations` and editing it will prevent the service from starting.

### Migration note: `watched_transactions` (017 vs 019)

Migrations `017` and `019` both created `watched_transactions` with different
column sets, so the resulting schema depended on which migration ran and whether
the table already existed (see issue #1747). This is resolved as follows:

- **`019_create_watched_transactions.sql` is the canonical schema.** It uses
  `CREATE TABLE IF NOT EXISTS` and guarded `ALTER TABLE ... ADD COLUMN IF NOT
  EXISTS` statements, so it is idempotent and safe on databases that already
  have the table (from either migration).
- **`017` no longer creates the table.** Its `watched_transactions` DDL has been
  reduced to a no-op so it cannot produce a divergent schema.

Final `watched_transactions` column set (after all migrations):

| Column | Type | Notes |
|--------|------|-------|
| `id` | `bigserial` | Primary key |
| `user_id` | `uuid` | Owner of the watch record |
| `tx_hash` | `text` | On-chain transaction hash |
| `chain_id` | `bigint` | Chain the transaction belongs to |
| `status` | `text` | Watch status (default `'pending'`) |
| `created_at` | `timestamptz` | Default `now()` |
| `updated_at` | `timestamptz` | Default `now()` |

Applying all migrations to an empty database and to one that already has the
old `watched_transactions` table both yield this same schema. Backfilling data
into `watched_transactions` is out of scope.

## Migration Immutability Guarantee

Migration files are embedded at compile time and their SHA-256 checksums are
stored in `schema_migrations` when first applied. At every startup the
`MigrationRunner` compares the checksum of each embedded migration file against
the stored value. If any mismatch is detected the service **refuses to start**
and logs a `FATAL` error identifying the affected migration version and name.

**Do not edit a migration file after it has been applied to any environment.**
Instead, create a new migration that applies the desired change incrementally.

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
| `012_add_performance_indexes.sql` | `rollbacks/012_add_performance_indexes_down.sql` |
| `013_add_email_queue_composite_index.sql` | `rollbacks/013_add_email_queue_composite_index_down.sql` |
| `014_add_email_events_composite_index.sql` | `rollbacks/014_add_email_events_composite_index_down.sql` |
| `015_add_newsletter_cleanup_index.sql` | `rollbacks/015_add_newsletter_cleanup_index_down.sql` |
| `016_add_markets_deadline_index.sql` | `rollbacks/016_add_markets_deadline_index_down.sql` |
| `017_add_soft_delete_markets.sql` | `rollbacks/017_add_soft_delete_markets_down.sql` |
| `018_add_varchar_constraints.sql` | `rollbacks/018_add_varchar_constraints_down.sql` |
| `019_create_watched_transactions.sql` | `rollbacks/019_create_watched_transactions_down.sql` |

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

1. `019` → `018` → `017` → `016` → `015` → `014` → `013` → `012` → `011` → `010b` → `010a` → `009` → `008` → `007` → `006` → `005` → `004` → `003` → `002` → `001` → `000`

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
| `performance_indexes.sql` | Source for the index

/* … truncated 893 chars — edit only what you need near the top … */
