-- Users table
CREATE TABLE IF NOT EXISTS users (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    password_hash VARCHAR(255) NOT NULL,
    is_admin BOOLEAN DEFAULT false,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Content table
CREATE TABLE IF NOT EXISTS content (
    id SERIAL PRIMARY KEY,
    section VARCHAR(100) NOT NULL,
    content JSONB NOT NULL,
    version INTEGER NOT NULL,
    created_by INTEGER REFERENCES users(id),
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Content audit log
CREATE TABLE IF NOT EXISTS content_audit_log (
    id SERIAL PRIMARY KEY,
    section VARCHAR(100) NOT NULL,
    version INTEGER NOT NULL,
    action VARCHAR(50) NOT NULL,
    user_id INTEGER REFERENCES users(id),
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Newsletter signups
CREATE TABLE IF NOT EXISTS newsletter_signups (
    id SERIAL PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_content_section_active ON content(section, is_active);
CREATE INDEX IF NOT EXISTS idx_content_section_version ON content(section, version);
CREATE INDEX IF NOT EXISTS idx_audit_section ON content_audit_log(section);
CREATE INDEX IF NOT EXISTS idx_newsletter_email ON newsletter_signups(email);

-- Insert default admin user (password: admin123)
INSERT INTO users (email, password_hash, is_admin) 
VALUES ('admin@predictiq.com', '$2b$10$rKvVJvH8qN5xZ8YvH8qN5.YvH8qN5xZ8YvH8qN5xZ8YvH8qN5xZ8Y', true)
ON CONFLICT (email) DO NOTHING;

-- Insert default content
INSERT INTO content (section, content, version, created_by, is_active)
VALUES 
    ('hero', '{"headline": "Welcome to PredictIQ", "subheadline": "Decentralized prediction markets", "ctaPrimary": "Get Started", "ctaSecondary": "Learn More"}', 1, 1, true),
    ('features', '{"items": [{"title": "Decentralized", "description": "Built on Stellar"}, {"title": "Secure", "description": "Audited smart contracts"}]}', 1, 1, true),
    ('faq', '{"items": [{"question": "What is PredictIQ?", "answer": "A prediction market platform"}]}', 1, 1, true)
ON CONFLICT DO NOTHING;
