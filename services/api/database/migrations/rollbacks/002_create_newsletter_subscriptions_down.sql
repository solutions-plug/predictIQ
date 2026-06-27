-- Rollback for 002_create_newsletter_subscriptions.sql
-- Drops all indexes then the table. All subscriber data will be lost.

DROP INDEX IF EXISTS idx_newsletter_subscribers_created_at;
DROP INDEX IF EXISTS idx_newsletter_subscribers_confirmation_token;
DROP INDEX IF EXISTS idx_newsletter_subscribers_email;
DROP TABLE IF EXISTS newsletter_subscribers;
