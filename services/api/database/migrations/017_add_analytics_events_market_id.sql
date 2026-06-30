-- Composite index for market-scoped time-series analytics queries.
--
-- Queries that filter analytics_events by market and sort by time perform
-- full table scans because the table has no market_id column or composite
-- index. This migration adds the nullable market_id foreign key and the
-- covering index so market-scoped event lookups resolve in a single index scan.

ALTER TABLE analytics_events
    ADD COLUMN IF NOT EXISTS market_id BIGINT REFERENCES markets(id) ON DELETE SET NULL;

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_analytics_events_market_time
    ON analytics_events (market_id, occurred_at DESC);
