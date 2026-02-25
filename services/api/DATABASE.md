# Database Schema Documentation

This service uses PostgreSQL. The schema and seed scripts for backend issue #13 are in:

- `services/api/database/migrations`
- `services/api/database/seeds`

## Tables

- `newsletter_subscriptions`
- `contact_form_submissions`
- `waitlist_entries`
- `analytics_events`
- `content_management`
- `audit_logs`

## Migration Files

1. `001_enable_pgcrypto.sql`
2. `002_create_newsletter_subscriptions.sql`
3. `003_create_contact_form_submissions.sql`
4. `004_create_waitlist_entries.sql`
5. `005_create_content_management.sql`
6. `006_create_analytics_events.sql`
7. `007_create_audit_logs.sql`

## Apply Migrations

```bash
cd /home/gene/Desktop/drips/predictIQ

# Example: apply all migrations in order
for f in services/api/database/migrations/*.sql; do
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$f"
done
```

## Rollback

This repository currently uses forward SQL migrations only.

For rollback, run explicit reverse scripts in a controlled window (recommended approach):

- Create `down` scripts for each migration before production rollout.
- Restore from backup/snapshot if a hot rollback is required.

## Seeding

```bash
cd /home/gene/Desktop/drips/predictIQ

for f in services/api/database/seeds/*.sql; do
  psql "$DATABASE_URL" -v ON_ERROR_STOP=1 -f "$f"
done
```

## Backup Strategy

- Daily logical backups with `pg_dump` and 30-day retention.
- Weekly full snapshot backup and 90-day retention.
- Quarterly restore drills in staging to verify backup integrity.
- Encrypt backup storage at rest and enforce access controls.

## Data Retention Policy

- `analytics_events`: retain raw events for 13 months, then archive/aggregate.
- `audit_logs`: retain for at least 24 months for compliance and investigations.
- `contact_form_submissions`: retain 12 months unless legal hold is applied.
- `newsletter_subscriptions` and `waitlist_entries`: retain active records; hard-delete per GDPR request.
- Soft-deleted content/audit records (`deleted_at`) should be purged per compliance schedule.

## Notes on Integrity and Performance

- UUID primary keys are generated with `gen_random_uuid()`.
- All core tables include `created_at` and `updated_at` timestamps.
- Soft deletes are implemented with `deleted_at` in `content_management` and `audit_logs`.
- Foreign keys are defined in `analytics_events` and `audit_logs` where relationships are concrete.
- Indexes are included for high-frequency query fields (`email`, `status`, `created_at`, event dimensions).
