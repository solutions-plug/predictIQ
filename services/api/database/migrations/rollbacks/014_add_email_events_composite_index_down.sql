-- Rollback for 014_add_email_events_composite_index.sql
-- Drops the composite index for ordered email event lookups by job.

DROP INDEX IF EXISTS idx_email_events_job_timestamp;
