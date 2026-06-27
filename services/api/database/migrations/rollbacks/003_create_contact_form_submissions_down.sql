-- Rollback for 003_create_contact_form_submissions.sql
-- Drops all indexes then the table. All contact form data will be lost.

DROP INDEX IF EXISTS idx_contact_form_submissions_created_at;
DROP INDEX IF EXISTS idx_contact_form_submissions_status;
DROP INDEX IF EXISTS idx_contact_form_submissions_email;
DROP TABLE IF EXISTS contact_form_submissions;
