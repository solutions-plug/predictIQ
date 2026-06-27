-- Rollback for 009_add_newsletter_indexes.sql
-- Drops the two partial indexes added for active-subscriber query performance.

DROP INDEX IF EXISTS idx_newsletter_subscribers_email_status;
DROP INDEX IF EXISTS idx_newsletter_subscribers_status;
