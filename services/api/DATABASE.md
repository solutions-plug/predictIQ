# Database Documentation

The PredictIQ API service uses PostgreSQL. All schema changes are managed through ordered SQL migration files.

## API server connection pool

The API builds a [`sqlx`](https://docs.rs/sqlx) PostgreSQL pool at startup. Pool sizing and timeouts can be tuned with environment variables (defaults match the previous hard-coded values).

| Variable | Default | Description |
|----------|---------|-------------|
| `DB_POOL_MIN_CONNECTIONS` | `5` | Minimum idle connections kept open. |
| `DB_POOL_MAX_CONNECTIONS` | `25` | Upper bound on concurrent connections. If `min` and `max` are reversed, they are swapped; `max` is clamped to at least `1`. |
| `DB_POOL_ACQUIRE_TIMEOUT_SECS` | `5` | Max time to wait for a connection from the pool (seconds). Clamped to at least `1`. |
| `DB_POOL_IDLE_TIMEOUT_SECS` | *(unset)* | If set to a positive integer, idle connections older than this are closed (seconds). When unset, sqlx’s default applies. `0` is ignored (same as unset). |
| `DB_POOL_MAX_LIFETIME_SECS` | *(unset)* | If set to a positive integer, connections are recycled after this lifetime (seconds). When unset, sqlx’s default applies. `0` is ignored (same as unset). |

`DATABASE_URL` is still required for the connection string (see default in `config.rs` for local development).

## Running Migrations

From the workspace root:

```bash
# Apply all migrations (requires DATABASE_URL in environment or .env)
cd services/api
cargo sqlx migrate run
```

Or apply manually in order:

```bash
psql "$DATABASE_URL" -f services/api/database/migrations/001_enable_pgcrypto.sql
psql "$DATABASE_URL" -f services/api/database/migrations/002_create_newsletter_subscribers.sql
psql "$DATABASE_URL" -f services/api/database/migrations/003_create_contact_form_submissions.sql
psql "$DATABASE_URL" -f services/api/database/migrations/004_create_waitlist_entries.sql
psql "$DATABASE_URL" -f services/api/database/migrations/005_create_content_management.sql
psql "$DATABASE_URL" -f services/api/database/migrations/006_create_analytics_events.sql
psql "$DATABASE_URL" -f services/api/database/migrations/007_create_audit_logs.sql
psql "$DATABASE_URL" -f services/api/database/migrations/008_create_email_tracking.sql
```

## Seed Data (Development Only)

```bash
psql "$DATABASE_URL" -f services/api/database/seeds/01_newsletter_subscriptions.sql
psql "$DATABASE_URL" -f services/api/database/seeds/02_waitlist_entries.sql
psql "$DATABASE_URL" -f services/api/database/seeds/03_contact_form_submissions.sql
psql "$DATABASE_URL" -f services/api/database/seeds/04_content_management.sql
psql "$DATABASE_URL" -f services/api/database/seeds/05_analytics_events.sql
psql "$DATABASE_URL" -f services/api/database/seeds/06_audit_logs.sql
```

## Tables

### `newsletter_subscribers`

Stores newsletter subscription state. The application code (see `services/api/src/db.rs`) references this table as `newsletter_subscribers`.

> **Note:** The migration file is named `002_create_newsletter_subscribers.sql` and the seed file inserts into `newsletter_subscriptions` — the migration must be updated to match. See [Schema Reconciliation](#schema-reconciliation) below.

| Column | Type | Notes |
|--------|------|-------|
| `id` | `UUID` | Primary key, auto-generated |
| `email` | `VARCHAR(255)` | Unique, normalized to lowercase |
| `source` | `VARCHAR(100)` | Acquisition channel (e.g. `homepage`, `blog`) |
| `confirmed` | `BOOLEAN` | `true` after email confirmation |
| `confirmation_token` | `TEXT` | Nullable; cleared on confirmation |
| `created_at` | `TIMESTAMPTZ` | Row creation time |
| `confirmed_at` | `TIMESTAMPTZ` | Nullable |
| `unsubscribed_at` | `TIMESTAMPTZ` | Nullable |

### `waitlist_entries`

| Column | Type | Notes |
|--------|------|-------|
| `id` | `UUID` | Primary key |
| `email` | `VARCHAR(255)` | Unique |
| `created_at` | `TIMESTAMPTZ` | |

### `contact_form_submissions`

Stores contact form messages.

### `content_management`

CMS content items with category and publish state.

### `analytics_events`

Append-only event log for product analytics.

### `audit_logs`

Immutable audit trail for admin actions.

### `email_jobs` / `email_tracking`

Email delivery queue and open/click tracking (migration `008`).

---

## Schema Reconciliation

The application (`db.rs`) uses the table name **`newsletter_subscribers`**, while the migration file (`002`) and seed file (`01`) currently create/insert into **`newsletter_subscriptions`**.

**Resolution:** The migration file `002_create_newsletter_subscriptions.sql` should be renamed to `002_create_newsletter_subscribers.sql` and the `CREATE TABLE` statement updated to use `newsletter_subscribers`. The seed file column list should also be updated to match the application schema (`confirmed`, `confirmation_token` columns instead of `status`).

Until that migration is applied, run the following one-time rename in your database:

```sql
ALTER TABLE newsletter_subscriptions RENAME TO newsletter_subscribers;
```
