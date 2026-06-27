-- Rollback for 000_create_schema_migrations.sql
-- Removes the migration tracking table itself.
-- WARNING: applying this drop means all migration history is lost.

DROP TABLE IF EXISTS schema_migrations;
