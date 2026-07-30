-- Rollback for 018_add_varchar_constraints.sql
-- Drops all CHECK constraints added for column-level length validation.

ALTER TABLE markets DROP CONSTRAINT IF EXISTS chk_markets_title_length;
ALTER TABLE contact_form_submissions DROP CONSTRAINT IF EXISTS chk_contact_message_length;
ALTER TABLE analytics_events DROP CONSTRAINT IF EXISTS chk_analytics_page_url_length;
ALTER TABLE analytics_events DROP CONSTRAINT IF EXISTS chk_analytics_referrer_length;
ALTER TABLE analytics_events DROP CONSTRAINT IF EXISTS chk_analytics_user_agent_length;
ALTER TABLE email_jobs DROP CONSTRAINT IF EXISTS chk_email_jobs_error_message_length;
ALTER TABLE email_suppressions DROP CONSTRAINT IF EXISTS chk_email_suppressions_reason_length;
ALTER TABLE email_template_variants DROP CONSTRAINT IF EXISTS chk_email_subject_line_length;
ALTER TABLE audit_logs DROP CONSTRAINT IF EXISTS chk_audit_logs_reason_length;
ALTER TABLE content_management DROP CONSTRAINT IF EXISTS chk_content_body_length;
