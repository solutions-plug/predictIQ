DROP INDEX IF EXISTS idx_analytics_events_market_time;
ALTER TABLE analytics_events DROP COLUMN IF EXISTS market_id;
