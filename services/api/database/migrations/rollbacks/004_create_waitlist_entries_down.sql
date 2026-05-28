-- Rollback for 004_create_waitlist_entries.sql
-- Drops all indexes then the table. All waitlist data will be lost.

DROP INDEX IF EXISTS idx_waitlist_entries_created_at;
DROP INDEX IF EXISTS idx_waitlist_entries_status;
DROP INDEX IF EXISTS idx_waitlist_entries_email;
DROP TABLE IF EXISTS waitlist_entries;
