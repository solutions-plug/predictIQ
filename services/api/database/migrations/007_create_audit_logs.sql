CREATE TABLE IF NOT EXISTS audit_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    action VARCHAR(120) NOT NULL,
    entity_type VARCHAR(80) NOT NULL,
    entity_id UUID,
    actor_email VARCHAR(255),
    actor_ip INET,
    reason TEXT,
    changes JSONB NOT NULL DEFAULT '{}'::JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at TIMESTAMPTZ,
    newsletter_subscription_id UUID,
    contact_submission_id UUID,
    waitlist_entry_id UUID,
    content_id UUID,
    CONSTRAINT fk_audit_newsletter
        FOREIGN KEY (newsletter_subscription_id) REFERENCES newsletter_subscriptions(id)
        ON DELETE SET NULL,
    CONSTRAINT fk_audit_contact
        FOREIGN KEY (contact_submission_id) REFERENCES contact_form_submissions(id)
        ON DELETE SET NULL,
    CONSTRAINT fk_audit_waitlist
        FOREIGN KEY (waitlist_entry_id) REFERENCES waitlist_entries(id)
        ON DELETE SET NULL,
    CONSTRAINT fk_audit_content
        FOREIGN KEY (content_id) REFERENCES content_management(id)
        ON DELETE SET NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_logs_action
ON audit_logs (action);

CREATE INDEX IF NOT EXISTS idx_audit_logs_entity_type
ON audit_logs (entity_type);

CREATE INDEX IF NOT EXISTS idx_audit_logs_entity_id
ON audit_logs (entity_id);

CREATE INDEX IF NOT EXISTS idx_audit_logs_created_at
ON audit_logs (created_at DESC);

CREATE INDEX IF NOT EXISTS idx_audit_logs_deleted_at
ON audit_logs (deleted_at);
