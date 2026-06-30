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
