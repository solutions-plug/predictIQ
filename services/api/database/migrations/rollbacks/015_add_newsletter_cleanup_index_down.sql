-- Rollback for 015_add_newsletter_cleanup_index.sql
-- Drops the partial index used by the hourly unconfirmed-subscriber cleanup job.

DROP INDEX IF EXISTS idx_newsletter_subscribers_cleanup;
