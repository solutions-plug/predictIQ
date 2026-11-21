CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    source TEXT NOT NULL DEFAULT 'direct',
    confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    -- Stores only the SHA-256 hex digest of the confirmation token (never the
    -- raw token), matching how unsubscribe tokens are handled.
    confirmation_token TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,
    unsubscribed_at TIMESTAMPTZ,
    CONSTRAINT chk_newsletter_confirmation_token_hash
        CHECK (confirmation_token IS NULL OR confirmation_token ~ '^[0-9a-f]{64}$')
);

CREATE INDEX IF NOT EXISTS idx_newsletter_confirmation_token
ON newsletter_subscribers (confirmation_token);

CREATE INDEX IF NOT EXISTS idx_newsletter_confirmed
ON newsletter_subscribers (confirmed);