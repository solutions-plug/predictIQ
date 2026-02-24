-- Email tracking and analytics tables

-- Email jobs queue tracking
CREATE TABLE IF NOT EXISTS email_jobs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    job_type VARCHAR(50) NOT NULL,
    recipient_email VARCHAR(255) NOT NULL,
    template_name VARCHAR(100) NOT NULL,
    template_data JSONB NOT NULL DEFAULT '{}'::JSONB,
    status VARCHAR(50) NOT NULL DEFAULT 'pending',
    priority INTEGER NOT NULL DEFAULT 0,
    attempts INTEGER NOT NULL DEFAULT 0,
    max_attempts INTEGER NOT NULL DEFAULT 3,
    scheduled_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    started_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    failed_at TIMESTAMPTZ,
    error_message TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_email_jobs_status ON email_jobs (status);
CREATE INDEX IF NOT EXISTS idx_email_jobs_scheduled_at ON email_jobs (scheduled_at);
CREATE INDEX IF NOT EXISTS idx_email_jobs_recipient ON email_jobs (recipient_email);
CREATE INDEX IF NOT EXISTS idx_email_jobs_type ON email_jobs (job_type);

-- Email events tracking (sent, delivered, opened, clicked, bounced, complained)
CREATE TABLE IF NOT EXISTS email_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email_job_id UUID REFERENCES email_jobs(id) ON DELETE CASCADE,
    message_id VARCHAR(255),
    event_type VARCHAR(50) NOT NULL,
    recipient_email VARCHAR(255) NOT NULL,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_email_events_job_id ON email_events (email_job_id);
CREATE INDEX IF NOT EXISTS idx_email_events_message_id ON email_events (message_id);
CREATE INDEX IF NOT EXISTS idx_email_events_type ON email_events (event_type);
CREATE INDEX IF NOT EXISTS idx_email_events_recipient ON email_events (recipient_email);
CREATE INDEX IF NOT EXISTS idx_email_events_timestamp ON email_events (timestamp DESC);

-- Email bounces and complaints
CREATE TABLE IF NOT EXISTS email_suppressions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email VARCHAR(255) UNIQUE NOT NULL,
    suppression_type VARCHAR(50) NOT NULL,
    reason TEXT,
    bounce_type VARCHAR(50),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_email_suppressions_email ON email_suppressions (email);
CREATE INDEX IF NOT EXISTS idx_email_suppressions_type ON email_suppressions (suppression_type);

-- Email templates for A/B testing
CREATE TABLE IF NOT EXISTS email_template_variants (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_name VARCHAR(100) NOT NULL,
    variant_name VARCHAR(50) NOT NULL,
    subject_line TEXT NOT NULL,
    html_content TEXT NOT NULL,
    text_content TEXT,
    is_active BOOLEAN NOT NULL DEFAULT true,
    weight INTEGER NOT NULL DEFAULT 100,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(template_name, variant_name)
);

CREATE INDEX IF NOT EXISTS idx_email_template_variants_name ON email_template_variants (template_name);
CREATE INDEX IF NOT EXISTS idx_email_template_variants_active ON email_template_variants (is_active);

-- Email analytics aggregates
CREATE TABLE IF NOT EXISTS email_analytics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    template_name VARCHAR(100) NOT NULL,
    variant_name VARCHAR(50),
    date DATE NOT NULL,
    sent_count INTEGER NOT NULL DEFAULT 0,
    delivered_count INTEGER NOT NULL DEFAULT 0,
    opened_count INTEGER NOT NULL DEFAULT 0,
    clicked_count INTEGER NOT NULL DEFAULT 0,
    bounced_count INTEGER NOT NULL DEFAULT 0,
    complained_count INTEGER NOT NULL DEFAULT 0,
    unsubscribed_count INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(template_name, variant_name, date)
);

CREATE INDEX IF NOT EXISTS idx_email_analytics_template ON email_analytics (template_name);
CREATE INDEX IF NOT EXISTS idx_email_analytics_date ON email_analytics (date DESC);
