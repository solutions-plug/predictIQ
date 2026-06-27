-- Rollback for 008_create_email_tracking.sql
-- Drop tables in reverse dependency order.
-- email_events has a FK to email_jobs; drop it before email_jobs.

DROP INDEX IF EXISTS idx_email_analytics_date;
DROP INDEX IF EXISTS idx_email_analytics_template;
DROP TABLE IF EXISTS email_analytics;

DROP TABLE IF EXISTS email_template_variants;

DROP INDEX IF EXISTS idx_email_suppressions_type;
DROP INDEX IF EXISTS idx_email_suppressions_email;
DROP TABLE IF EXISTS email_suppressions;

DROP INDEX IF EXISTS idx_email_events_timestamp;
DROP INDEX IF EXISTS idx_email_events_recipient;
DROP INDEX IF EXISTS idx_email_events_type;
DROP INDEX IF EXISTS idx_email_events_message_id;
DROP INDEX IF EXISTS idx_email_events_job_id;
DROP TABLE IF EXISTS email_events;

DROP INDEX IF EXISTS idx_email_jobs_type;
DROP INDEX IF EXISTS idx_email_jobs_recipient;
DROP INDEX IF EXISTS idx_email_jobs_scheduled_at;
DROP INDEX IF EXISTS idx_email_jobs_status;
DROP TABLE IF EXISTS email_jobs;
