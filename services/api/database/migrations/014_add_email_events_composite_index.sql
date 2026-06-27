-- Composite index for fetching ordered events for a specific email job.
--
-- Analytics queries join email_events to a specific job and order results by
-- timestamp. The existing idx_email_events_job_id covers the equality predicate
-- but the planner must then sort the results. This composite index covers both
-- columns so ordered event lists for a single job are resolved in one scan.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_email_events_job_timestamp
ON email_events (email_job_id, timestamp DESC);
