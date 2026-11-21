#[cfg(test)]
mod api_key_auth_tests {
    use predictiq_api::security::ApiKeyAuth;

    #[test]
    fn test_api_key_auth_verify_valid_key() {
        let auth = ApiKeyAuth::new(vec!["test-key-123".to_string(), "another-key".to_string()]);
        assert!(auth.verify("test-key-123"));
        assert!(auth.verify("another-key"));
    }

    #[test]
    fn test_api_key_auth_verify_invalid_key() {
        let auth = ApiKeyAuth::new(vec!["test-key-123".to_string()]);
        assert!(!auth.verify("wrong-key"));
        assert!(!auth.verify("test-key-12")); // Different length
        assert!(!auth.verify("test-key-1234")); // Different length
        assert!(!auth.verify(""));
    }

    #[test]
    fn test_api_key_auth_verify_empty_keys() {
        let auth = ApiKeyAuth::new(vec![]);
        assert!(!auth.verify("any-key"));
        assert!(!auth.verify(""));
    }

    #[test]
    fn test_api_key_auth_verify_edge_cases() {
        let auth = ApiKeyAuth::new(vec!["".to_string(), "a".to_string()]);
        assert!(auth.verify("")); // Empty string key
        assert!(auth.verify("a")); // Single character key
        assert!(!auth.verify("b")); // Same length but different content
    }

    #[test]
    fn test_api_key_auth_revoke_sets_revoked_at() {
        let mut auth = ApiKeyAuth::new(vec!["test-key-123".to_string()]);
        assert!(auth.revoke("test-key-123"));
        assert!(auth.is_revoked("test-key-123"));
    }

    #[test]
    fn test_api_key_auth_revoke_unknown_key() {
        let mut auth = ApiKeyAuth::new(vec!["test-key-123".to_string()]);
        assert!(!auth.revoke("unknown-key"));
        assert!(!auth.is_revoked("unknown-key"));
        assert!(!auth.is_revoked("test-key-123"));
    }

    #[test]
    fn test_api_key_auth_rejects_revoked_key() {
        let mut auth = ApiKeyAuth::new(vec!["test-key-123".to_string(), "another-key".to_string()]);
        assert!(auth.verify("test-key-123"));

        assert!(auth.revoke("test-key-123"));

        // Revoked key is rejected, other keys still work
        assert!(!auth.verify("test-key-123"));
        assert!(auth.verify("another-key"));
    }

    #[test]
    fn test_api_key_auth_revoke_is_idempotent() {
        let mut auth = ApiKeyAuth::new(vec!["test-key-123".to_string()]);
        assert!(auth.revoke("test-key-123"));
        assert!(auth.revoke("test-key-123"));
        assert!(!auth.verify("test-key-123"));
    }

    #[test]
    fn test_api_key_auth_constant_time_behavior() {
        // Test that verification time doesn't depend on how many keys match partially
        let keys = vec![
            "aaaaaaaaaaaaaaaa".to_string(),
            "baaaaaaaaaaaaaaaa".to_string(),
            "caaaaaaaaaaaaaaaa".to_string(),
            "daaaaaaaaaaaaaaaa".to_string(),
            "target-key-123456".to_string(),
        ];
        let auth = ApiKeyAuth::new(keys);

        // These should all take roughly the same time regardless of where they differ
        assert!(!auth.verify("aaaaaaaaaaaaaaab")); // Differs at last char of first key
        assert!(!auth.verify("baaaaaaaaaaaaaaab")); // Differs at last char of second key
        assert!(!auth.verify("caaaaaaaaaaaaaaab")); // Differs at last char of third key
        assert!(!auth.verify("daaaaaaaaaaaaaaab")); // Differs at last char of fourth key
        assert!(auth.verify("target-key-123456")); // Exact match
        assert!(!auth.verify("target-key-123457")); // Differs at last char of matching key
    }
}
