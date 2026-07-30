-- Rollback for 013_add_email_queue_composite_index.sql
-- Drops the composite index for priority-ordered email queue worker scans.

DROP INDEX IF EXISTS idx_email_jobs_status_priority_scheduled;
