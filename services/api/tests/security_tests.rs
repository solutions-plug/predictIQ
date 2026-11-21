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
        ip_whitelist_middleware, sanitize, signing, ApiKeyAuth, IpWhitelist, MetricsAuthConfig, RateLimitConfig, RateLimiter, TrustProxy,
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
    fn trust_proxy_defaults_to_false_in_from_env() {
        // Preserve existing env and restore after the test to avoid flakiness
        let orig_trust = std::env::var("TRUST_PROXY").ok();
        let orig_trusted = std::env::var("TRUSTED_PROXY_CIDRS").ok();
        // Ensure variables are not set to simulate fresh deployment
        std::env::remove_var("TRUST_PROXY");
        std::env::remove_var("TRUSTED_PROXY_CIDRS");

        let cfg = predictiq_api::config::Config::from_env();
        assert_eq!(cfg.trust_proxy, false, "TRUST_PROXY must default to false");
        assert!(cfg.trusted_proxy_cidrs.is_empty(), "TRUSTED_PROXY_CIDRS must default to empty");

        // Restore original environment
        if let Some(v) = orig_trust { std::env::set_var("TRUST_PROXY", v); }
        if let Some(v) = orig_trusted { std::env::set_var("TRUSTED_PROXY_CIDRS", v); }
    }

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

    // ── metrics_auth_middleware — HTTP-level tests ────────────────────────────

    fn metrics_app(public: bool, allowlist: Vec<&str>, keys: Vec<&str>) -> Router {
        use predictiq_api::security::metrics_auth_middleware;
        let config = Arc::new(MetricsAuthConfig::new(
            public,
            allowlist.iter().filter_map(|s| s.parse().ok()).collect(),
            Arc::new(ApiKeyAuth::new(keys.iter().map(|s| s.to_string()).collect())),
        ));
        Router::new()
            .route("/metrics", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(config, metrics_auth_middleware))
    }

    async fn call_metrics(app: Router, api_key: Option<&str>) -> StatusCode {
        let mut builder = Request::builder().uri("/metrics");
        if let Some(k) = api_key {
            builder = builder.header("x-api-key", k);
        }
        app.oneshot(builder.body(Body::empty()).unwrap())
            .await
            .unwrap()
            .status()
    }

    #[tokio::test]
    async fn metrics_public_mode_no_key_required() {
        assert_eq!(call_metrics(metrics_app(true, vec![], vec!["s"]), None).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn metrics_no_key_returns_unauthorized() {
        assert_eq!(call_metrics(metrics_app(false, vec![], vec!["s"]), None).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn metrics_valid_key_returns_ok() {
        assert_eq!(call_metrics(metrics_app(false, vec![], vec!["s"]), Some("s")).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn metrics_wrong_key_returns_unauthorized() {
        assert_eq!(call_metrics(metrics_app(false, vec![], vec!["s"]), Some("bad")).await, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn metrics_ip_not_in_allowlist_returns_forbidden() {
        // allowlist = 10.0.0.1 only; request has no ConnectInfo → "unknown" → rejected
        assert_eq!(
            call_metrics(metrics_app(false, vec!["10.0.0.1"], vec!["s"]), Some("s")).await,
            StatusCode::FORBIDDEN,
        );
    }

    // -------------------------------------------------------------------------
    // #1861: SendGrid webhook ECDSA signature verification
    // -------------------------------------------------------------------------

    use predictiq_api::newsletter::{
        verify_sendgrid_signature, SendGridSignatureError, SENDGRID_SIGNATURE_HEADER,
        SENDGRID_TIMESTAMP_HEADER,
    };

    /// A throwaway P-256 key pair used only for tests. The private key is
    /// generated at test time; the public key is exported in SPKI PEM form so
    /// it can be fed to the verifier exactly like a configured SendGrid key.
    struct TestKeyPair {
        signing_key: p256::ecdsa::SigningKey,
        public_key_pem: String,
    }

    impl TestKeyPair {
        fn generate() -> Self {
            use p256::pkcs8::{EncodePublicKey, LineEnding};

            let signing_key = p256::ecdsa::SigningKey::random(&mut rand_core::OsRng);
            let public_key_pem = signing_key
                .verifying_key()
                .to_public_key_pem(LineEnding::LF)
                .expect("failed to encode test public key as PEM");
            Self {
                signing_key,
                public_key_pem,
            }
        }

        /// Sign `timestamp || payload` the way SendGrid does and return the
        /// base64 DER-encoded signature.
        fn sign(&self, timestamp: &str, payload: &[u8]) -> String {
            use p256::ecdsa::{signature::Signer, Signature};
            use base64::Engine;

            let mut message = Vec::with_capacity(timestamp.len() + payload.len());
            message.extend_from_slice(timestamp.as_bytes());
            message.extend_from_slice(payload);

            let signature: Signature = self.signing_key.sign(&message);
            base64::engine::general_purpose::STANDARD.encode(signature.to_der().as_bytes())
        }
    }

    const SENDGRID_TEST_TIMESTAMP: &str = "1700000000";
    const SENDGRID_TEST_PAYLOAD: &[u8] = br#"[{"event":"open","email":"user@example.com"}]"#;

    #[test]
    fn sendgrid_valid_signature_is_accepted() {
        let keys = TestKeyPair::generate();
        let signature = keys.sign(SENDGRID_TEST_TIMESTAMP, SENDGRID_TEST_PAYLOAD);

        let result = verify_sendgrid_signature(
            &keys.public_key_pem,
            SENDGRID_TEST_TIMESTAMP,
            &signature,
            SENDGRID_TEST_PAYLOAD,
        );

        assert!(
            result.is_ok(),
            "correctly signed SendGrid payload must be accepted, got {result:?}"
        );
    }

    #[test]
    fn sendgrid_tampered_payload_is_rejected() {
        let keys = TestKeyPair::generate();
        let signature = keys.sign(SENDGRID_TEST_TIMESTAMP, SENDGRID_TEST_PAYLOAD);

        let tampered = br#"[{"event":"open","email":"attacker@example.com"}]"#;
        let result = verify_sendgrid_signature(
            &keys.public_key_pem,
            SENDGRID_TEST_TIMESTAMP,
            &signature,
            tampered,
        );

        assert_eq!(
            result,
            Err(SendGridSignatureError::InvalidSignature),
            "tampered payload must be rejected"
        );
    }

    #[test]
    fn sendgrid_tampered_timestamp_is_rejected() {
        let keys = TestKeyPair::generate();
        let signature = keys.sign(SENDGRID_TEST_TIMESTAMP, SENDGRID_TEST_PAYLOAD);

        let result = verify_sendgrid_signature(
            &keys.public_key_pem,
            "1700000001",
            &signature,
            SENDGRID_TEST_PAYLOAD,
        );

        assert_eq!(result, Err(SendGridSignatureError::InvalidSignature));
    }

    #[test]
    fn sendgrid_missing_signature_is_rejected() {
        let keys = TestKeyPair::generate();

        let result = verify_sendgrid_signature(
            &keys.public_key_pem,
            SENDGRID_TEST_TIMESTAMP,
            "",
            SENDGRID_TEST_PAYLOAD,
        );

        assert_eq!(result, Err(SendGridSignatureError::MissingSignature));
    }

    #[test]
    fn sendgrid_missing_timestamp_is_rejected() {
        let keys = TestKeyPair::generate();
        let signature = keys.sign(SENDGRID_TEST_TIMESTAMP, SENDGRID_TEST_PAYLOAD);

        let result = verify_sendgrid_signature(&keys.public_key_pem, "", &signature, SENDGRID_TEST_PAYLOAD);

        assert_eq!(result, Err(SendGridSignatureError::MissingTimestamp));
    }

    #[test]
    fn sendgrid_malformed_signature_is_rejected() {
        let keys = TestKeyPair::generate();

        let result = verify_sendgrid_signature(
            &keys.public_key_pem,
            SENDGRID_TEST_TIMESTAMP,
            "not-valid-base64!!!",
            SENDGRID_TEST_PAYLOAD,
        );

        assert_eq!(result, Err(SendGridSignatureError::MalformedSignature));
    }

    #[test]
    fn sendgrid_signature_from_wrong_key_is_rejected() {
        let signer = TestKeyPair::generate();
        let other = TestKeyPair::generate();
        let signature = signer.sign(SENDGRID_TEST_TIMESTAMP, SENDGRID_TEST_PAYLOAD);

        let result = verify_sendgrid_signature(
            &other.public_key_pem,
            SENDGRID_TEST_TIMESTAMP,
            &signature,
            SENDGRID_TEST_PAYLOAD,
        );

        assert_eq!(result, Err(SendGridSignatureError::InvalidSignature));
    }

    #[test]
    fn sendgrid_invalid_public_key_is_rejected() {
        let keys = TestKeyPair::generate();
        let signature = keys.sign(SENDGRID_TEST_TIMESTAMP, SENDGRID_TEST_PAYLOAD);

        let result = verify_sendgrid_signature(
            "not a pem key",
            SENDGRID_TEST_TIMESTAMP,
            &signature,
            SENDGRID_TEST_PAYLOAD,
        );

        assert_eq!(result, Err(SendGridSignatureError::InvalidPublicKey));
    }

    #[test]
    fn sendgrid_signature_error_maps_to_4xx_status() {
        // Every rejection reason must surface as a client error (4xx), never a
        // 5xx or a 2xx, so forged events are never accepted.
        let errors = [
            SendGridSignatureError::MissingSignature,
            SendGridSignatureError::MissingTimestamp,
            SendGridSignatureError::MalformedSignature,
            SendGridSignatureError::InvalidSignature,
            SendGridSignatureError::InvalidPublicKey,
        ];
        for err in errors {
            let status = err.status_code();
            assert!(
                status.is_client_error(),
                "{err:?} must map to a 4xx status, got {status}"
            );
        }
    }

    #[tokio::test]
    async fn sendgrid_webhook_accepts_valid_signature() {
        use axum::routing::post;

        let keys = TestKeyPair::generate();
        let signature = keys.sign(SENDGRID_TEST_TIMESTAMP, SENDGRID_TEST_PAYLOAD);
        let public_key = Arc::new(keys.public_key_pem);

        let app = Router::new().route(
            "/webhooks/sendgrid",
            post(
                move |headers: axum::http::HeaderMap, body: axum::body::Bytes| {
                    let public_key = Arc::clone(&public_key);
                    async move {
                        let signature = headers
                            .get(SENDGRID_SIGNATURE_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default();
                        let timestamp = headers
                            .get(SENDGRID_TIMESTAMP_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default();
                        match verify_sendgrid_signature(&public_key, timestamp, signature, &body) {
                            Ok(()) => StatusCode::OK,
                            Err(err) => err.status_code(),
                        }
                    }
                },
            ),
        );

        let request = Request::builder()
            .method("POST")
            .uri("/webhooks/sendgrid")
            .header(SENDGRID_SIGNATURE_HEADER, signature)
            .header(SENDGRID_TIMESTAMP_HEADER, SENDGRID_TEST_TIMESTAMP)
            .body(Body::from(SENDGRID_TEST_PAYLOAD.to_vec()))
            .unwrap();

        let status = app.oneshot(request).await.unwrap().status();
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn sendgrid_webhook_rejects_tampered_payload() {
        use axum::routing::post;

        let keys = TestKeyPair::generate();
        let signature = keys.sign(SENDGRID_TEST_TIMESTAMP, SENDGRID_TEST_PAYLOAD);
        let public_key = Arc::new(keys.public_key_pem);

        let app = Router::new().route(
            "/webhooks/sendgrid",
            post(
                move |headers: axum::http::HeaderMap, body: axum::body::Bytes| {
                    let public_key = Arc::clone(&public_key);
                    async move {
                        let signature = headers
                            .get(SENDGRID_SIGNATURE_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default();
                        let timestamp = headers
                            .get(SENDGRID_TIMESTAMP_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default();
                        match verify_sendgrid_signature(&public_key, timestamp, signature, &body) {
                            Ok(()) => StatusCode::OK,
                            Err(err) => err.status_code(),
                        }
                    }
                },
            ),
        );

        let tampered = br#"[{"event":"open","email":"attacker@example.com"}]"#;
        let request = Request::builder()
            .method("POST")
            .uri("/webhooks/sendgrid")
            .header(SENDGRID_SIGNATURE_HEADER, signature)
            .header(SENDGRID_TIMESTAMP_HEADER, SENDGRID_TEST_TIMESTAMP)
            .body(Body::from(tampered.to_vec()))
            .unwrap();

        let status = app.oneshot(request).await.unwrap().status();
        assert!(
            status.is_client_error(),
            "tampered payload must be rejected with 4xx, got {status}"
        );
    }

    #[tokio::test]
    async fn sendgrid_webhook_rejects_unsigned_payload() {
        use axum::routing::post;

        let keys = TestKeyPair::generate();
        let public_key = Arc::new(keys.public_key_pem);

        let app = Router::new().route(
            "/webhooks/sendgrid",
            post(
                move |headers: axum::http::HeaderMap, body: axum::body::Bytes| {
                    let public_key = Arc::clone(&public_key);
                    async move {
                        let signature = headers
                            .get(SENDGRID_SIGNATURE_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default();
                        let timestamp = headers
                            .get(SENDGRID_TIMESTAMP_HEADER)
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or_default();
                        match verify_sendgrid_signature(&public_key, timestamp, signature, &body) {
                            Ok(()) => StatusCode::OK,
                            Err(err) => err.status_code(),
                        }
                    }
                },
            ),
        );

        let request = Request::builder()
            .method("POST")
            .uri("/webhooks/sendgrid")
            .body(Body::from(SENDGRID_TEST_PAYLOAD.to_vec()))
            .unwrap();

        let status = app.oneshot(request).await.unwrap().status();
        assert!(
            status.is_client_error(),
            "unsigned payload must be rejected with 4xx, got {status}"
        );
    }
}
