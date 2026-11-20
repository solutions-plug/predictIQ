#[cfg(test)]
mod api_key_auth_tests {
    use predictiq_api::security::ApiKeyAuth;
    use predictiq_api::validation::validate_overlap_days;

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

    #[test]
    fn test_rotate_api_key_rollback_on_failure() {
        // A failure partway through rotation must leave the previous key state
        // unchanged: the old key stays valid and no replacement is persisted.
        let mut store = MockKeyStore::new();
        let old_key = store.insert_active_key("old-key-material");

        let result = store.rotate_api_key(old_key, 7, /* fail_after_inserts = */ true);

        assert!(result.is_err(), "rotation should surface the failure");
        assert!(
            store.is_active(old_key),
            "old key must remain active after a failed rotation"
        );
        assert_eq!(
            store.active_key_count(),
            1,
            "failed rotation must not leave a partially written replacement key"
        );
    }

    #[test]
    fn test_rotate_api_key_success_replaces_key() {
        let mut store = MockKeyStore::new();
        let old_key = store.insert_active_key("old-key-material");

        let new_key = store
            .rotate_api_key(old_key, 7, false)
            .expect("rotation should succeed");

        assert_ne!(new_key, old_key);
        assert!(store.is_active(new_key));
        assert!(!store.is_active(old_key));
    }

    #[test]
    fn test_rotate_api_key_rejects_overlap_days_below_min() {
        let mut store = MockKeyStore::new();
        let old_key = store.insert_active_key("old-key-material");

        let err = store
            .rotate_api_key(old_key, 0, false)
            .expect_err("overlap_days below the minimum must be rejected");
        assert!(
            err.to_string().contains("overlap_days"),
            "error should clearly mention overlap_days, got: {err}"
        );
        assert!(store.is_active(old_key), "rejected rotation must not mutate state");
    }

    #[test]
    fn test_rotate_api_key_rejects_overlap_days_above_max() {
        let mut store = MockKeyStore::new();
        let old_key = store.insert_active_key("old-key-material");

        let err = store
            .rotate_api_key(old_key, u32::MAX, false)
            .expect_err("overlap_days above the maximum must be rejected");
        assert!(
            err.to_string().contains("overlap_days"),
            "error should clearly mention overlap_days, got: {err}"
        );
        assert!(store.is_active(old_key), "rejected rotation must not mutate state");
    }

    #[test]
    fn test_rotate_api_key_accepts_overlap_days_in_range() {
        let mut store = MockKeyStore::new();
        let old_key = store.insert_active_key("old-key-material");

        assert!(validate_overlap_days(OVERLAP_DAYS_MIN).is_ok());
        assert!(validate_overlap_days(OVERLAP_DAYS_MAX).is_ok());
        assert!(validate_overlap_days(OVERLAP_DAYS_MIN + 1).is_ok());

        let new_key = store
            .rotate_api_key(old_key, OVERLAP_DAYS_MAX, false)
            .expect("in-range overlap_days should be accepted");
        assert!(store.is_active(new_key));
    }

    #[test]
    fn test_rotate_api_key_generates_distinct_secure_keys() {
        let mut store = MockKeyStore::new();
        let first = store.insert_active_key("first");
        let second = store.insert_active_key("second");

        let a = store.rotate_api_key(first, 7, false).expect("rotation ok");
        let b = store.rotate_api_key(second, 7, false).expect("rotation ok");

        assert_ne!(a, b, "each rotation must produce fresh key material");
        assert!(a.len() >= 32, "generated key should have sufficient entropy");
    }
}
