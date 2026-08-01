-- Rollback for 017_add_soft_delete_markets.sql
-- Drops the cleanup function, the partial index, and the deleted_at column.
-- Any rows with a non-NULL deleted_at value will have that data discarded.

DROP FUNCTION IF EXISTS cleanup_soft_deleted_markets();
DROP INDEX IF EXISTS idx_markets_deleted_at;
ALTER TABLE markets DROP COLUMN IF EXISTS deleted_at;
