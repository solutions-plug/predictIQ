#[cfg(test)]
mod tests {
    use std::{net::IpAddr, sync::Arc, time::Duration};

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use predictiq_api::security::{
        ip_whitelist_middleware, sanitize, signing, IpWhitelist, RateLimitConfig, RateLimiter, TrustProxy,
    };
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

    #[tokio::test]
    async fn test_rate_limiter() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(3, Duration::from_secs(60));

        // First 3 requests should succeed
        assert!(limiter.check("test-key", &config).await);
        assert!(limiter.check("test-key", &config).await);
        assert!(limiter.check("test-key", &config).await);

        // 4th request should fail
        assert!(!limiter.check("test-key", &config).await);

        // Different key should succeed
        assert!(limiter.check("other-key", &config).await);
    }

    #[tokio::test]
    async fn test_rate_limiter_window_reset() {
        let limiter = RateLimiter::new();
        let config = RateLimitConfig::new(2, Duration::from_millis(100));

        // Use up the limit
        assert!(limiter.check("test-key", &config).await);
        assert!(limiter.check("test-key", &config).await);
        assert!(!limiter.check("test-key", &config).await);

        // Wait for window to reset
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should work again
        assert!(limiter.check("test-key", &config).await);
    }

    #[test]
    fn test_request_signing() {
        let payload = b"test payload";
        let secret = "test-secret";

        let signature = signing::generate_signature(payload, secret).unwrap();
        assert!(signing::verify_signature(payload, &signature, secret));

        // Wrong payload should fail
        assert!(!signing::verify_signature(
            b"wrong payload",
            &signature,
            secret
        ));

        // Wrong secret should fail
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

    // -------------------------------------------------------------------------
    // #281: trust-boundary tests for spoofed forwarding headers
    // -------------------------------------------------------------------------

    #[test]
    fn trust_proxy_disabled_xff_is_ignored() {
        use axum::extract::ConnectInfo;
        use axum::http::HeaderMap;
        use predictiq_api::security::extract_client_ip;
        use std::net::SocketAddr;

        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9".parse().unwrap());
        let ci = ConnectInfo("1.2.3.4:80".parse::<SocketAddr>().unwrap());

        assert_eq!(
            extract_client_ip(&headers, Some(&ci), false),
            "1.2.3.4",
            "X-Forwarded-For must be ignored when trust_proxy is false"
        );
    }

    #[test]
    fn trust_proxy_disabled_x_real_ip_is_ignored() {
        use axum::extract::ConnectInfo;
        use axum::http::HeaderMap;
        use predictiq_api::security::extract_client_ip;
        use std::net::SocketAddr;

        let mut headers = HeaderMap::new();
        headers.insert("x-real-ip", "9.9.9.9".parse().unwrap());
        let ci = ConnectInfo("1.2.3.4:80".parse::<SocketAddr>().unwrap());

        assert_eq!(
            extract_client_ip(&headers, Some(&ci), false),
            "1.2.3.4",
            "X-Real-IP must be ignored when trust_proxy is false"
        );
    }

    #[test]
    fn trust_proxy_enabled_xff_is_used() {
        use axum::extract::ConnectInfo;
        use axum::http::HeaderMap;
        use predictiq_api::security::extract_client_ip;
        use std::net::SocketAddr;

        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "5.6.7.8".parse().unwrap());
        let ci = ConnectInfo("1.2.3.4:80".parse::<SocketAddr>().unwrap());

        assert_eq!(
            extract_client_ip(&headers, Some(&ci), true),
            "5.6.7.8",
            "X-Forwarded-For must be used when trust_proxy is true"
        );
    }

    #[test]
    fn trust_proxy_disabled_no_connect_info_returns_unknown() {
        use axum::http::HeaderMap;
        use predictiq_api::security::extract_client_ip;

        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "9.9.9.9".parse().unwrap());

        assert_eq!(
            extract_client_ip(&headers, None, false),
            "unknown",
            "Must return 'unknown' when trust_proxy is false and no socket info"
        );
    }

    // -------------------------------------------------------------------------
    // #290: generate_signature — fallible API + panic-safety tests
    // -------------------------------------------------------------------------

    #[test]
    fn generate_signature_returns_ok_for_valid_inputs() {
        let sig = signing::generate_signature(b"hello", "secret");
        assert!(sig.is_ok(), "expected Ok for valid payload and secret");
    }

    #[test]
    fn generate_signature_ok_is_verifiable() {
        let payload = b"data";
        let secret = "key";
        let sig = signing::generate_signature(payload, secret).unwrap();
        assert!(signing::verify_signature(payload, &sig, secret));
    }

    #[test]
    fn generate_signature_empty_secret_returns_err() {
        // HMAC rejects a zero-length key; previously this would have panicked
        // via .expect(). Now it must surface as Err(SigningError::InvalidKey).
        let result = signing::generate_signature(b"payload", "");
        assert_eq!(result, Err(signing::SigningError::InvalidKey));
    }

    #[test]
    fn generate_signature_error_is_display_safe() {
        // Ensure the error can be formatted without panicking (used in logs/responses).
        let err = signing::SigningError::InvalidKey;
        assert!(!err.to_string().is_empty());
    }

    // -------------------------------------------------------------------------
    // IpWhitelist — unit tests
    // -------------------------------------------------------------------------

    #[test]
    fn ip_whitelist_allows_ipv4() {
        let wl = IpWhitelist::new(vec!["192.168.1.1".parse().unwrap()]);
        assert!(wl.is_allowed("192.168.1.1"));
        assert!(!wl.is_allowed("192.168.1.2"));
    }

    #[test]
    fn ip_whitelist_allows_ipv6() {
        let wl = IpWhitelist::new(vec!["::1".parse::<IpAddr>().unwrap()]);
        assert!(wl.is_allowed("::1"));
        assert!(!wl.is_allowed("::2"));
    }

    #[test]
    fn ip_whitelist_rejects_malformed_input() {
        let wl = IpWhitelist::new(vec!["10.0.0.1".parse().unwrap()]);
        assert!(!wl.is_allowed("not-an-ip"));
        assert!(!wl.is_allowed(""));
        assert!(!wl.is_allowed("999.999.999.999"));
        assert!(!wl.is_allowed("unknown"));
    }

    #[test]
    fn ip_whitelist_empty_list_denies_all() {
        let wl = IpWhitelist::new(vec![]);
        assert!(!wl.is_allowed("127.0.0.1"));
        assert!(!wl.is_allowed("::1"));
    }

    // ── IpWhitelist middleware — HTTP-level tests ────────────────────────────

    fn whitelist_app(wl: Arc<IpWhitelist>) -> Router {
        Router::new()
            .route("/admin", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(
                (wl, TrustProxy(true)),
                ip_whitelist_middleware,
            ))
    }

    fn req_with_ip(ip: &str) -> Request<Body> {
        Request::builder()
            .uri("/admin")
            .header("x-forwarded-for", ip)
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn middleware_allows_whitelisted_ipv4() {
        let wl = Arc::new(IpWhitelist::new(vec!["10.0.0.1".parse().unwrap()]));
        let status = whitelist_app(wl).oneshot(req_with_ip("10.0.0.1")).await.unwrap().status();
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn middleware_blocks_non_whitelisted_ipv4() {
        let wl = Arc::new(IpWhitelist::new(vec!["10.0.0.1".parse().unwrap()]));
        let status = whitelist_app(wl).oneshot(req_with_ip("10.0.0.2")).await.unwrap().status();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn middleware_allows_whitelisted_ipv6() {
        let wl = Arc::new(IpWhitelist::new(vec!["2001:db8::1".parse::<IpAddr>().unwrap()]));
        let status = whitelist_app(wl).oneshot(req_with_ip("2001:db8::1")).await.unwrap().status();
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn middleware_blocks_malformed_ip_header() {
        let wl = Arc::new(IpWhitelist::new(vec!["10.0.0.1".parse().unwrap()]));
        let status = whitelist_app(wl).oneshot(req_with_ip("not-an-ip")).await.unwrap().status();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn middleware_blocks_unknown_fallback() {
        // No x-forwarded-for and no ConnectInfo → extract_client_ip returns "unknown"
        let wl = Arc::new(IpWhitelist::new(vec!["10.0.0.1".parse().unwrap()]));
        let req = Request::builder().uri("/admin").body(Body::empty()).unwrap();
        let status = whitelist_app(wl).oneshot(req).await.unwrap().status();
        assert_eq!(status, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn middleware_allows_when_whitelist_empty() {
        let wl = Arc::new(IpWhitelist::new(vec![]));
        let req = Request::builder().uri("/admin").body(Body::empty()).unwrap();
        let status = whitelist_app(wl).oneshot(req).await.unwrap().status();
        assert_eq!(status, StatusCode::OK);
    }

    // -------------------------------------------------------------------------
    // sanitize::contains_sql_injection — expanded corpus
    // -------------------------------------------------------------------------

    #[test]
    fn sql_injection_detects_known_patterns() {
        let payloads = [
            "' OR '1'='1",
            "' or 1=1 --",
            "'; DROP TABLE users;",
            "'; DELETE FROM accounts",
            "UNION SELECT username, password FROM users",
            "exec(xp_cmdshell('dir'))",
            "execute(something)",
            "<script>alert(1)</script>",
            "javascript:void(0)",
            "onerror=alert(document.cookie)",
            "onload=fetch('https://evil.com')",
        ];
        for p in &payloads {
            assert!(
                sanitize::contains_sql_injection(p),
                "expected detection for: {p}"
            );
        }
    }

    #[test]
    fn sql_injection_passes_benign_inputs() {
        let benign = [
            "hello world",
            "user@example.com",
            "SELECT your best option",   // "select" alone, no "union select"
            "drop the ball",             // "drop" alone, no "drop table"
            "execute your plan",         // "execute(" not present
            "script writing tips",       // no "<script" tag
            "100% organic",
            "it's a great day",
            "O'Brien",                   // apostrophe but no injection pattern
        ];
        for b in &benign {
            assert!(
                !sanitize::contains_sql_injection(b),
                "false positive for: {b}"
            );
        }
    }

    #[test]
    fn sql_injection_detects_mixed_case_variants() {
        // The function lowercases before matching, so these must all be caught.
        assert!(sanitize::contains_sql_injection("EXEC(something)"));
        assert!(sanitize::contains_sql_injection("Union Select id FROM t"));
        assert!(sanitize::contains_sql_injection("JAVASCRIPT:alert(1)"));
        assert!(sanitize::contains_sql_injection("ONERROR=x"));
    }
}
