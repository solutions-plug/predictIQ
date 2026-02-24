CREATE TABLE IF NOT EXISTS content_management (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    slug VARCHAR(180) UNIQUE NOT NULL,
    title VARCHAR(220) NOT NULL,
    body TEXT NOT NULL,
    excerpt TEXT,
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    author_email VARCHAR(255) NOT NULL,
    version INTEGER NOT NULL DEFAULT 1,
    metadata JSONB NOT NULL DEFAULT '{}'::JSONB,
    published_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ
);

CREATE INDEX IF NOT EXISTS idx_content_management_slug
ON content_management (slug);

CREATE INDEX IF NOT EXISTS idx_content_management_status
ON content_management (status);

CREATE INDEX IF NOT EXISTS idx_content_management_created_at
ON content_management (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_content_management_deleted_at
ON content_management (deleted_at);
