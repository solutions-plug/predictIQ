-- Composite and partial indexes for failed/bounced email lookups.
--
-- Queries that fetch all events for a given recipient address (e.g. to decide
-- whether to suppress future sends) scan the full email_events table today
-- because the existing idx_email_events_recipient covers only the equality
-- predicate, and the planner must then filter on event_type separately.
--
-- The composite index covers both columns so recipient+type lookups resolve in
-- one scan. The partial index is scoped to bounce and dropped events only,
-- keeping it small while accelerating the most latency-sensitive suppression
-- queries.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_email_tracking_email_status
    ON email_events (recipient_email, event_type);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_email_tracking_email_status_failed
    ON email_events (recipient_email, event_type)
    WHERE event_type IN ('bounce', 'dropped');
