-- Partial index for deadline-based active market queries.
--
-- Queries that look up markets expiring before a given timestamp —
-- e.g. background jobs that auto-close overdue markets — scan the full
-- markets table today because the existing composite index leads with status
-- and total_volume. A partial index scoped to active markets and ordered by
-- ends_at lets those scans skip the resolved and cancelled majority of rows.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_markets_active_ends_at
ON markets (ends_at ASC)
WHERE status = 'active';
