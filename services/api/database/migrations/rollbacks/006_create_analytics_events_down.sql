-- Rollback for 006_create_analytics_events.sql
-- Drops all indexes then the table.
-- The FK to content_management is dropped with the table automatically.

DROP INDEX IF EXISTS idx_analytics_events_properties_gin;
DROP INDEX IF EXISTS idx_analytics_events_created_at;
DROP INDEX IF EXISTS idx_analytics_events_session_id;
DROP INDEX IF EXISTS idx_analytics_events_occurred_at;
DROP INDEX IF EXISTS idx_analytics_events_event_name;
DROP TABLE IF EXISTS analytics_events;
