CREATE TABLE IF NOT EXISTS analytics_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_name VARCHAR(120) NOT NULL,
    event_category VARCHAR(80),
    user_id UUID,
    session_id VARCHAR(120),
    page_url TEXT,
    referrer TEXT,
    properties JSONB NOT NULL DEFAULT '{}'::JSONB,
    ip_address INET,
    user_agent TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    content_id UUID,
    CONSTRAINT fk_analytics_content
        FOREIGN KEY (content_id) REFERENCES content_management(id)
        ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_analytics_events_event_name
ON analytics_events (event_name);

CREATE INDEX IF NOT EXISTS idx_analytics_events_occurred_at
ON analytics_events (occurred_at DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_events_session_id
ON analytics_events (session_id);

CREATE INDEX IF NOT EXISTS idx_analytics_events_created_at
ON analytics_events (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_analytics_events_properties_gin
ON analytics_events USING GIN (properties);
