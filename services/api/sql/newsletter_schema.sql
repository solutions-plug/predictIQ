CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id BIGSERIAL PRIMARY KEY,
    email TEXT NOT NULL UNIQUE,
    source TEXT NOT NULL DEFAULT 'direct',
    confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    confirmation_token TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,
    unsubscribed_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_newsletter_confirmation_token
ON newsletter_subscribers (confirmation_token);

CREATE INDEX IF NOT EXISTS idx_newsletter_confirmed
ON newsletter_subscribers (confirmed);

