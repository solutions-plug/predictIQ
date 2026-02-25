CREATE TABLE IF NOT EXISTS newsletter_subscriptions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL,
    source VARCHAR(100),
    subscribed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    confirmed_at TIMESTAMPTZ,
    unsubscribed_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_newsletter_subscriptions_email
ON newsletter_subscriptions (email);

CREATE INDEX IF NOT EXISTS idx_newsletter_subscriptions_status
ON newsletter_subscriptions (status);

CREATE INDEX IF NOT EXISTS idx_newsletter_subscriptions_created_at
ON newsletter_subscriptions (created_at DESC);
