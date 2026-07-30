-- Migration 019: watched_transactions table
--
-- Persists the set of transaction hashes being monitored so the watch-map
-- state survives API restarts.  The UNIQUE constraint on tx_hash enforces the
-- deduplication invariant at the database layer (#937).  The expires_at column
-- allows server-side TTL expiry to be driven by WATCHED_TX_TTL_SECS (#933).
--
-- This table is append-only during normal operation; a periodic cleanup job
-- (or a simple DELETE WHERE expires_at < NOW()) is responsible for pruning
-- expired rows.

CREATE TABLE IF NOT EXISTS watched_transactions (
    id          BIGSERIAL    PRIMARY KEY,
    tx_hash     VARCHAR(128) NOT NULL,
    watched_at  TIMESTAMPTZ  NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ  NOT NULL,

    CONSTRAINT uq_watched_transactions_tx_hash UNIQUE (tx_hash)
);

-- Index to speed up TTL cleanup queries.
CREATE INDEX IF NOT EXISTS idx_watched_transactions_expires_at
    ON watched_transactions (expires_at);
