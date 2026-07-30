-- Migration 017: opaque unsubscribe tokens
--
-- Replaces the predictable base64(email || "." || HMAC(email)) scheme with
-- random 256-bit tokens stored in the database.
--
-- Schema:
--   token_hash  SHA-256 hex of the 32-byte random token stored by the caller.
--               Only the hash is persisted so a DB breach does not expose
--               usable tokens.
--   subscriber_id  FK to newsletter_subscribers(id). Cascade-delete keeps the
--               table clean when a subscriber is hard-deleted.
--   expires_at  Tokens expire after the configured TTL (default 7 days).
--   used_at     Set on first use; subsequent uses are rejected (single-use).

CREATE TABLE IF NOT EXISTS unsubscribe_tokens (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    token_hash    CHAR(64)    NOT NULL UNIQUE,          -- SHA-256 hex, 64 chars
    subscriber_id UUID        NOT NULL
                  REFERENCES newsletter_subscribers(id)
                  ON DELETE CASCADE,
    expires_at    TIMESTAMPTZ NOT NULL,
    used_at       TIMESTAMPTZ             DEFAULT NULL,
    created_at    TIMESTAMPTZ NOT NULL    DEFAULT NOW()
);

-- Fast lookup by hash (primary use-case: validate incoming token)
CREATE INDEX IF NOT EXISTS idx_unsubscribe_tokens_hash
    ON unsubscribe_tokens (token_hash);

-- Housekeeping: quickly find expired / already-used tokens for cleanup
CREATE INDEX IF NOT EXISTS idx_unsubscribe_tokens_expires_at
    ON unsubscribe_tokens (expires_at)
    WHERE used_at IS NULL;

COMMENT ON TABLE unsubscribe_tokens IS
    'Single-use opaque unsubscribe tokens (256-bit random, SHA-256 hashed). '
    'See services/api/src/newsletter.rs and issue #896.';
