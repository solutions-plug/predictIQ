-- Add CHECK-based length constraints to user-supplied TEXT columns that lack
-- a database-level length guard. Validation was previously enforced only in
-- application code; these constraints protect against direct INSERT/UPDATE
-- bypassing the application layer.
--
-- Limits are chosen to be generous enough for real-world content while
-- still preventing runaway storage from malformed or malicious inputs.

-- markets.title: free-form market title supplied by operators
ALTER TABLE markets
    ADD CONSTRAINT IF NOT EXISTS chk_markets_title_length
        CHECK (char_length(title) <= 500);

-- contact_form_submissions.message: user-supplied contact message body
ALTER TABLE contact_form_submissions
    ADD CONSTRAINT IF NOT EXISTS chk_contact_message_length
        CHECK (char_length(message) <= 10000);

-- analytics_events.page_url: URL of the page where the event occurred
ALTER TABLE analytics_events
    ADD CONSTRAINT IF NOT EXISTS chk_analytics_page_url_length
        CHECK (char_length(page_url) <= 2048);

-- analytics_events.referrer: HTTP Referer header value
ALTER TABLE analytics_events
    ADD CONSTRAINT IF NOT EXISTS chk_analytics_referrer_length
        CHECK (char_length(referrer) <= 2048);

-- analytics_events.user_agent: User-Agent header string
ALTER TABLE analytics_events
    ADD CONSTRAINT IF NOT EXISTS chk_analytics_user_agent_length
        CHECK (char_length(user_agent) <= 512);

-- email_jobs.error_message: internal error detail from the mailer
ALTER TABLE email_jobs
    ADD CONSTRAINT IF NOT EXISTS chk_email_jobs_error_message_length
        CHECK (char_length(error_message) <= 4000);

-- email_suppressions.reason: human-readable suppression reason
ALTER TABLE email_suppressions
    ADD CONSTRAINT IF NOT EXISTS chk_email_suppressions_reason_length
        CHECK (char_length(reason) <= 1000);

-- email_template_variants.subject_line: rendered email subject
ALTER TABLE email_template_variants
    ADD CONSTRAINT IF NOT EXISTS chk_email_subject_line_length
        CHECK (char_length(subject_line) <= 998);

-- audit_logs.reason: free-text audit rationale entered by an operator
ALTER TABLE audit_logs
    ADD CONSTRAINT IF NOT EXISTS chk_audit_logs_reason_length
        CHECK (char_length(reason) <= 2000);

-- content_management.body: CMS page body (allow large content)
ALTER TABLE content_management
    ADD CONSTRAINT IF NOT EXISTS chk_content_body_length
        CHECK (char_length(body) <= 500000);
