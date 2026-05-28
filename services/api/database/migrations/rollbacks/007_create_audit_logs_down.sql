-- Rollback for 007_create_audit_logs.sql
-- Drops all indexes then the table (UUID-keyed audit_logs, not the bigserial audit_log).
-- FKs to newsletter_subscribers, contact_form_submissions, waitlist_entries, content_management
-- are dropped automatically with the table.

DROP INDEX IF EXISTS idx_audit_logs_deleted_at;
DROP INDEX IF EXISTS idx_audit_logs_created_at;
DROP INDEX IF EXISTS idx_audit_logs_entity_id;
DROP INDEX IF EXISTS idx_audit_logs_entity_type;
DROP INDEX IF EXISTS idx_audit_logs_action;
DROP TABLE IF EXISTS audit_logs;
