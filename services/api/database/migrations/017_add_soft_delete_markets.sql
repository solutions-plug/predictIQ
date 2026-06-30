-- Add soft-delete support to the markets table.
--
-- Existing rows are backfilled with deleted_at = NULL (the column default),
-- so no data migration is required beyond adding the column.
--
-- Query helpers must filter WHERE deleted_at IS NULL to exclude soft-deleted
-- markets from all normal reads. A scheduled cleanup function is also created
-- to hard-delete rows that have been soft-deleted for more than 30 days.

ALTER TABLE markets
    ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_markets_deleted_at
    ON markets (deleted_at)
    WHERE deleted_at IS NOT NULL;

-- Cleanup function: hard-delete markets soft-deleted more than 30 days ago.
-- Call this from a scheduled job (e.g. pg_cron or application background task).
CREATE OR REPLACE FUNCTION cleanup_soft_deleted_markets() RETURNS INTEGER
    LANGUAGE plpgsql AS $$
DECLARE
    deleted_count INTEGER;
BEGIN
    DELETE FROM markets
    WHERE deleted_at IS NOT NULL
      AND deleted_at < NOW() - INTERVAL '30 days';
    GET DIAGNOSTICS deleted_count = ROW_COUNT;
    RETURN deleted_count;
END;
$$;
