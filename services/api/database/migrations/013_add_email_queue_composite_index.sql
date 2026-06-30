-- Composite index for priority-ordered queue worker scans.
--
-- The email queue worker queries: WHERE status = 'pending' ORDER BY priority DESC, scheduled_at ASC
-- The existing separate idx_email_jobs_status and idx_email_jobs_scheduled_at indexes force the
-- planner to choose one and filter on the other. This composite index covers both columns so the
-- planner can satisfy the full predicate and sort in a single index scan.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_email_jobs_status_priority_scheduled
ON email_jobs (status, priority DESC, scheduled_at ASC);
