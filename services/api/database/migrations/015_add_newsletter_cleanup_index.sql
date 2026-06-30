-- Partial composite index for the newsletter token cleanup query.
--
-- The hourly cleanup deletes rows WHERE confirmed = FALSE AND created_at <= threshold.
-- The existing idx_newsletter_confirmed covers the boolean predicate but leaves the
-- planner to filter on created_at afterward. A partial index scoped to unconfirmed
-- rows means the index is small (only pending subscribers) and covers both the
-- equality predicate and the range filter in a single scan.

CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_newsletter_subscribers_cleanup
ON newsletter_subscribers (created_at ASC)
WHERE confirmed = FALSE;
