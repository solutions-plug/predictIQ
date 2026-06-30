# Email Replay Protection Strategy

## Overview

Webhook events from SendGrid are protected against replay attacks using a two-layer defence.

### Layer 1 — Redis nonce (active window)

On receipt, an atomic `INCR` is performed on a Redis key scoped to
`(message_id, event_type, recipient_email)` with a TTL equal to
`WEBHOOK_REPLAY_WINDOW_SECS` (default: 300 s, configurable via env).

- If the counter returns **1** → first time seen, proceed.
- If the counter returns **> 1** → replay within the active window, discard silently.

The `INCR + EXPIRE` operation is executed as a Lua script to be atomic — there is no
race window between checking and setting the key.

### Layer 2 — Database dedup (historical)

After the Redis check passes, a query against the `email_events` table verifies that no
matching `(message_id, event_type, recipient_email)` row already exists.  This catches
replays that arrive after the Redis TTL has expired (i.e. more than
`WEBHOOK_REPLAY_WINDOW_SECS` seconds after the original event).

## Server-side timestamp (`received_at`)

The `received_at` time used for all deduplication decisions is **always the server-side
wall-clock time** at which the request was processed.  The `timestamp` field present in
the SendGrid webhook payload is **not used for any security decision** because it
originates from an external, potentially spoofed source — an attacker who can replay a
webhook payload could set an arbitrary timestamp to bypass window-based checks.

## Deployment / migration notes

- **New deployments**: both layers are active immediately; no migration needed.
- **Existing deployments**: the DB dedup layer (Layer 2) remains effective for all
  historical events regardless of Redis state.  The Redis layer (Layer 1) only covers
  events received after the feature is deployed; events older than
  `WEBHOOK_REPLAY_WINDOW_SECS` rely on the DB layer.
- **Redis TTL configuration**: tune `WEBHOOK_REPLAY_WINDOW_SECS` to balance replay
  protection window vs. Redis memory usage.  The default (300 s) matches the SendGrid
  retry window.
