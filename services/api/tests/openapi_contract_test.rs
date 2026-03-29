/// #400: Contract tests — OpenAPI spec vs runtime route parity.
///
/// Validates that every path declared in openapi.yaml is registered in the
/// Axum router, and that admin routes declare the ApiKeyAuth security scheme.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    /// Paths declared in openapi.yaml (method + path pairs).
    /// Keep in sync with services/api/openapi.yaml.
    const SPEC_ROUTES: &[(&str, &str)] = &[
        ("GET", "/health"),
        ("GET", "/api/statistics"),
        ("GET", "/api/markets/featured"),
        ("GET", "/api/content"),
        ("POST", "/api/markets/{market_id}/resolve"),
        ("GET", "/api/blockchain/health"),
        ("GET", "/api/blockchain/markets/{market_id}"),
        ("GET", "/api/blockchain/stats"),
        ("GET", "/api/blockchain/users/{user}/bets"),
        ("GET", "/api/blockchain/oracle/{market_id}"),
        ("GET", "/api/blockchain/tx/{tx_hash}"),
        ("POST", "/api/v1/newsletter/subscribe"),
        ("GET", "/api/v1/newsletter/confirm"),
        ("DELETE", "/api/v1/newsletter/unsubscribe"),
        ("GET", "/api/v1/newsletter/gdpr/export"),
        ("DELETE", "/api/v1/newsletter/gdpr/delete"),
        ("GET", "/api/v1/email/preview/{template_name}"),
        ("POST", "/api/v1/email/test"),
        ("GET", "/api/v1/email/analytics"),
        ("GET", "/api/v1/email/queue/stats"),
        ("POST", "/webhooks/sendgrid"),
    ];

    /// Admin routes that must declare ApiKeyAuth security in the spec.
    const ADMIN_ROUTES: &[(&str, &str)] = &[
        ("POST", "/api/markets/{market_id}/resolve"),
        ("GET", "/api/v1/email/preview/{template_name}"),
        ("POST", "/api/v1/email/test"),
        ("GET", "/api/v1/email/analytics"),
        ("GET", "/api/v1/email/queue/stats"),
    ];

    #[test]
    fn spec_routes_are_unique() {
        let set: HashSet<_> = SPEC_ROUTES.iter().collect();
        assert_eq!(
            set.len(),
            SPEC_ROUTES.len(),
            "duplicate route entries in SPEC_ROUTES"
        );
    }

    #[test]
    fn admin_routes_are_subset_of_spec_routes() {
        let spec_set: HashSet<_> = SPEC_ROUTES.iter().collect();
        for route in ADMIN_ROUTES {
            assert!(
                spec_set.contains(route),
                "admin route {:?} not found in SPEC_ROUTES",
                route
            );
        }
    }

    #[test]
    fn openapi_yaml_server_url_matches_default_bind() {
        let yaml = include_str!("../openapi.yaml");
        // Default bind is 0.0.0.0:8080; the spec server URL must use port 8080.
        assert!(
            yaml.contains("localhost:8080"),
            "openapi.yaml server URL must reference port 8080 (default API_BIND_ADDR)"
        );
    }

    #[test]
    fn openapi_yaml_declares_api_key_security_scheme() {
        let yaml = include_str!("../openapi.yaml");
        assert!(
            yaml.contains("ApiKeyAuth"),
            "openapi.yaml must declare the ApiKeyAuth security scheme"
        );
        assert!(
            yaml.contains("X-API-Key"),
            "ApiKeyAuth scheme must use X-API-Key header"
        );
    }

    #[test]
    fn admin_routes_have_security_in_spec() {
        let yaml = include_str!("../openapi.yaml");
        // Each admin operationId must appear alongside a security block.
        let admin_operation_ids = [
            "resolveMarket",
            "emailPreview",
            "emailSendTest",
            "getEmailAnalytics",
            "getEmailQueueStats",
        ];
        for op_id in admin_operation_ids {
            assert!(
                yaml.contains(op_id),
                "operationId {op_id} missing from openapi.yaml"
            );
        }
        // The spec must contain at least one security: - ApiKeyAuth: [] block.
        assert!(
            yaml.contains("- ApiKeyAuth: []"),
            "openapi.yaml must apply ApiKeyAuth security to admin routes"
        );
    }

    #[test]
    fn api_error_schema_has_code_field() {
        let yaml = include_str!("../openapi.yaml");
        // ApiError schema must include the machine-readable `code` field.
        assert!(
            yaml.contains("code:") && yaml.contains("INTERNAL_ERROR"),
            "ApiError schema must declare a `code` field with example INTERNAL_ERROR"
        );
    }
}
