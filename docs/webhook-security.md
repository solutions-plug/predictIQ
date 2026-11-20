# SendGrid Webhook Security

This document describes the security mechanisms protecting the predictIQ API's SendGrid webhook endpoint.

## Overview

The `/webhooks/sendgrid` endpoint receives and processes email delivery status events (bounces, clicks, opens, etc.) from SendGrid. The endpoint must:

1. Authenticate that requests actually originate from SendGrid
2. Prevent duplicate processing if SendGrid retries a delivery
3. Reject oversized payloads that could cause DoS
4. Sanitize event data before persistence

## Authentication: Cryptographic Signatures

SendGrid signs every webhook request with an **ECDSA P-256** signature over the raw event payload. The signature is verified by the `sendgrid_webhook_middleware` before the request reaches the handler.

**Configuration:**
- `SENDGRID_WEBHOOK_SECRET`: SendGrid's Signed Event Webhook **public key** (base64 DER, ECDSA P-256), supplied via configuration. It is never hard-coded in the source. Must be set in production.
- Signature is in the `X-Twilio-Email-Event-Webhook-Signature` header
- Verification happens in `security::sendgrid_webhook_middleware` before body parsing

**Production Requirements:**
- Must match the public key configured in the SendGrid dashboard
- Requests with a missing signature are rejected with HTTP 4xx
- Requests with an invalid or tampered signature are rejected with HTTP 4xx
- If the public key is not configured, all webhook requests are rejected (safe default)

**Local Testing Fallback:**
- Outside production only, an HMAC-SHA256 shared secret may be used instead of ECDSA verification, to simplify local testing. This fallback is never used in production.

## Replay Protection: Two-Stage Deduplication

SendGrid may retry webhook deliveries if the endpoint fails to respond. Without replay protection, the same event (bounce, complaint, etc.) could be processed multiple times, leading to:

- Duplicate database records
- Multiple emails sent to suppressed recipients  
- Inflated analytics counters

The WebhookHandler implements **two-stage replay protection** to prevent duplicates:

### Stage 1: Redis Nonce (Primary Guard)

**Mechanism:**
Each event creates a composite nonce key:
```
webhook_nonce:{message_id}:{event_type}:{email}
```

The key is stored in Redis with TTL equal to `WEBHOOK_REPLAY_WINDOW_SECS` (default: 300 seconds / 5 minutes). When an event arrives:

1. Attempt to increment the counter for this nonce
2. If the counter > 1, the event is a duplicate → reject silently, return OK
3. If the counter == 1, the event is new → proceed to process
4. Redis TTL ensures the nonce expires after 5 minutes, allowing the same event (message_id + event_type + email) to be processed again if it genuinely re-arrives after that window

**Survives Process Restarts:**
The Redis nonce is stored externally, so it persists across:
- Process restarts (SIGTERM, crashes)
- Deployments (as long as Redis remains operational)
- Horizontal scaling (shared across all API instances via Redis)

### Stage 2: Database Deduplication (Secondary Guard)

**Mechanism:**
The `email_events` table stores processed events with a unique constraint on:
```
(message_id, event_type, email)
```

When an event is processed:
1. Query the DB to check if this (message_id, event_type, email) combination already exists
2. If it exists, skip processing (return early, silently)
3. If it doesn't exist, proceed to store the event in the database

**Purpose:**
Provides fallback deduplication if:
- Redis cache is unavailable or flushed
- An event arrives after the Redis TTL window expires but still represents a genuine duplicate

**Example Scenario:**
1. SendGrid sends webhook: `{message_id: "123", event_type: "delivered", email: "user@ex.com"}` at 2024-09-28 10:00:00
2. API accepts and stores event in DB; Redis nonce expires at 10:05:00
3. SendGrid retries at 10:06:00 (after Redis nonce expired)
4. Redis nonce check misses (key expired)
5. DB dedup check queries and finds event from 10:00:00
6. Event is rejected at the DB level → no duplicate inserted

## Configuration

**Environment Variables:**

| Variable | Default | Purpose |
|---|---|---|
| `WEBHOOK_REPLAY_WINDOW_SECS` | `300` | Duration (seconds) that Redis nonces are retained |
| `SENDGRID_WEBHOOK_SECRET` | (required) | SendGrid ECDSA P-256 public key (base64 DER) for signature verification; HMAC shared secret only outside production |

**Production Checklist:**
- [ ] `SENDGRID_WEBHOOK_SECRET` is set to the ECDSA public key and matches the SendGrid dashboard
- [ ] `WEBHOOK_REPLAY_WINDOW_SECS` is appropriate for your retry policy (default 5 min)
- [ ] Redis is configured with data persistence or high availability
- [ ] Database backups are in place (email_events table contains audit trail)

## Payload Validation

**Size Enforcement:**
- Maximum payload size: 64 KiB
- Larger payloads are rejected before parsing

**Field Sanitization:**
- HTML tags are stripped from text fields
- JavaScript injection patterns are removed (e.g., `javascript:`, `onerror=`, `eval(`)
- Fields are truncated to maximum lengths:
  - Email / message_id / status: 254 characters
  - Reason / response / URL: 1024 characters
- Invalid or malicious payloads cause the entire webhook to be rejected

**Example Rejection:**
```json
{
  "email": "user@example.com",
  "event": "bounce",
  "timestamp": 1695825600,
  "reason": "<script>alert('xss')</script>Hard bounce"
}
```
Would be sanitized to:
```json
{
  "email": "user@example.com",
  "event": "bounce",
  "timestamp": 1695825600,
  "reason": "Hard bounce"
}
```

## Observability

**Logging:**

The webhook handler logs:
- Accepted deliveries at INFO level
- Duplicate/replay detections at WARN level (Redis nonce hits)
- Database dedup detections at WARN level (events already processed)
- Signature verification failures at ERROR level (auth failures)
- Payload parse failures at ERROR level (malformed JSON)

All logs include redacted email addresses to protect user privacy.

**Metrics:**

Webhook processing metrics are tracked:
- Request count and success rate
- Replay attack frequency (Redis nonce hits)
- Database dedup hits (stale duplicates)
- Signature verification failures

## Testing

**Unit Tests:**

See `services/api/src/email/webhook.rs` for comprehensive test coverage:
- `test_strip_html_helper_*`: HTML sanitization
- `test_*_field_truncated_at_max_len`: Field length enforcement
- `test_oversized_payload_is_rejected`: Payload size enforcement
- `test_replay_window_state_is_redis_backed_survives_process_restart`: Replay guard persistence
- `test_webhook_replay_detection_uses_composite_key`: Nonce key composition
- `test_webhook_has_secondary_db_dedup_guard_after_redis_ttl_expires`: Two-stage dedup

**Integration Testing:**

To test replay protection in a live environment:

1. Send a webhook POST to `/webhooks/sendgrid` with a valid SendGrid ECDSA signature
2. Verify the event is processed (check DB and logs)
3. Send the *exact same request* again immediately
4. Verify the duplicate is rejected (check logs for "Replay attack detected")
5. Wait 5 minutes (for Redis TTL to expire)
6. Send the same request again
7. Verify the DB secondary guard rejects it (check logs for "Duplicate event detected (DB)")

**Signature Verification Tests:**

See `services/api/tests/security_tests.rs` for coverage of:
- A correctly signed SendGrid payload is accepted
- A tampered payload (valid signature over different content) is rejected with 4xx
- An unsigned payload (missing signature header) is rejected with 4xx

## Incident Response

**If replay attacks are detected:**

1. Check logs for `"Replay attack detected (Redis nonce)"` entries
2. Verify the attacker is not forging valid SendGrid ECDSA signatures (check `X-Twilio-Email-Event-Webhook-Signature` in access logs)
3. If signatures are invalid, verify `SENDGRID_WEBHOOK_SECRET` still matches the SendGrid dashboard public key
4. Monitor metrics for spike in replay attempts

**If deduplication fails (same event processed twice):**

1. Check if Redis was unavailable (Redis connection errors in logs)
2. Verify database unique constraint on `(message_id, event_type, email)` exists
3. Check if the email_events table was recently truncated or rolled back
4. Review transaction logs to see if constraints were temporarily disabled

**If webhooks stop being processed:**

1. Verify `SENDGRID_WEBHOOK_SECRET` matches the dashboard ECDSA public key
2. Check Redis connectivity (admin endpoint logs)
3. Check database connectivity (admin endpoint logs)
4. Verify `/metrics` shows non-zero `sendgrid_webhook_*` counters

## Future Improvements

Potential enhancements:
- Batch multiple webhooks in a single request (currently processed concurrently but not batched server-side)
- Configurable TTL per event type (different retry windows for different event types)
- Webhook delivery acknowledgment to SendGrid (currently implicit 200 OK on HTTP response)
