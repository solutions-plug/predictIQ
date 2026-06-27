-- Rollback for 001_enable_pgcrypto.sql
-- Drops the pgcrypto extension.
-- WARNING: any existing gen_random_uuid() values are not affected, but the
--          function will no longer be available for new inserts.

DROP EXTENSION IF EXISTS pgcrypto;
