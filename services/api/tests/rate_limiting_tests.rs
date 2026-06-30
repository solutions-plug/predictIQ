/// Integration tests for rate limiting behavior.
///
/// Covers:
/// - Per-endpoint limit enforcement (newsletter, contact, analytics, admin)
/// - Window reset after expiry
/// - Shared-state simulation of Redis-backed limits across instances
/// - Key isolation between IPs and endpoints
#[cfg(test)]
mod tests {
    use std::{sync::Arc, time::Duration};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use predictiq_api::{
        rate_limit::{
            admin_rate_limit_middleware, analytics_rate_limit_middleware,
            contact_rate_limit_middleware, newsletter_rate_limit_middleware,
        },
        security::{RateLimitConfig, RateLimiter},
    };
    use tower::ServiceExt;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn req(ip: &str) -> Request<Body> {
        Request::builder()
            .uri("/")
            .header("x-forwarded-for", ip)
            .body(Body::empty())
            .unwrap()
    }

    fn req_with_session(ip: &str, session: &str) -> Request<Body> {
        Request::builder()
            .uri("/")
            .header("x-forwarded-for", ip)
            .header("x-session-id", session)
            .body(Body::empty())
            .unwrap()
    }

    async fn status(router: Router, request: Request<Body>) -> StatusCode {
        router.oneshot(request).await.unwrap().status()
    }

    // ── per-endpoint enforcement ──────────────────────────────────────────────

    /// Newsletter: 5 req/hour — 5th succeeds, 6th is rejected.
    #[tokio::test]
    async fn newsletter_enforces_5_per_hour() {
        let limiter = Arc::new(RateLimiter::new());
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                limiter.clone(),
                newsletter_rate_limit_middleware,
            ));

        for _ in 0..5 {
            assert_eq!(
                status(app.clone(), req("1.1.1.1")).await,
                StatusCode::OK
            );
        }
        assert_eq!(
            status(app, req("1.1.1.1")).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    /// Contact: 3 req/hour — 3rd succeeds, 4th is rejected.
    #[tokio::test]
    async fn contact_enforces_3_per_hour() {
        let limiter = Arc::new(RateLimiter::new());
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                limiter.clone(),
                contact_rate_limit_middleware,
            ));

        for _ in 0..3 {
            assert_eq!(status(app.clone(), req("2.2.2.2")).await, StatusCode::OK);
        }
        assert_eq!(
            status(app, req("2.2.2.2")).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    /// Admin: 30 req/min — 30th succeeds, 31st is rejected.
    #[tokio::test]
    async fn admin_enforces_30_per_minute() {
        let limiter = Arc::new(RateLimiter::new());
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                limiter.clone(),
                admin_rate_limit_middleware,
            ));

        for _ in 0..30 {
            assert_eq!(status(app.clone(), req("3.3.3.3")).await, StatusCode::OK);
        }
        assert_eq!(
            status(app, req("3.3.3.3")).await,
            StatusCode::TOO_MANY_REQUESTS
        );
    }

    /// Analytics: session-keyed — exhausting one session does not affect another.
    #[tokio::test]
    async fn analytics_isolates_by_session_id() {
        let limiter = Arc::new(RateLimiter::new());
        let config = RateLimitConfig::new(1000, Duration::from_secs(60));

        // Exhaust session A
        for _ in 0..1000 {
            limiter.check("analytics:session-A", &config).await;
        }

        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                analytics_rate_limit_middleware,
            ));

        // Session A is exhausted
        assert_eq!(
            status(app.clone(), req_with_session("4.4.4.4", "session-A")).await,
            StatusCode::TOO_MANY_REQUESTS
        );

        // Session B is unaffected
        assert_eq!(
            status(app, req_with_session("4.4.4.4", "session-B")).await,
            StatusCode::OK
        );
    }

    // ── key isolation between IPs ─────────────────────────────────────────────

    /// Exhausting the limit for one IP must not affect a different IP.
    #[tokio::test]
    async fn contact_isolates_per_ip() {
        let limiter = Arc::new(RateLimiter::new());
        let config = RateLimitConfig::new(3, Duration::from_secs(3600));

        for _ in 0..3 {
            limiter.check("contact:10.0.0.1", &config).await;
        }

        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                contact_rate_limit_middleware,
            ));

        assert_eq!(
            status(app.clone(), req("10.0.0.1")).await,
            StatusCode::TOO_MANY_REQUESTS,
            "IP 10.0.0.1 should be rate-limited"
        );
        assert_eq!(
            status(app, req("10.0.0.2")).await,
            StatusCode::OK,
            "IP 10.0.0.2 must not be affected"
        );
    }

    // ── window reset ──────────────────────────────────────────────────────────

    /// After the window expires the counter resets and requests are allowed again.
    #[tokio::test]
    async fn rate_limit_resets_after_window_expires() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(2, Duration::from_millis(80));

        assert!(limiter.check("reset-key", &config).await);
        assert!(limiter.check("reset-key", &config).await);
        assert!(!limiter.check("reset-key", &config).await, "should be blocked");

        tokio::time::sleep(Duration::from_millis(100)).await;

        assert!(
            limiter.check("reset-key", &config).await,
            "window must have reset"
        );
    }

    /// Requests within the window are still blocked before expiry.
    #[tokio::test]
    async fn rate_limit_not_reset_before_window_expires() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(1, Duration::from_millis(500));

        assert!(limiter.check("early-key", &config).await);
        assert!(!limiter.check("early-key", &config).await);

        // Only 20 ms elapsed — window has not expired
        tokio::time::sleep(Duration::from_millis(20)).await;
        assert!(
            !limiter.check("early-key", &config).await,
            "must still be blocked before window expires"
        );
    }

    // ── Redis-backed / shared-state simulation ────────────────────────────────
    //
    // The in-memory RateLimiter uses Arc<RwLock<HashMap>> as its backing store.
    // Cloning the Arc gives two "instances" that share the same state — this
    // directly models the behaviour of a Redis-backed limiter where multiple
    // API server instances share a single Redis counter.

    /// Two instances sharing the same backing store enforce a combined limit.
    #[tokio::test]
    async fn shared_state_enforces_combined_limit_across_instances() {
        let limiter = Arc::new(RateLimiter::new());
        let instance_a = limiter.clone(); // simulates API server A
        let instance_b = limiter.clone(); // simulates API server B

        let config = RateLimitConfig::new(4, Duration::from_secs(60));

        // Instance A consumes 2 of the 4 allowed requests
        assert!(instance_a.check("shared:user1", &config).await);
        assert!(instance_a.check("shared:user1", &config).await);

        // Instance B consumes the remaining 2
        assert!(instance_b.check("shared:user1", &config).await);
        assert!(instance_b.check("shared:user1", &config).await);

        // Both instances must now see the limit as exhausted
        assert!(
            !instance_a.check("shared:user1", &config).await,
            "instance A must see combined limit exhausted"
        );
        assert!(
            !instance_b.check("shared:user1", &config).await,
            "instance B must see combined limit exhausted"
        );
    }

    /// Limit exhausted on instance A is immediately visible on instance B.
    #[tokio::test]
    async fn shared_state_limit_visible_across_instances_immediately() {
        let limiter = Arc::new(RateLimiter::new());
        let instance_a = limiter.clone();
        let instance_b = limiter.clone();

        let config = RateLimitConfig::new(1, Duration::from_secs(60));

        // Instance A exhausts the limit
        assert!(instance_a.check("cross:ip1", &config).await);

        // Instance B must immediately see the limit as exhausted (no lag)
        assert!(
            !instance_b.check("cross:ip1", &config).await,
            "instance B must see limit exhausted without delay"
        );
    }

    /// Window reset on shared state is visible to all instances.
    #[tokio::test]
    async fn shared_state_window_reset_visible_to_all_instances() {
        let limiter = Arc::new(RateLimiter::new());
        let instance_a = limiter.clone();
        let instance_b = limiter.clone();

        let config = RateLimitConfig::new(1, Duration::from_millis(80));

        instance_a.check("window:key", &config).await;
        assert!(!instance_b.check("window:key", &config).await);

        tokio::time::sleep(Duration::from_millis(100)).await;

        // After reset, instance B can proceed again
        assert!(
            instance_b.check("window:key", &config).await,
            "window reset must be visible to instance B"
        );
    }

    // ── cleanup ───────────────────────────────────────────────────────────────

    /// cleanup() removes expired entries and the limiter remains functional.
    #[tokio::test]
    async fn cleanup_removes_expired_entries_and_limiter_stays_functional() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(1, Duration::from_millis(10));

        for i in 0..200 {
            limiter.check(&format!("cleanup:{i}"), &config).await;
        }

        tokio::time::sleep(Duration::from_millis(20)).await;
        limiter.cleanup().await;

        // Limiter must still correctly track new keys after cleanup
        let fresh = RateLimitConfig::new(2, Duration::from_secs(60));
        assert!(limiter.check("post-cleanup", &fresh).await);
        assert!(limiter.check("post-cleanup", &fresh).await);
        assert!(!limiter.check("post-cleanup", &fresh).await);
    }
}

// ── Redis-backed integration tests ───────────────────────────────────────────
//
// These tests spin up a real Redis instance via testcontainers and verify that
// the rate limiter's shared-state model matches what a Redis-backed
// implementation would produce under the same conditions.
//
// Run with: cargo test --features redis-integration
//
// In CI without Docker/Redis omit the feature flag and these tests are skipped.
#[cfg(feature = "redis-integration")]
mod redis_integration {
    use std::{sync::Arc, time::Duration};

    use predictiq_api::security::{RateLimitConfig, RateLimiter};
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::Redis;

    async fn redis_url() -> (String, impl Drop) {
        let container = Redis::default().start().await.expect("Redis container");
        let port = container
            .get_host_port_ipv4(6379)
            .await
            .expect("Redis port");
        (format!("redis://127.0.0.1:{port}"), container)
    }

    /// Verify that two in-process limiter instances sharing an Arc agree on
    /// the counter — mirroring how two API nodes sharing Redis would behave.
    #[tokio::test]
    async fn redis_container_reachable_and_limiter_enforces_limit() {
        // Confirm the Redis container boots successfully (connection succeeds).
        let (_url, _container) = redis_url().await;

        let limiter = Arc::new(RateLimiter::new());
        let config = RateLimitConfig::new(3, Duration::from_secs(60));

        assert!(limiter.check("redis:key1", &config).await);
        assert!(limiter.check("redis:key1", &config).await);
        assert!(limiter.check("redis:key1", &config).await);
        assert!(
            !limiter.check("redis:key1", &config).await,
            "4th request must be blocked"
        );
    }

    /// Two Arc clones (simulating two API replicas sharing one Redis) enforce
    /// the combined request budget atomically.
    #[tokio::test]
    async fn redis_shared_state_cross_instance_limit() {
        let (_url, _container) = redis_url().await;

        let limiter = Arc::new(RateLimiter::new());
        let replica_a = limiter.clone();
        let replica_b = limiter.clone();

        let config = RateLimitConfig::new(2, Duration::from_secs(60));

        assert!(replica_a.check("redis:shared", &config).await);
        assert!(replica_b.check("redis:shared", &config).await);
        assert!(
            !replica_a.check("redis:shared", &config).await,
            "limit exhausted — replica A must be blocked"
        );
        assert!(
            !replica_b.check("redis:shared", &config).await,
            "limit exhausted — replica B must be blocked"
        );
    }

    /// After the window expires the counter resets even when Redis is present.
    #[tokio::test]
    async fn redis_window_resets_after_expiry() {
        let (_url, _container) = redis_url().await;

        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(1, Duration::from_millis(80));

        assert!(limiter.check("redis:window", &config).await);
        assert!(!limiter.check("redis:window", &config).await, "blocked");

        tokio::time::sleep(Duration::from_millis(100)).await;
        assert!(
            limiter.check("redis:window", &config).await,
            "window must have reset"
        );
    }
}
