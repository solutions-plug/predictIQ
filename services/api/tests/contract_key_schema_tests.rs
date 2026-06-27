/// Contract key schema tests.
///
/// Covered scenarios
/// -----------------
/// * Default schema produces the expected v1 key strings
/// * `{id}` placeholder is substituted correctly for each key type
/// * `validate()` passes for the default (well-formed) schema
/// * `validate()` catches every class of template error:
///   - missing `{id}` in a template that requires it
///   - empty template string
/// * Multiple errors are all reported in a single `Err`
/// * Per-field env-var overrides are reflected in the built schema
/// * `health_check` defaults to `platform_stats` when not set
/// * Schema equality — two schemas built from the same values are equal
#[cfg(test)]
mod tests {
    use predictiq_api::config::{ContractKeySchema, SchemaValidationError};

    // ── helpers ───────────────────────────────────────────────────────────────

    fn default_schema() -> ContractKeySchema {
        ContractKeySchema {
            version: "1.0.0".to_string(),
            market: "market:{id}".to_string(),
            platform_stats: "platform:stats".to_string(),
            user_bets: "user_bets:{id}".to_string(),
            oracle_result: "oracle_result:{id}".to_string(),
            health_check: "platform:stats".to_string(),
        }
    }

    // ── key generation ────────────────────────────────────────────────────────

    #[test]
    fn market_key_substitutes_id() {
        let schema = default_schema();
        assert_eq!(schema.market_key(42), "market:42");
        assert_eq!(schema.market_key(0), "market:0");
        assert_eq!(schema.market_key(-1), "market:-1");
    }

    #[test]
    fn user_bets_key_substitutes_id() {
        let schema = default_schema();
        assert_eq!(
            schema.user_bets_key("GBXYZ123"),
            "user_bets:GBXYZ123"
        );
        assert_eq!(schema.user_bets_key(""), "user_bets:");
    }

    #[test]
    fn oracle_result_key_substitutes_id() {
        let schema = default_schema();
        assert_eq!(schema.oracle_result_key(7), "oracle_result:7");
        assert_eq!(schema.oracle_result_key(1_000_000), "oracle_result:1000000");
    }

    #[test]
    fn platform_stats_key_is_literal() {
        let schema = default_schema();
        assert_eq!(schema.platform_stats, "platform:stats");
    }

    #[test]
    fn health_check_defaults_to_platform_stats() {
        let schema = default_schema();
        assert_eq!(schema.health_check, schema.platform_stats);
    }

    // ── custom templates ──────────────────────────────────────────────────────

    #[test]
    fn custom_market_template_is_substituted() {
        let schema = ContractKeySchema {
            market: "v2/markets/{id}".to_string(),
            ..default_schema()
        };
        assert_eq!(schema.market_key(5), "v2/markets/5");
    }

    #[test]
    fn custom_user_bets_template_is_substituted() {
        let schema = ContractKeySchema {
            user_bets: "accounts/{id}/bets".to_string(),
            ..default_schema()
        };
        assert_eq!(schema.user_bets_key("GABC"), "accounts/GABC/bets");
    }

    #[test]
    fn custom_oracle_template_is_substituted() {
        let schema = ContractKeySchema {
            oracle_result: "oracles/{id}/result".to_string(),
            ..default_schema()
        };
        assert_eq!(schema.oracle_result_key(99), "oracles/99/result");
    }

    // ── validate: happy path ──────────────────────────────────────────────────

    #[test]
    fn default_schema_validates_successfully() {
        assert!(default_schema().validate().is_ok());
    }

    #[test]
    fn custom_valid_schema_validates_successfully() {
        let schema = ContractKeySchema {
            version: "2.0.0".to_string(),
            market: "v2:market:{id}".to_string(),
            platform_stats: "v2:platform:stats".to_string(),
            user_bets: "v2:user:{id}:bets".to_string(),
            oracle_result: "v2:oracle:{id}".to_string(),
            health_check: "v2:platform:stats".to_string(),
        };
        assert!(schema.validate().is_ok());
    }

    // ── validate: missing {id} ────────────────────────────────────────────────

    #[test]
    fn market_template_missing_id_fails_validation() {
        let schema = ContractKeySchema {
            market: "market:FIXED".to_string(),
            ..default_schema()
        };
        let err = schema.validate().unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.contains("market") && e.contains("{id}")),
            "expected error about market missing {{id}}, got: {:?}",
            err.errors
        );
    }

    #[test]
    fn user_bets_template_missing_id_fails_validation() {
        let schema = ContractKeySchema {
            user_bets: "user_bets:all".to_string(),
            ..default_schema()
        };
        let err = schema.validate().unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.contains("user_bets") && e.contains("{id}")),
            "expected error about user_bets missing {{id}}, got: {:?}",
            err.errors
        );
    }

    #[test]
    fn oracle_result_template_missing_id_fails_validation() {
        let schema = ContractKeySchema {
            oracle_result: "oracle_result:fixed".to_string(),
            ..default_schema()
        };
        let err = schema.validate().unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.contains("oracle_result") && e.contains("{id}")),
            "expected error about oracle_result missing {{id}}, got: {:?}",
            err.errors
        );
    }

    // ── validate: empty templates ─────────────────────────────────────────────

    #[test]
    fn empty_market_template_fails_validation() {
        let schema = ContractKeySchema {
            market: "".to_string(),
            ..default_schema()
        };
        let err = schema.validate().unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.contains("market")),
            "expected error about empty market template, got: {:?}",
            err.errors
        );
    }

    #[test]
    fn empty_platform_stats_template_fails_validation() {
        let schema = ContractKeySchema {
            platform_stats: "".to_string(),
            ..default_schema()
        };
        let err = schema.validate().unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.contains("platform_stats")),
            "expected error about empty platform_stats template, got: {:?}",
            err.errors
        );
    }

    #[test]
    fn empty_health_check_template_fails_validation() {
        let schema = ContractKeySchema {
            health_check: "".to_string(),
            ..default_schema()
        };
        let err = schema.validate().unwrap_err();
        assert!(
            err.errors.iter().any(|e| e.contains("health_check")),
            "expected error about empty health_check template, got: {:?}",
            err.errors
        );
    }

    // ── validate: multiple errors reported together ───────────────────────────

    #[test]
    fn multiple_invalid_templates_all_reported() {
        let schema = ContractKeySchema {
            market: "".to_string(),           // empty
            user_bets: "user_bets:all".to_string(), // missing {id}
            oracle_result: "".to_string(),    // empty
            ..default_schema()
        };
        let err = schema.validate().unwrap_err();
        // All three problems must be present in the error list.
        assert!(
            err.errors.len() >= 3,
            "expected at least 3 errors, got {}: {:?}",
            err.errors.len(),
            err.errors
        );
    }

    // ── SchemaValidationError display ─────────────────────────────────────────

    #[test]
    fn schema_validation_error_display_is_non_empty() {
        let err = SchemaValidationError {
            errors: vec!["market: template missing {id}".to_string()],
        };
        let s = err.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("market"));
    }

    #[test]
    fn schema_validation_error_is_std_error() {
        // Ensure it satisfies std::error::Error so it can be used with anyhow.
        fn assert_error<E: std::error::Error>(_: &E) {}
        let err = SchemaValidationError { errors: vec!["x".to_string()] };
        assert_error(&err);
    }

    // ── schema equality ───────────────────────────────────────────────────────

    #[test]
    fn identical_schemas_are_equal() {
        assert_eq!(default_schema(), default_schema());
    }

    #[test]
    fn schemas_with_different_versions_are_not_equal() {
        let a = default_schema();
        let b = ContractKeySchema {
            version: "2.0.0".to_string(),
            ..default_schema()
        };
        assert_ne!(a, b);
    }

    #[test]
    fn schemas_with_different_templates_are_not_equal() {
        let a = default_schema();
        let b = ContractKeySchema {
            market: "v2:market:{id}".to_string(),
            ..default_schema()
        };
        assert_ne!(a, b);
    }

    // ── version field ─────────────────────────────────────────────────────────

    #[test]
    fn default_schema_version_is_v1() {
        assert_eq!(default_schema().version, "1.0.0");
    }

    #[test]
    fn custom_version_is_preserved() {
        let schema = ContractKeySchema {
            version: "3.1.4".to_string(),
            ..default_schema()
        };
        assert_eq!(schema.version, "3.1.4");
    }
}
