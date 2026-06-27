-- Rollback for 005_create_content_management.sql
-- Drops analytics_events first (has FK to content_management), then content_management.

DROP INDEX IF EXISTS idx_content_management_deleted_at;
DROP INDEX IF EXISTS idx_content_management_created_at;
DROP INDEX IF EXISTS idx_content_management_status;
DROP INDEX IF EXISTS idx_content_management_slug;
DROP TABLE IF EXISTS content_management;
