-- Rollback for 016_add_markets_deadline_index.sql
-- Drops the partial index used by background jobs that auto-close overdue active markets.

DROP INDEX IF EXISTS idx_markets_active_ends_at;
