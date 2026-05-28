-- Rollback for 010_add_soft_delete_newsletter.sql
-- Removes the two partial indexes and drops the deleted_at column.
-- Existing soft-deleted rows will have their deleted_at value discarded.

DROP INDEX IF EXISTS idx_newsletter_subscribers_active;
DROP INDEX IF EXISTS idx_newsletter_subscribers_deleted_at;
ALTER TABLE newsletter_subscribers DROP COLUMN IF EXISTS deleted_at;
