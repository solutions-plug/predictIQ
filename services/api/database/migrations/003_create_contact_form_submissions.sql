CREATE TABLE IF NOT EXISTS contact_form_submissions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(120) NOT NULL,
    email VARCHAR(255) NOT NULL,
    subject VARCHAR(200) NOT NULL,
    message TEXT NOT NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'new',
    submitted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_contact_form_submissions_email
ON contact_form_submissions (email);

CREATE INDEX IF NOT EXISTS idx_contact_form_submissions_status
ON contact_form_submissions (status);

CREATE INDEX IF NOT EXISTS idx_contact_form_submissions_created_at
ON contact_form_submissions (created_at DESC);
