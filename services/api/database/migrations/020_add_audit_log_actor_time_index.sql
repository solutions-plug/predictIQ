-- Composite index for actor-scoped audit trail queries.
--
-- Admin compliance queries that filter by actor and a time range perform
-- sequential scans because the existing idx_audit_log_actor and
-- idx_audit_log_timestamp indexes are single-column. The composite index
-- covers the equality predicate on actor and the ORDER BY timestamp DESC
-- in a single scan.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_audit_log_actor_time
    ON audit_log (actor, timestamp DESC);
