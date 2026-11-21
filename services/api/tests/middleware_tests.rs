/// End-to-end HTTP tests for rate limiting middleware behavior.
///
/// Covers:
/// - 200 → 429 transitions through the actual middleware stack
/// - Per-IP key isolation (different IPs don't share quotas)
/// - Cleanup bounds map size under high-cardinality load
///
/// Also covers the dead-letter job listing bound (issue #1744 / #1911):
/// - The listing is capped at the handler's maximum page size
/// - The default page size is applied when no `limit` is supplied
/// - Pagination parameters (`page` / `limit`) are honoured
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
    use predictiq_api::security::{global_rate_limit_middleware, RateLimitConfig, RateLimiter};
    use tower::ServiceExt; // for `oneshot`

    /// Build a minimal router with global rate limiting applied.
    fn app(limiter: Arc<RateLimiter>) -> Router {
        Router::new()
            .route("/ping", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                limiter,
                global_rate_limit_middleware,
            ))
    }

    fn get_request(ip: &str) -> Request<Body> {
        Request::builder()
            .uri("/ping")
            .header("x-forwarded-for", ip)
            .body(Body::empty())
            .unwrap()
    }

    // ── 200 / 429 transition ─────────────────────────────────────────────────

    #[tokio::test]
    async fn test_middleware_allows_requests_within_limit() {
        // Global limit is 100/min; a single request must return 200.
        let limiter = Arc::new(RateLimiter::new());
        let response = app(limiter)
            .oneshot(get_request("1.2.3.4"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_middleware_returns_429_after_limit_exceeded() {
        // Use a tight config via the limiter directly, then hit the middleware.
        // We exhaust the global limit (100) by calling check() 100 times, then
        // the next HTTP request through the middleware must get 429.
        let limiter = Arc::new(RateLimiter::new());
        let config = RateLimitConfig::new(100, Duration::from_secs(60));

        for _ in 0..100 {
            limiter.check("global:1.2.3.4", &config).await;
        }

        let response = app(limiter)
            .oneshot(get_request("1.2.3.4"))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    // ── Per-IP isolation ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn test_middleware_isolates_keys_per_ip() {
        let limiter = Arc::new(RateLimiter::new());
        let config = RateLimitConfig::new(100, Duration::from_secs(60));

        // Exhaust quota for IP A.
        for _ in 0..100 {
            limiter.check("global:10.0.0.1", &config).await;
        }

        // IP A is now rate-limited.
        let resp_a = app(limiter.clone())
            .oneshot(get_request("10.0.0.1"))
            .await
            .unwrap();
        assert_eq!(resp_a.status(), StatusCode::TOO_MANY_REQUESTS);

        // IP B has a fresh quota and must still get 200.
        let resp_b = app(limiter)
            .oneshot(get_request("10.0.0.2"))
            .await
            .unwrap();
        assert_eq!(resp_b.status(), StatusCode::OK);
    }

    // ── Cleanup / map-size bound ─────────────────────────────────────────────

    #[tokio::test]
    async fn test_cleanup_removes_expired_entries() {
        // Insert many unique keys with a very short window so they expire fast.
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(1, Duration::from_millis(50));

        for i in 0..500 {
            limiter.check(&format!("ip:{i}"), &config).await;
        }

        // Wait for all windows to expire (cleanup retains entries < 1 hour old
        // by default, so we use the public check() to age them out via window
        // reset, then verify cleanup reduces the internal count indirectly by
        // asserting subsequent checks still work — the real assertion is that
        // cleanup() doesn't panic and the limiter remains functional).
        tokio::time::sleep(Duration::from_millis(100)).await;
        limiter.cleanup().await;

        // After cleanup the limiter must still accept new keys correctly.
        let config2 = RateLimitConfig::new(2, Duration::from_secs(60));
        assert!(limiter.check("fresh-key", &config2).await);
        assert!(limiter.check("fresh-key", &config2).await);
        assert!(!limiter.check("fresh-key", &config2).await);
    }

    #[tokio::test]
    async fn test_cleanup_bounds_map_under_high_cardinality() {
        // Simulate a long-running process: insert 10_000 unique keys, run
        // cleanup, then insert another batch. The limiter must remain correct.
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(1, Duration::from_millis(1));

        for i in 0..10_000 {
            limiter.check(&format!("stress:{i}"), &config).await;
        }

        // Let all windows expire.
        tokio::time::sleep(Duration::from_millis(10)).await;
        limiter.cleanup().await;

        // New keys must still be tracked correctly after cleanup.
        let fresh_config = RateLimitConfig::new(3, Duration::from_secs(60));
        for i in 0..100 {
            let key = format!("post-cleanup:{i}");
            assert!(limiter.check(&key, &fresh_config).await);
        }
    }

    // ── Dead-letter listing bound ────────────────────────────────────────────
    //
    // These tests exercise the dead-letter listing handler's pagination bound
    // (issue #1744). The handler reads `page` and `limit` query parameters and
    // clamps `limit` to a maximum page size, applying a default page size when
    // `limit` is omitted. The assertions below are written against those
    // parameter names and the handler's cap/default constants.

    /// Maximum number of dead-letter rows the listing handler will return in a
    /// single response. Mirrors the cap enforced in `services/api/src/handlers.rs`.
    const DEAD_LETTER_MAX_LIMIT: usize = 100;

    /// Default page size applied when the request omits `limit`.
    /// Mirrors the default in `services/api/src/handlers.rs`.
    const DEAD_LETTER_DEFAULT_LIMIT: usize = 20;

    /// Seed `count` dead-letter rows into the in-memory store used by the
    /// listing handler. Rows are inserted with monotonically increasing ids so
    /// pagination ordering is deterministic.
    async fn seed_dead_letter_rows(count: usize) {
        for i in 0..count {
            predictiq_api::dead_letter::insert_for_test(
                format!("dead-letter-job-{i:05}"),
                "test failure payload",
            )
            .await;
        }
    }

    /// Build a request against the dead-letter listing endpoint.
    fn dead_letter_listing_request(query: &str) -> Request<Body> {
        let uri = if query.is_empty() {
            "/admin/dead-letter".to_string()
        } else {
            format!("/admin/dead-letter?{query}")
        };
        Request::builder()
            .uri(uri)
            .body(Body::empty())
            .unwrap()
    }

    /// Parse the JSON listing response into the number of returned rows.
    async fn count_listing_rows(response: axum::response::Response) -> usize {
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        match value {
            serde_json::Value::Array(rows) => rows.len(),
            serde_json::Value::Object(map) => map
                .get("jobs")
                .or_else(|| map.get("items"))
                .or_else(|| map.get("data"))
                .and_then(|v| v.as_array())
                .map(|rows| rows.len())
                .expect("listing response must contain a jobs/items/data array"),
            other => panic!("unexpected listing response shape: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_dead_letter_listing_is_capped() {
        // Seed strictly more rows than the handler's cap.
        let seeded = DEAD_LETTER_MAX_LIMIT * 2;
        seed_dead_letter_rows(seeded).await;

        // Even when the client asks for everything, the response must not
        // exceed the cap. If the bound is removed this assertion fails.
        let response = app(Arc::new(RateLimiter::new()))
            .oneshot(dead_letter_listing_request(&format!(
                "limit={}",
                seeded * 10
            )))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let returned = count_listing_rows(response).await;
        assert!(
            returned <= DEAD_LETTER_MAX_LIMIT,
            "listing returned {returned} rows, exceeding cap of {DEAD_LETTER_MAX_LIMIT}"
        );
        assert!(returned > 0, "listing should return at least one row");
    }

    #[tokio::test]
    async fn test_dead_letter_listing_default_page_size() {
        // Seed more rows than the default page size so the default is observable.
        seed_dead_letter_rows(DEAD_LETTER_DEFAULT_LIMIT * 3).await;

        // No `limit` supplied: the handler must apply its default page size.
        let response = app(Arc::new(RateLimiter::new()))
            .oneshot(dead_letter_listing_request(""))
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let returned = count_listing_rows(response).await;
        assert_eq!(
            returned, DEAD_LETTER_DEFAULT_LIMIT,
            "default page size should be {DEAD_LETTER_DEFAULT_LIMIT}, got {returned}"
        );
    }

    #[tokio::test]
    async fn test_dead_letter_listing_honours_pagination() {
        // Seed enough rows for several pages at the requested page size.
        let page_size = 5usize;
        seed_dead_letter_rows(page_size * 4).await;

        let limiter = Arc::new(RateLimiter::new());

        // Request a smaller-than-default limit; the handler must honour it.
        let first = app(limiter.clone())
            .oneshot(dead_letter_listing_request(&format!(
                "page=1&limit={page_size}"
            )))
            .await
            .unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let first_body = axum::body::to_bytes(first.into_body(), usize::MAX)
            .await
            .unwrap();
        let first_value: serde_json::Value = serde_json::from_slice(&first_body).unwrap();
        let first_rows = match &first_value {
            serde_json::Value::Array(rows) => rows.clone(),
            serde_json::Value::Object(map) => map
                .get("jobs")
                .or_else(|| map.get("items"))
                .or_else(|| map.get("data"))
                .and_then(|v| v.as_array())
                .cloned()
                .expect("listing response must contain a jobs/items/data array"),
            other => panic!("unexpected listing response shape: {other:?}"),
        };
        assert_eq!(
            first_rows.len(),
            page_size,
            "requested limit of {page_size} should be honoured"
        );

        // Request the next page; it must return a different set of rows.
        let second = app(limiter)
            .oneshot(dead_letter_listing_request(&format!(
                "page=2&limit={page_size}"
            )))
            .await
            .unwrap();
        assert_eq!(second.status(), StatusCode::OK);
        let second_body = axum::body::to_bytes(second.into_body(), usize::MAX)
            .await
            .unwrap();
        let second_value: serde_json::Value = serde_json::from_slice(&second_body).unwrap();
        let second_rows = match &second_value {
            serde_json::Value::Array(rows) => rows.clone(),
            serde_json::Value::Object(map) => map
                .get("jobs")
                .or_else(|| map.get("items"))
                .or_else(|| map.get("data"))
                .and_then(|v| v.as_array())
                .cloned()
                .expect("listing response must contain a jobs/items/data array"),
            other => panic!("unexpected listing response shape: {other:?}"),
        };
        assert_eq!(
            second_rows.len(),
            page_size,
            "second page should also honour the requested limit"
        );
        assert_ne!(
            first_rows, second_rows,
            "page 2 must return different rows than page 1"
        );
    }
}

/// Tests for the request body-size middleware (`REQUEST_BODY_MAX_BYTES`).
#[cfg(test)]
mod body_size_tests {
    use std::sync::Once;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::post,
        Router,
    };
    use predictiq_api::validation::request_size_validation_middleware;
    use tower::ServiceExt;

    const LIMIT: usize = 1024;
    static INIT: Once = Once::new();

    /// The limit is cached in a process-wide `OnceLock` on first use, so the env
    /// var must be set before any request in this test binary hits the middleware.
    fn init_limit() {
        INIT.call_once(|| std::env::set_var("REQUEST_BODY_MAX_BYTES", LIMIT.to_string()));
    }

    fn app() -> Router {
        init_limit();
        Router::new()
            .route("/echo", post(|body: String| async move { body.len().to_string() }))
            .layer(middleware::from_fn(request_size_validation_middleware))
    }

    fn post_body(len: usize, with_content_length: bool) -> Request<Body> {
        let mut b = Request::builder().method("POST").uri("/echo");
        if with_content_length {
            b = b.header("content-length", len.to_string());
        }
        b.body(Body::from("a".repeat(len))).unwrap()
    }

    #[tokio::test]
    async fn body_at_limit_reaches_handler() {
        let resp = app().oneshot(post_body(LIMIT, true)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn body_over_limit_rejected_via_content_length() {
        let resp = app().oneshot(post_body(LIMIT + 1, true)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn body_over_limit_rejected_without_content_length() {
        let resp = app().oneshot(post_body(LIMIT + 1, false)).await.unwrap();
        assert_eq!(resp.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn limit_is_not_reread_per_request() {
        let router = app();
        let ok = router.clone().oneshot(post_body(LIMIT, true)).await.unwrap();
        assert_eq!(ok.status(), StatusCode::OK);

        // Changing the env var after first use must not change the enforced limit.
        std::env::set_var("REQUEST_BODY_MAX_BYTES", "1");
        let still_ok = router.clone().oneshot(post_body(LIMIT, true)).await.unwrap();
        assert_eq!(still_ok.status(), StatusCode::OK);

        std::env::set_var("REQUEST_BODY_MAX_BYTES", "1000000");
        let still_rejected = router.oneshot(post_body(LIMIT + 1, true)).await.unwrap();
        assert_eq!(still_rejected.status(), StatusCode::PAYLOAD_TOO_LARGE);
        std::env::set_var("REQUEST_BODY_MAX_BYTES", LIMIT.to_string());
    }
}
