/// Integration tests for the blockchain sync worker (issue #976).
///
/// Covers:
///  - RPC timeout → worker retries and resumes from the correct ledger
///  - Connection reset → worker retries without crashing
///  - Ledger gap detection → warning metric is incremented
///  - Replay crossing the confirmed tip → skipped events are reported, not cached as completed
///
/// All tests require a live Redis instance (started via testcontainers).
/// Run with: cargo test --features redis-integration
#[cfg(feature = "redis-integration")]
mod tests {
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            Arc,
        },
        time::Duration,
    };

    use axum::{routing::post, Json, Router};
    use predictiq_api::{blockchain::BlockchainClient, cache::RedisCache, metrics::Metrics};
    use reqwest::Client;
    use serde_json::{json, Value};
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::Redis;
    use tokio::{net::TcpListener, sync::Mutex};

    // ── helpers ───────────────────────────────────────────────────────────────

    async fn start_redis() -> (String, impl Drop) {
        let container = Redis::default().start().await.expect("Redis container failed to start");
        let port = container
            .get_host_port_ipv4(6379)
            .await
            .expect("Redis port");
        (format!("redis://127.0.0.1:{port}"), container)
    }

    async fn make_cache(redis_url: &str) -> RedisCache {
        RedisCache::new(redis_url).await.expect("RedisCache::new")
    }

    fn make_metrics() -> Metrics {
        Metrics::new().expect("Metrics::new")
    }

    /// Start an axum mock RPC server that returns `responses` in sequence.
    /// Each response is a complete JSON-RPC result envelope.
    async fn start_mock_rpc(responses: Vec<Value>) -> String {
        let queue = Arc::new(Mutex::new(responses));

        let app = Router::new().route(
            "/",
            post(move |Json(_body): Json<Value>| {
                let queue = queue.clone();
                async move {
                    let mut q = queue.lock().await;
                    let resp = if q.is_empty() {
                        // Return a default "no ledger" response once the queue is drained.
                        json!({ "result": { "latestLedger": { "sequence": 0 } } })
                    } else {
                        q.remove(0)
                    };
                    Json(resp)
                }
            }),
        );

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}");

        tokio::spawn(async move {
            axum::serve(listener, app).await.unwrap();
        });

        url
    }

    /// Start a server that delays all responses indefinitely (simulates timeout).
    async fn start_timeout_rpc() -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}");

        tokio::spawn(async move {
            let app = Router::new().route(
                "/",
                post(|| async {
                    // Sleep longer than the client's configured timeout.
                    tokio::time::sleep(Duration::from_secs(60)).await;
                    Json(json!({}))
                }),
            );
            axum::serve(listener, app).await.unwrap();
        });

        url
    }

    /// Start a server that accepts the TCP connection then immediately drops it
    /// (simulates a connection reset / RST).
    async fn start_reset_rpc(accept_count: Arc<AtomicUsize>) -> String {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        let url = format!("http://127.0.0.1:{port}");

        tokio::spawn(async move {
            loop {
                if let Ok((stream, _)) = listener.accept().await {
                    accept_count.fetch_add(1, Ordering::SeqCst);
                    // Dropping the stream immediately closes it — connection reset.
                    drop(stream);
                }
            }
        });

        url
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    /// The worker retries the configured number of times when every RPC call
    /// exceeds the client timeout, then surfaces an error without panicking.
    #[tokio::test]
    async fn retry_on_rpc_timeout() {
        let (redis_url, _container) = start_redis().await;
        let cache = make_cache(&redis_url).await;
        let metrics = make_metrics();

        let rpc_url = start_timeout_rpc().await;

        // Use a 200 ms client timeout so the test completes quickly.
        let http = Client::builder()
            .timeout(Duration::from_millis(200))
            .connect_timeout(Duration::from_millis(200))
            .build()
            .unwrap();

        let client = BlockchainClient::new_for_test(rpc_url, cache, metrics, http, 3);

        // health_check_cached() calls latest_ledger() which will time out.
        // After `retry_attempts` failures the call must return Err, not panic.
        let result = client.health_check_cached().await;
        assert!(result.is_err(), "expected Err after all retries exhausted");
    }

    /// The worker retries when the TCP connection is reset by the peer.
    /// We verify that multiple connection attempts are made (retry behaviour)
    /// before the call ultimately fails.
    #[tokio::test]
    async fn retry_on_connection_reset() {
        let (redis_url, _container) = start_redis().await;
        let cache = make_cache(&redis_url).await;
        let metrics = make_metrics();

        let accept_count = Arc::new(AtomicUsize::new(0));
        let rpc_url = start_reset_rpc(accept_count.clone()).await;

        let http = Client::builder()
            .timeout(Duration::from_millis(300))
            .connect_timeout(Duration::from_millis(300))
            .build()
            .unwrap();

        let retry_attempts = 3_u32;
        let client =
            BlockchainClient::new_for_test(rpc_url, cache, metrics, http, retry_attempts);

        let result = client.health_check_cached().await;
        assert!(result.is_err(), "should fail after connection resets");

        // The server must have received at least `retry_attempts` connections.
        let total_accepts = accept_count.load(Ordering::SeqCst);
        assert!(
            total_accepts >= retry_attempts as usize,
            "expected >= {retry_attempts} connection attempts, got {total_accepts}"
        );
    }

    /// After a transient failure the worker retries and, on a subsequent
    /// success, resumes from the correct ledger sequence (not from zero).
    #[tokio::test]
    async fn resumes_from_correct_ledger_after_retry() {
        let (redis_url, _container) = start_redis().await;
        let cache = make_cache(&redis_url).await;
        let metrics = make_metrics();

        // First call returns latest ledger = 500.
        // Subsequent calls (for events, market data, etc.) return minimal stubs.
        let latest_ledger_response = json!({
            "result": {
                "latestLedger": { "sequence": 500_u32 }
            }
        });
        let events_response = json!({
            "result": {
                "events": [],
                "latestLedger": 500_u32
            }
        });

        let rpc_url = start_mock_rpc(vec![
            latest_ledger_response.clone(),
            events_response,
            latest_ledger_response, // for reorg check
        ])
        .await;

        let http = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        let client = Arc::new(BlockchainClient::new_for_test(
            rpc_url, cache, metrics, http, 2,
        ));

        // sync_once(cursor=490) — co

    /// A replay that crosses the confirmed tip must report the events beyond
    /// the tip as skipped, exclude them from the processed count, and must not
    /// write a "completed" cache entry (issue #1894).
    #[tokio::test]
    async fn replay_reports_skipped_events_beyond_confirmed_tip() {
        let (redis_url, _container) = start_redis().await;
        let cache = make_cache(&redis_url).await;
        let metrics = make_metrics();

        // Confirmed tip is ledger 500. The replay range extends past it, so
        // events at ledgers 501 and 502 must be skipped.
        let latest_ledger_response = json!({
            "result": {
                "latestLedger": { "sequence": 500_u32 }
            }
        });
        let events_response = json!({
            "result": {
                "events": [
                    { "ledger": 499_u32, "id": "evt-499" },
                    { "ledger": 500_u32, "id": "evt-500" },
                    { "ledger": 501_u32, "id": "evt-501" },
                    { "ledger": 502_u32, "id": "evt-502" }
                ],
                "latestLedger": 502_u32
            }
        });

        let rpc_url = start_mock_rpc(vec![
            latest_ledger_response.clone(),
            events_response,
            latest_ledger_response,
        ])
        .await;

        let http = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        let client = BlockchainClient::new_for_test(rpc_url, cache, metrics, http, 2);

        let result = client
            .replay_events(498)
            .await
            .expect("replay_events should succeed");

        // Only events at or below the confirmed tip count as processed.
        assert_eq!(result.processed, 2, "only events up to the tip are processed");
        assert_eq!(result.skipped, 2, "events beyond the tip must be reported as skipped");

        // A partial replay must not be cached as completed.
        let cached = client
            .replay_status()
            .await
            .expect("replay_status should be readable");
        assert_ne!(
            cached.as_deref(),
            Some("completed"),
            "completed status must not be cached when events were skipped"
        );
    }
}
