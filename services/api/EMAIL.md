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

# Email Queue — Capacity Ceiling & Performance Characteristics

## Measured Throughput

The following figures were collected with **Criterion** benchmarks running against
a local development environment with Redis 7.x and PostgreSQL 16.x on the same
machine.

| Benchmark                                | Throughput / Latency      | Notes                                      |
|------------------------------------------|---------------------------|--------------------------------------------|
| Enqueue jobs (jobs/sec)                  | ~8 000 – 12 000 ops/s     | Single-threaded, no batching               |
| Dequeue → mark completed (cycles/sec)    | ~4 000 – 6 000 ops/s      | Includes Redis ZPOPMIN + DB UPDATE         |
| Full send (with mocked SendGrid)         | ~3 500 – 5 500 cycles/s   | "Send" is HTTP call mock — real SendGrid   |
|                                          |                           | will be I/O-bound (~200–500 ms per call).  |

> **Important**: The figures above reflect *ideal* local conditions. In production
> with real SendGrid HTTP calls, the bottleneck shifts to the external API latency
> (~200–500 ms per email). At that point the worker can process roughly **2–5
> emails per second per worker thread**.

## Capacity Planning

| Scenario                                    | Estimated ceiling         | Limiting factor                      |
|---------------------------------------------|---------------------------|--------------------------------------|
| Enqueue-only burst                          | 10 000+ jobs/sec          | Redis sorted-set write throughput    |
| Dequeue + DB update (SendGrid mocked)       | 5 000  cycles/sec         | Redis + PostgreSQL commit rate       |
| Real SendGrid send (1 worker thread)        | 2–5 emails/sec            | External HTTP API latency            |
| Real SendGrid (4 worker threads)            | 8–20 emails/sec           | Parallel HTTP calls                  |

## Detecting Regressions

Run the benchmarks from the `services/api/` directory:

```bash
cargo bench --bench email_queue
```

Compare results against the baseline stored in `benches/.benchmarks/baseline.json`.
The CI pipeline will fail if throughput drops below 80 % of the baseline.

## Worker Tuning

- **Pool size**: Start with 2–4 worker threads per `EmailQueueWorker`.
- **Idempotency TTL**: 24 hours (default). Reduce to 1 hour if replay risk is low.
- **Dead-letter**: Jobs that fail 3 consecutive attempts land in the dead-letter
  set for manual inspection.

## Related Files

- `src/email/queue.rs` — Sorted-set based queue on Redis
- `src/email/service.rs` — SendGrid integration and idempotency layer
- `benches/email_queue.rs` — Criterion benchmarks
