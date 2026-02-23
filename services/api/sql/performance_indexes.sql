-- Recommended indexes for query performance.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_markets_status_volume_ends_at
ON markets (status, total_volume DESC, ends_at ASC);

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_content_published_at
ON content (is_published, published_at DESC);
