-- Migration: 017_create_api_keys
-- Creates the api_keys table for DB-backed key rotation with overlap window support.

CREATE TABLE IF NOT EXISTS api_keys (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    key_hash    TEXT NOT NULL UNIQUE,          -- SHA-256 hex of the raw key
    label       TEXT NOT NULL DEFAULT '',      -- human-readable label
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at  TIMESTAMPTZ,                   -- NULL = never expires; set on rotation
    revoked_at  TIMESTAMPTZ                    -- NULL = active; set to hard-revoke immediately
);

CREATE INDEX IF NOT EXISTS idx_api_keys_expires_at ON api_keys (expires_at)
    WHERE expires_at IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_api_keys_revoked_at ON api_keys (revoked_at)
    WHERE revoked_at IS NOT NULL;
