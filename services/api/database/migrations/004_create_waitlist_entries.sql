CREATE TABLE IF NOT EXISTS waitlist_entries (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    source VARCHAR(100),
    priority_score INTEGER NOT NULL DEFAULT 0,
    joined_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    converted_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_waitlist_entries_email
ON waitlist_entries (email);

CREATE INDEX IF NOT EXISTS idx_waitlist_entries_status
ON waitlist_entries (status);

CREATE INDEX IF NOT EXISTS idx_waitlist_entries_created_at
ON waitlist_entries (created_at DESC);
