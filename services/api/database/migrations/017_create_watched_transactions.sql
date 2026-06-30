CREATE TABLE IF NOT EXISTS watched_transactions (
    tx_hash    TEXT        PRIMARY KEY,
    market_id  BIGINT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    status     TEXT        NOT NULL DEFAULT 'pending'
               CONSTRAINT watched_transactions_status_check
                   CHECK (status IN ('pending', 'confirmed', 'expired'))
);

CREATE INDEX IF NOT EXISTS idx_watched_transactions_pending
    ON watched_transactions (expires_at)
    WHERE status = 'pending';
