CREATE TABLE IF NOT EXISTS newsletter_subscribers (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    source VARCHAR(100) NOT NULL DEFAULT 'direct',
    confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    confirmation_token VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,
    unsubscribed_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_email
ON newsletter_subscribers (email);

CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_confirmation_token
ON newsletter_subscribers (confirmation_token) WHERE confirmation_token IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_newsletter_subscribers_created_at
ON newsletter_subscribers (created_at DESC);
