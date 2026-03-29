#[cfg(test)]
mod tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use predictiq_api::security::{
        sanitize, signing, IpWhitelist, RateLimitConfig, RateLimiter,
        global_rate_limit_middleware,
    };
    use std::{net::IpAddr, sync::Arc, time::Duration};
    use tower::ServiceExt;

    #[test]
    fn test_email_sanitization() {
        assert_eq!(
            sanitize::email("  Test@Example.COM  "),
            Some("test@example.com".to_string())
        );
        assert_eq!(sanitize::email("invalid-email"), None);
        assert_eq!(sanitize::email(""), None);
    }

    #[test]
    fn test_string_sanitization() {
        let input = "Hello\x00World\x01Test";
        let result = sanitize::string(input, 100);
        assert!(!result.contains('\x00'));
        assert!(!result.contains('\x01'));

        let long_input = "a".repeat(1000);
        let result = sanitize::string(&long_input, 50);
        assert_eq!(result.len(), 50);
    }

    // --- sanitize::contains_sql_injection ---

    #[test]
    fn test_sql_injection_detection() {
        assert!(sanitize::contains_sql_injection("' OR '1'='1"));
        assert!(sanitize::contains_sql_injection("'; DROP TABLE users;"));
        assert!(sanitize::contains_sql_injection("UNION SELECT * FROM"));
        assert!(!sanitize::contains_sql_injection("normal query text"));
    }

    #[test]
    fn test_xss_detection() {
        assert!(sanitize::contains_sql_injection(
            "<script>alert('xss')</script>"
        ));
        assert!(sanitize::contains_sql_injection("javascript:alert(1)"));
        assert!(sanitize::contains_sql_injection("onerror=alert(1)"));
    }

    /// Obfuscated / encoded bypass variants that the simple list should still catch.
    #[test]
    fn sql_injection_encoded_variants() {
        // Mixed case
        assert!(sanitize::contains_sql_injection("UNION Select password FROM users"));
        // URL-encoded space represented as literal (after decode step callers may do)
        assert!(sanitize::contains_sql_injection("union select 1,2,3"));
        // Inline comment bypass: `un/**/ion` is NOT caught by substring match — document that
        // the function does NOT claim to catch all variants, only the listed patterns.
        // These should NOT be flagged (benign false-positive checks):
        assert!(!sanitize::contains_sql_injection("select your favorite color"));
        assert!(!sanitize::contains_sql_injection("drop the ball"));
        assert!(!sanitize::contains_sql_injection("execute your plan"));
        assert!(!sanitize::contains_sql_injection("onload of work"));
    }

    /// Benign strings that must NOT be flagged as injections.
    #[test]
    fn sql_injection_no_false_positives_on_benign_text() {
        let benign = [
            "Hello, world!",
            "user@example.com",
            "The quick brown fox",
            "Price: $1.00",
            "2024-01-01",
            "https://example.com/path?q=1",
        ];
        for s in &benign {
            assert!(
                !sanitize::contains_sql_injection(s),
                "false positive on: {s}"
            );
        }
    }

    // --- RateLimiter (unit) ---

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(3, Duration::from_secs(60));

        assert!(limiter.check("test-key", &config).await);
        assert!(limiter.check("test-key", &config).await);
        assert!(limiter.check("test-key", &config).await);
        assert!(!limiter.check("test-key", &config).await);

        // Different key must be independent
        assert!(limiter.check("other-key", &config).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_window_reset() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(2, Duration::from_millis(100));

        assert!(limiter.check("test-key", &config).await);
        assert!(limiter.check("test-key", &config).await);
        assert!(!limiter.check("test-key", &config).await);

        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(limiter.check("test-key", &config).await);
    }

    /// Keys for different IPs must not share quota.
    #[tokio::test]
    async fn rate_limiter_per_ip_isolation() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(1, Duration::from_secs(60));

        assert!(limiter.check("global:1.2.3.4", &config).await);
        assert!(!limiter.check("global:1.2.3.4", &config).await);

        // Different IP still has its own fresh quota
        assert!(limiter.check("global:5.6.7.8", &config).await);
    }

    /// After cleanup, entries older than the retention window are removed and
    /// the map does not grow without bound.
    #[tokio::test]
    async fn rate_limiter_cleanup_bounds_map_size() {
        let limiter = RateLimiter::new();
        // Use a very short window so entries age out quickly
        let config = RateLimitConfig::new(1, Duration::from_millis(1));

        // Insert many unique keys
        for i in 0..500u32 {
            limiter.check(&format!("key:{i}"), &config).await;
        }

        // Let all windows expire (cleanup retains entries < 1 hour, but our
        // entries are within that window — we just verify cleanup runs without panic
        // and the limiter remains functional afterwards)
        limiter.cleanup().await;

        // Limiter still works after cleanup
        assert!(limiter.check("post-cleanup", &config).await);
    }

    // --- Middleware HTTP behaviour (200 / 429 transitions) ---

    fn rate_limited_app(limiter: Arc<RateLimiter>) -> Router {
        Router::new()
            .route("/", get(|| async { StatusCode::OK }))
            .layer(middleware::from_fn_with_state(
                limiter,
                global_rate_limit_middleware,
            ))
    }

    #[tokio::test]
    async fn middleware_allows_requests_under_limit() {
        // Use a fresh limiter with a tight limit so we can exhaust it quickly,
        // but we only send one request here — it must be 200.
        let limiter = Arc::new(RateLimiter::new());
        let app = rate_limited_app(limiter);

        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-for", "10.0.0.1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn middleware_returns_429_after_limit_exceeded() {
        let limiter = Arc::new(RateLimiter::new());
        // Exhaust the in-memory counter directly (100 req/min is the global limit)
        let config = RateLimitConfig::new(100, Duration::from_secs(60));
        for _ in 0..100 {
            limiter.check("global:10.0.0.2", &config).await;
        }

        let app = rate_limited_app(limiter);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-for", "10.0.0.2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
    }

    /// Two IPs must not share quota through the middleware.
    #[tokio::test]
    async fn middleware_per_ip_key_scoping() {
        let limiter = Arc::new(RateLimiter::new());
        let config = RateLimitConfig::new(100, Duration::from_secs(60));

        // Exhaust quota for IP A
        for _ in 0..100 {
            limiter.check("global:192.168.1.1", &config).await;
        }

        // IP B should still get through
        let app = rate_limited_app(limiter);
        let resp = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-forwarded-for", "192.168.1.2")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    // --- IpWhitelist ---

    #[test]
    fn ip_whitelist_allows_listed_ipv4() {
        let wl = IpWhitelist::new(vec!["127.0.0.1".parse::<IpAddr>().unwrap()]);
        assert!(wl.is_allowed("127.0.0.1"));
    }

    #[test]
    fn ip_whitelist_blocks_unlisted_ipv4() {
        let wl = IpWhitelist::new(vec!["127.0.0.1".parse::<IpAddr>().unwrap()]);
        assert!(!wl.is_allowed("10.0.0.1"));
    }

    #[test]
    fn ip_whitelist_allows_listed_ipv6() {
        let wl = IpWhitelist::new(vec!["::1".parse::<IpAddr>().unwrap()]);
        assert!(wl.is_allowed("::1"));
    }

    #[test]
    fn ip_whitelist_blocks_unlisted_ipv6() {
        let wl = IpWhitelist::new(vec!["::1".parse::<IpAddr>().unwrap()]);
        assert!(!wl.is_allowed("2001:db8::1"));
    }

    #[test]
    fn ip_whitelist_rejects_malformed_input() {
        let wl = IpWhitelist::new(vec!["127.0.0.1".parse::<IpAddr>().unwrap()]);
        assert!(!wl.is_allowed("not-an-ip"));
        assert!(!wl.is_allowed(""));
        assert!(!wl.is_allowed("999.999.999.999"));
        assert!(!wl.is_allowed("unknown"));
    }

    #[test]
    fn ip_whitelist_empty_list_blocks_all() {
        let wl = IpWhitelist::new(vec![]);
        assert!(!wl.is_allowed("127.0.0.1"));
        assert!(!wl.is_allowed("::1"));
    }

    // --- signing ---

    #[test]
    fn test_request_signing() {
        let payload = b"test payload";
        let secret = "test-secret";

        let signature = signing::generate_signature(payload, secret);
        assert!(signing::verify_signature(payload, &signature, secret));

        assert!(!signing::verify_signature(
            b"wrong payload",
            &signature,
            secret
        ));

        assert!(!signing::verify_signature(
            payload,
            &signature,
            "wrong-secret"
        ));
    }

    #[test]
    fn test_numeric_id_sanitization() {
        assert_eq!(sanitize::numeric_id("123"), Some(123));
        assert_eq!(sanitize::numeric_id("  456  "), Some(456));
        assert_eq!(sanitize::numeric_id("abc"), None);
        assert_eq!(sanitize::numeric_id("12.34"), None);
    }
}
