-- Rollback for 012_add_performance_indexes.sql
-- Drops the two composite/compound performance indexes added for markets and content.
-- CONCURRENTLY is not valid inside a transaction block, so these use plain DROP INDEX IF EXISTS.

DROP INDEX IF EXISTS idx_markets_status_volume_ends_at;
DROP INDEX IF EXISTS idx_content_published_at;
