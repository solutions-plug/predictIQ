# Database Schema Reference

This document describes the column-level constraints enforced at the database
layer for user-supplied string fields. All VARCHAR widths and CHECK constraint
limits here correspond to the definitions in `database/migrations/`.

## String Column Limits

### `newsletter_subscribers`

| Column | Type | Limit |
|--------|------|-------|
| `email` | `VARCHAR(255)` | 255 chars — covers the RFC 5321 maximum |
| `source` | `VARCHAR(100)` | 100 chars |
| `confirmation_token` | `VARCHAR(255)` | 255 chars |

### `contact_form_submissions`

| Column | Type / Constraint | Limit |
|--------|-------------------|-------|
| `name` | `VARCHAR(120)` | 120 chars |
| `email` | `VARCHAR(255)` | 255 chars |
| `subject` | `VARCHAR(200)` | 200 chars |
| `message` | `TEXT + CHECK` | 10 000 chars (`chk_contact_message_length`) |
| `status` | `VARCHAR(50)` | 50 chars |

### `waitlist_entries`

| Column | Type | Limit |
|--------|------|-------|
| `email` | `VARCHAR(255)` | 255 chars |
| `status` | `VARCHAR(50)` | 50 chars |
| `source` | `VARCHAR(100)` | 100 chars |

### `content_management`

| Column | Type / Constraint | Limit |
|--------|-------------------|-------|
| `slug` | `VARCHAR(180)` | 180 chars |
| `title` | `VARCHAR(220)` | 220 chars |
| `body` | `TEXT + CHECK` | 500 000 chars (`chk_content_body_length`) |
| `author_email` | `VARCHAR(255)` | 255 chars |
| `status` | `VARCHAR(50)` | 50 chars |

### `markets`

| Column | Type / Constraint | Limit |
|--------|-------------------|-------|
| `title` | `TEXT + CHECK` | 500 chars (`chk_markets_title_length`) |
| `status` | `TEXT + CHECK(IN)` | enumerated: `active`, `resolved`, `cancelled` |

### `email_jobs`

| Column | Type / Constraint | Limit |
|--------|-------------------|-------|
| `job_type` | `VARCHAR(50)` | 50 chars |
| `recipient_email` | `VARCHAR(255)` | 255 chars |
| `template_name` | `VARCHAR(100)` | 100 chars |
| `status` | `VARCHAR(50)` | 50 chars |
| `error_message` | `TEXT + CHECK` | 4 000 chars (`chk_email_jobs_error_message_length`) |

### `email_events`

| Column | Type | Limit |
|--------|------|-------|
| `message_id` | `VARCHAR(255)` | 255 chars |
| `event_type` | `VARCHAR(50)` | 50 chars |
| `recipient_email` | `VARCHAR(255)` | 255 chars |

### `email_suppressions`

| Column | Type / Constraint | Limit |
|--------|-------------------|-------|
| `email` | `VARCHAR(255)` | 255 chars |
| `suppression_type` | `VARCHAR(50)` | 50 chars |
| `reason` | `TEXT + CHECK` | 1 000 chars (`chk_email_suppressions_reason_length`) |
| `bounce_type` | `VARCHAR(50)` | 50 chars |

### `email_template_variants`

| Column | Type / Constraint | Limit |
|--------|-------------------|-------|
| `template_name` | `VARCHAR(100)` | 100 chars |
| `variant_name` | `VARCHAR(50)` | 50 chars |
| `subject_line` | `TEXT + CHECK` | 998 chars (`chk_email_subject_line_length`) — RFC 5322 line limit |

### `analytics_events`

| Column | Type / Constraint | Limit |
|--------|-------------------|-------|
| `event_name` | `VARCHAR(120)` | 120 chars |
| `event_category` | `VARCHAR(80)` | 80 chars |
| `session_id` | `VARCHAR(120)` | 120 chars |
| `page_url` | `TEXT + CHECK` | 2 048 chars (`chk_analytics_page_url_length`) |
| `referrer` | `TEXT + CHECK` | 2 048 chars (`chk_analytics_referrer_length`) |
| `user_agent` | `TEXT + CHECK` | 512 chars (`chk_analytics_user_agent_length`) |

### `audit_logs`

| Column | Type / Constraint | Limit |
|--------|-------------------|-------|
| `action` | `VARCHAR(120)` | 120 chars |
| `entity_type` | `VARCHAR(80)` | 80 chars |
| `actor_email` | `VARCHAR(255)` | 255 chars |
| `reason` | `TEXT + CHECK` | 2 000 chars (`chk_audit_logs_reason_length`) |

## Soft-Delete Columns

The following tables support soft-delete via a `deleted_at TIMESTAMPTZ` column.
All query helpers filter `WHERE deleted_at IS NULL` by default.

| Table | Soft-delete added by |
|-------|----------------------|
| `newsletter_subscribers` | `010_add_soft_delete_newsletter.sql` |
| `content_management` | `005_create_content_management.sql` |
| `audit_logs` | `007_create_audit_logs.sql` |
| `markets` | `017_add_soft_delete_markets.sql` |

Rows with `deleted_at < NOW() - INTERVAL '30 days'` are eligible for permanent
removal via the `cleanup_soft_deleted_markets()` database function (markets) or
the equivalent application-level cleanup job (newsletter subscribers).

## Conventions

- UUID primary keys use `gen_random_uuid()` (requires the `pgcrypto` extension,
  enabled by migration `001`). The `audit_log` table is the sole exception and
  uses `BIGSERIAL`.
- All tables carry `created_at` and `updated_at` timestamp columns.
- Foreign keys that reference parent rows use `ON DELETE SET NULL` or
  `ON DELETE CASCADE` as noted in the migration that defines them.
