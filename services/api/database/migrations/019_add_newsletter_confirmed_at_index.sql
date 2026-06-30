-- Partial index for subscriber growth analytics and cleanup queries.
--
-- Queries that bucket confirmed subscribers by confirmation date perform full
-- table scans because no index covers confirmed_at. The partial index is
-- scoped to non-NULL confirmed_at rows (i.e. confirmed subscribers only),
-- keeping it small and excluding unconfirmed rows that are never included in
-- growth reports or retention cleanup.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_newsletter_confirmed_at
    ON newsletter_subscribers (confirmed_at)
    WHERE confirmed_at IS NOT NULL;
