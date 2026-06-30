CREATE TABLE email_dead_letter_jobs (
    id              UUID        PRIMARY KEY,
    payload         JSONB       NOT NULL,
    failure_reason  TEXT        NOT NULL,
    failed_at       TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    retry_count     INTEGER     NOT NULL DEFAULT 0
);

CREATE INDEX idx_email_dead_letter_jobs_failed_at
    ON email_dead_letter_jobs (failed_at DESC);
