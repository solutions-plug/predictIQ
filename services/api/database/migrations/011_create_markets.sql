CREATE TABLE IF NOT EXISTS markets (
    id            BIGSERIAL PRIMARY KEY,
    title         TEXT        NOT NULL,
    status        TEXT        NOT NULL DEFAULT 'active'
                              CHECK (status IN ('active', 'resolved', 'cancelled')),
    outcome_index INTEGER,
    total_volume  DOUBLE PRECISION NOT NULL DEFAULT 0,
    ends_at       TIMESTAMPTZ NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at   TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS markets_status_idx ON markets (status);
