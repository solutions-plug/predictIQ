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

## `markets` Consistency Constraints

The `markets` table mirrors on-chain state. The invariants below are enforced at
the database level so that direct writes and migrations cannot persist
contradictory state (see `contracts/predict-iq/INVARIANTS.md`).

| Constraint | Type | Invariant enforced |
|---|---|---|
| `markets_on_chain_market_id_key` | UNIQUE | Each on-chain market identifier maps to exactly one row; no duplicate mirrors of the same market. |
| `markets_outcome_count_check` | CHECK | A market has at least two outcomes (`outcome_count >= 2`), matching the contract's minimum outcome count. |
| `markets_status_check` | CHECK | `status` is one of the contract's valid lifecycle states (`active`, `closed`, `resolved`, `cancelled`). |
| `markets_resolved_status_check` | CHECK | A market may only carry a `resolved_outcome` when `status = 'resolved'`; unresolved markets must leave it `NULL`. |
| `markets_resolved_outcome_check` | CHECK | When present, `resolved_outcome` is a valid outcome index (`resolved_outcome >= 0` and `< outcome_count`). |
| `markets_deadline_check` | CHECK | `deadline` is strictly after `created_at`, so a market cannot be created already expired. |

Any insert or update that violates one of these constraints fails at the
database level with a constraint-violation error.

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
| 013 | `013_add_email_queue_composite_index.sql` | Composite index for priority-ordered email queue scans |
| 014 | `014_add_email_events_composite_index.sql` | Composite index for ordered email event lookups by job |
| 015 | `015_add_newsletter_cleanup_index.sql` | Partial index for hourly unconfirmed-subscriber cleanup |
| 016 | `016_add_markets_deadline_index.sql` | Partial index for deadline-based active market queries |
| 017 | `017_add_soft_delete_markets.sql` | Adds `deleted_at` to `markets` + cleanup function |
| 018 | `018_add_varchar_constraints.sql` | CHECK constraints on user-supplied TEXT columns |
| 019 | `019_add_markets_consistency_constraints.sql` | UNIQUE + CHECK constraints enforcing `markets` invariants |

> **Note:** Two migration files share the `010_` prefix. Apply them in lexicographic
> order (`010_add_soft_delete_newsletter.sql` before `010_create_audit_log.sql`).

## Migration Naming and Numbering Rules

Every schema change lives in its own file under
`services/api/database/migrations/`. The filename determines both the
**version** recorded in `schema_migrations` and the **order** in which
migrations are applied, so the format is enforced by tooling.

### Filename format

```
<prefix>_<description>.sql
```

| Part | Rule | Examples |
|------|------|----------|
| `<prefix>` | Zero-padded number, optionally followed by a single lowercase letter suffix | `017`, `017b`, `017c` |
| `<description>` | Lowercase words separated by underscores, describing the change | `add_soft_delete_markets` |
| Extension | Always `.sql` | — |

The **filename stem** (the filename without the `.sql` extension) is the
canonical identifier for a migration, e.g. `017b_add_markets_status_index`.

### The numeric prefix is not the version

The numeric prefix is **not** the migration version. The version is the
**full filename stem**, and that is the value stored in the
`schema_migrations` table and compared against the embedded migration at
startup.

This means files that share a numeric prefix are all distinct migrations and
are all applied. For example, `017`, `017b`, `017c`, `017d` and `017e` are
five separate migrations, not one. Treating the numeric prefix as the version
causes files sharing a prefix to be silently skipped — see issue #1746 for the
precedent.

### Ordering rule

Migrations are applied in **lexicographic order of the filename**. Because
`_` (ASCII `0x5F`) sorts after any lowercase letter, a letter suffix sorts
after the bare number:

```
017_add_soft_delete_markets.sql
017b_...
017c_...
018_add_varchar_constraints.sql
```

Choose suffixes so that lexicographic order matches the intended apply order.
When adding a migration, use the next unused number; if that number is already
taken, append the next free letter suffix (`017b`, `017c`, …) rather than
renumbering existing files.

### Registry and rollback requirements

Each migration must be registered in the `MIGRATIONS` array in
`services/api/src/migrations.rs`:

| Field | Requirement |
|-------|-------------|
| `version` | Unique string equal to the filename stem (e.g. `"017b_add_markets_status_index"`) |
| `name` | Equal to the filename stem |
| Rollback | A matching `rollbacks/<stem>_down.sql` file must exist |

A migration file that is not in the registry, a registry entry with no
matching file, a missing rollback, an orphan rollback, or a duplicate
`version` will all fail the registry check.

## Migration Registry Check

`scripts/check_migrations_registry.sh` verifies that the migration registry stays
consistent with the files on disk. It checks that:

- every migration file in `services/api/database/migrations/` is registered in
  `services/api/src/migrations.rs`,
- every migration has a matching rollback script in
  `services/api/database/migrations/rollbacks/`,
- there are no orphan rollback scripts without a corresponding migration,
- there are no duplicate versions in the registry.

CI runs this check on every pull request (see the migration registry job in
`.github/workflows/test.yml`); a non-zero exit fails the job.

The registry check is `scripts/check_migrations_registry.sh`. It runs in CI
(the migrations workflow) and can be run locally from the repository root:

```bash
bash scripts/check_migrations_registry.sh
```

It fails on:

- migration files present on disk but missing from the `MIGRATIONS` registry,
- registry entries with no matching migration file,
- missing or orphaned rollback scripts,
- duplicate `version` strings in the registry.

Run it before opening a PR that adds or renames a migration.

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
| `019_add_markets_consistency_constraints.sql` | `rollbacks/019_add_markets_consistency_constraints_down.sql` |

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
| `DB_POOL_IDLE_TIMEOUT_SECS` | _(sqlx default)
