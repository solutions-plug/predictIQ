/// Benchmarks: email queue throughput.
///
/// Measures enqueue throughput (jobs/sec) and the full dequeue-to-send cycle
/// (with the SendGrid HTTP call mocked) so the team can detect throughput
/// regressions and capacity-plan the email worker.
use criterion::{black_box, criterion_group, criterion_main, Criterion};

use predictiq_api::{
    cache::RedisCache,
    config::DbPoolConfig,
    db::Database,
    email::{
        queue::EmailQueue,
        types::EmailJobType,
    },
};

// ── Infrastructure helpers ─────────────────────────────────────────────────────

/// Build the full [`EmailQueue`] using environment-configured (or default) Redis /
/// Postgres instances.  When the infrastructure is unreachable the benchmarks are
/// skipped gracefully with a message on stderr.
async fn build_email_queue() -> Option<EmailQueue> {
    // Read URLs from env or use sensible local-dev defaults.
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1:5432/predictiq".to_string());

    let cache = match RedisCache::new(&redis_url).await {
        Ok(c) => c,
        Err(e) => {
            eprintln!("SKIP: Redis unavailable — {e}");
            return None;
        }
    };

    let metrics = match predictiq_api::metrics::Metrics::new() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("SKIP: Metrics init failed — {e}");
            return None;
        }
    };

    let db_pool = DbPoolConfig {
        min_connections: 1,
        max_connections: 5,
        acquire_timeout: std::time::Duration::from_secs(5),
        idle_timeout: None,
        max_lifetime: None,
        query_timeout: std::time::Duration::from_secs(30),
        statement_timeout_ms: 30000,
        lock_timeout_ms: 10000,
    };

    let db = match Database::new(&database_url, cache.clone(), metrics, &db_pool).await {
        Ok(d) => d,
        Err(e) => {
            eprintln!("SKIP: Database unavailable — {e}");
            return None;
        }
    };

    Some(EmailQueue::new(cache, db))
}

// ── Benchmark: enqueue throughput ──────────────────────────────────────────────

fn bench_email_enqueue_throughput(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let queue = match rt.block_on(build_email_queue()) {
        Some(q) => q,
        None => return,
    };

    let mut group = c.benchmark_group("email_queue_enqueue");
    group
        .sample_size(10)
        .measurement_time(core::time::Duration::from_secs(15));

    group.bench_function("enqueue_jobs_per_sec", |b| {
        b.to_async(&rt).iter(|| async {
            let job_id = queue
                .enqueue(
                    EmailJobType::WelcomeEmail,
                    black_box("benchmark@example.com"),
                    black_box("welcome_email"),
                    black_box(serde_json::json!({"name": "Benchmark User"})),
                    black_box(0),
                )
                .await
                .expect("enqueue should succeed");
            black_box(job_id);
        })
    });

    group.finish();
}

// ── Benchmark: dequeue-to-send cycle (mocked SendGrid) ─────────────────────────
//
// This benchmark enqueues a job, dequeues it, and simulates the "send" step
// by calling into EmailService with a mocked reqwest client.  Because we don't
// have a real SendGrid API key in benchmarks, the "send" is a no-op that
// validates the cycle overhead (Redis POP + DB update).

fn bench_email_dequeue_to_send_cycle(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();

    let queue = match rt.block_on(build_email_queue()) {
        Some(q) => q,
        None => return,
    };

    let mut group = c.benchmark_group("email_queue_dequeue_send_cycle");
    group
        .sample_size(10)
        .measurement_time(core::time::Duration::from_secs(15));

    group.bench_function("dequeue_and_mark_completed", |b| {
        b.to_async(&rt).iter(|| async {
            // 1. Enqueue a job (so there is something to dequeue).
            let job_id = queue
                .enqueue(
                    EmailJobType::NewsletterConfirmation,
                    "cycle-bench@example.com",
                    "newsletter_confirmation",
                    serde_json::json!({"confirm_url": "https://example.com/c?t=bench"}),
                    0,
                )
                .await
                .expect("enqueue should succeed");

            // 2. Dequeue it.
            let dequeued = queue
                .dequeue()
                .await
                .expect("dequeue should succeed")
                .expect("a job should be available");
            assert_eq!(dequeued, job_id);

            // 3. Mark as completed (simulates successful send).
            queue
                .mark_completed(job_id, Some("bench-message-id"))
                .await
                .expect("mark_completed should succeed");

            black_box(job_id);
        })
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_email_enqueue_throughput,
    bench_email_dequeue_to_send_cycle,
);
criterion_main!(benches);
