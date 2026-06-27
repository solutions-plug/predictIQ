-- Rollback for 010_create_audit_log.sql
-- Removes the append-only audit_log table (bigserial PK) along with its
-- view, triggers, and trigger function.

DROP VIEW IF EXISTS recent_admin_actions;
DROP TRIGGER IF EXISTS prevent_audit_log_delete ON audit_log;
DROP TRIGGER IF EXISTS prevent_audit_log_update ON audit_log;
DROP FUNCTION IF EXISTS prevent_audit_log_modification();
DROP INDEX IF EXISTS idx_audit_log_request_id;
DROP INDEX IF EXISTS idx_audit_log_resource;
DROP INDEX IF EXISTS idx_audit_log_action;
DROP INDEX IF EXISTS idx_audit_log_actor;
DROP INDEX IF EXISTS idx_audit_log_timestamp;
DROP TABLE IF EXISTS audit_log;
