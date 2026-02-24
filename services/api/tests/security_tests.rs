#[cfg(test)]
mod tests {
    use predictiq_api::security::{sanitize, signing, RateLimitConfig, RateLimiter};
    use std::time::Duration;

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
        assert!(sanitize::contains_sql_injection("<script>alert('xss')</script>"));
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

        let signature = signing::generate_signature(payload, secret);
        assert!(signing::verify_signature(payload, &signature, secret));

        // Wrong payload should fail
        assert!(!signing::verify_signature(b"wrong payload", &signature, secret));

        // Wrong secret should fail
        assert!(!signing::verify_signature(payload, &signature, "wrong-secret"));
    }

    #[test]
    fn test_numeric_id_sanitization() {
        assert_eq!(sanitize::numeric_id("123"), Some(123));
        assert_eq!(sanitize::numeric_id("  456  "), Some(456));
        assert_eq!(sanitize::numeric_id("abc"), None);
        assert_eq!(sanitize::numeric_id("12.34"), None);
    }
}
