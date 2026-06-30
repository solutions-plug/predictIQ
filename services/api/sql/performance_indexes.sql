-- Recommended indexes for query performance.
-- Migrations 012–016 promote these into the versioned migration system;
-- this file is kept as a human-readable reference.

-- markets: featured-list query (status filter + volume sort + deadline sort)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_markets_status_volume_ends_at
ON markets (status, total_volume DESC, ends_at ASC);

-- markets: deadline-based active-market scans (e.g. auto-close jobs)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_markets_active_ends_at
ON markets (ends_at ASC)
WHERE status = 'active';

-- content: published content ordered by date
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_content_published_at
ON content (is_published, published_at DESC);

-- email_jobs: priority-ordered queue worker scan
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_email_jobs_status_priority_scheduled
ON email_jobs (status, priority DESC, scheduled_at ASC);

-- email_events: ordered event timeline for a single job
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_email_events_job_timestamp
ON email_events (email_job_id, timestamp DESC);

-- newsletter_subscribers: token expiry cleanup (partial — unconfirmed rows only)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_newsletter_subscribers_cleanup
ON newsletter_subscribers (created_at ASC)
WHERE confirmed = FALSE;
