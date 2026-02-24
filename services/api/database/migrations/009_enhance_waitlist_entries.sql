-- Add additional fields to waitlist_entries table for enhanced functionality
ALTER TABLE waitlist_entries
ADD COLUMN IF NOT EXISTS name VARCHAR(255),
ADD COLUMN IF NOT EXISTS role VARCHAR(50),
ADD COLUMN IF NOT EXISTS referral_code VARCHAR(50) UNIQUE,
ADD COLUMN IF NOT EXISTS referred_by_code VARCHAR(50),
ADD COLUMN IF NOT EXISTS position INTEGER,
ADD COLUMN IF NOT EXISTS invited_at TIMESTAMPTZ,
ADD COLUMN IF NOT EXISTS invitation_accepted_at TIMESTAMPTZ;

-- Create index for referral tracking
CREATE INDEX IF NOT EXISTS idx_waitlist_entries_referral_code
ON waitlist_entries (referral_code);

CREATE INDEX IF NOT EXISTS idx_waitlist_entries_referred_by
ON waitlist_entries (referred_by_code);

CREATE INDEX IF NOT EXISTS idx_waitlist_entries_position
ON waitlist_entries (position);

-- Create referral stats table
CREATE TABLE IF NOT EXISTS waitlist_referrals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    referrer_code VARCHAR(50) NOT NULL,
    referral_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(referrer_code)
);

CREATE INDEX IF NOT EXISTS idx_waitlist_referrals_code
ON waitlist_referrals (referrer_code);
