-- Create markets table for featured markets functionality
CREATE TABLE IF NOT EXISTS markets (
    id BIGSERIAL PRIMARY KEY,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    category VARCHAR(100) NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    total_volume DOUBLE PRECISION NOT NULL DEFAULT 0,
    participant_count INTEGER NOT NULL DEFAULT 0,
    ends_at TIMESTAMP WITH TIME ZONE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    outcome_options JSONB NOT NULL DEFAULT '[]'::jsonb,
    current_odds JSONB NOT NULL DEFAULT '{}'::jsonb,
    metadata JSONB NOT NULL DEFAULT '{}'::jsonb
);

-- Create indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_markets_status ON markets(status);
CREATE INDEX IF NOT EXISTS idx_markets_category ON markets(category);
CREATE INDEX IF NOT EXISTS idx_markets_ends_at ON markets(ends_at);
CREATE INDEX IF NOT EXISTS idx_markets_volume ON markets(total_volume DESC);
CREATE INDEX IF NOT EXISTS idx_markets_participants ON markets(participant_count DESC);
CREATE INDEX IF NOT EXISTS idx_markets_featured_ranking ON markets(status, total_volume DESC, participant_count DESC, ends_at);

-- Add trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_markets_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_markets_updated_at
    BEFORE UPDATE ON markets
    FOR EACH ROW
    EXECUTE FUNCTION update_markets_updated_at();
