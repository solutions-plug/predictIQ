-- Rollback for 011_create_markets.sql
-- Drops the markets table and its status index.

DROP INDEX IF EXISTS markets_status_idx;
DROP TABLE IF EXISTS markets;
