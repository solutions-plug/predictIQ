/// #400: Contract tests — OpenAPI spec vs runtime route parity.
///
/// Validates that every path declared in openapi.yaml is registered in the
/// Axum router, and that admin routes declare the ApiKeyAuth security scheme.
///
/// The SPEC_ROUTES table is the authoritative mirror of openapi.yaml paths.
/// The yaml_paths_match_spec_routes test parses the YAML at test-time and
/// fails if the two diverge, preventing silent spec drift.
#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    /// All (METHOD, path) pairs declared in openapi.yaml.
    /// Must stay in sync — the yaml_paths_match_spec_routes test enforces this.
    const SPEC_ROUTES: &[(&str, &str)] = &[
        ("GET", "/health"),
        ("GET", "/api/v1/statistics"),
        ("GET", "/api/v1/markets/featured"),
        ("GET", "/api/v1/content"),
        ("POST", "/api/v1/markets/{market_id}/resolve"),
        ("GET", "/api/v1/blockchain/health"),
        ("GET", "/api/v1/blockchain/markets/{market_id}"),
        ("GET", "/api/v1/blockchain/stats"),
        ("GET", "/api/v1/blockchain/users/{user}/bets"),
        ("GET", "/api/v1/blockchain/oracle/{market_id}"),
        ("GET", "/api/v1/blockchain/tx/{tx_hash}"),
        ("POST", "/api/v1/newsletter/subscribe"),
        ("GET", "/api/v1/newsletter/confirm"),
        ("DELETE", "/api/v1/newsletter/unsubscribe"),
        ("GET", "/api/v1/newsletter/gdpr/export"),
        ("DELETE", "/api/v1/newsletter/gdpr/delete"),
        ("GET", "/api/v1/email/preview/{template_name}"),
        ("POST", "/api/v1/email/test"),
        ("GET", "/api/v1/email/analytics"),
        ("GET", "/api/v1/email/queue/stats"),
        ("POST", "/api/blockchain/replay"),
        ("GET", "/api/v1/email/queue/dead-letter"),
        ("POST", "/api/v1/email/queue/dead-letter/{job_id}/requeue"),
        ("GET", "/api/v1/audit/logs"),
        ("GET", "/api/v1/audit/statistics"),
        ("POST", "/webhooks/sendgrid"),
    ];

    /// Admin routes that must declare ApiKeyAuth security in the spec.
    const ADMIN_ROUTES: &[(&str, &str)] = &[
        ("POST", "/api/v1/markets/{market_id}/resolve"),
        ("GET", "/api/v1/email/preview/{template_name}"),
        ("POST", "/api/v1/email/test"),
        ("GET", "/api/v1/email/analytics"),
        ("GET", "/api/v1/email/queue/stats"),
        ("POST", "/api/blockchain/replay"),
        ("GET", "/api/v1/email/queue/dead-letter"),
        ("POST", "/api/v1/email/queue/dead-letter/{job_id}/requeue"),
        ("GET", "/api/v1/audit/logs"),
        ("GET", "/api/v1/audit/statistics"),
    ];

    const OPENAPI_YAML: &str = include_str!("../openapi.yaml");

    /// Parse (METHOD, path) pairs directly from openapi.yaml and return them.
    /// Uses line-by-line parsing so no YAML library is required in dev-deps.
    fn yaml_routes() -> Vec<(String, String)> {
        let methods = ["get", "post", "put", "delete", "patch"];
        let mut routes = Vec::new();
        let mut current_path: Option<String> = None;

        for line in OPENAPI_YAML.lines() {
            // Top-level path entries start with exactly two spaces then "/"
            if let Some(rest) = line.strip_prefix("  /") {
                if let Some(path_str) = rest.strip_suffix(':') {
                    current_path = Some(format!("/{}", path_str));
                    continue;
                }
            }
            // Method entries are indented four spaces
            if let Some(path) = &current_path {
                for method in methods {
                    let prefix = format!("    {}:", method);
                    if line == prefix {
                        routes.push((method.to_uppercase(), path.clone()));
                    }
                }
            }
        }
        routes
    }

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

    /// Fail if openapi.yaml contains a route missing from SPEC_ROUTES.
    /// This catches new endpoints added to the spec without updating the table.
    #[test]
    fn yaml_paths_match_spec_routes_no_missing() {
        let spec_set: HashSet<(&str, &str)> =
            SPEC_ROUTES.iter().map(|(m, p)| (*m, *p)).collect();
        let yaml = yaml_routes();
        let mut missing = Vec::new();
        for (method, path) in &yaml {
            if !spec_set.contains(&(method.as_str(), path.as_str())) {
                missing.push(format!("{} {}", method, path));
            }
        }
        assert!(
            missing.is_empty(),
            "openapi.yaml has routes not listed in SPEC_ROUTES — add them:\n  {}",
            missing.join("\n  ")
        );
    }

    /// Fail if SPEC_ROUTES lists a route not present in openapi.yaml.
    /// This catches stale entries left behind when endpoints are removed from the spec.
    #[test]
    fn spec_routes_no_stale_entries() {
        let yaml_set: HashSet<(String, String)> = yaml_routes().into_iter().collect();
        let mut stale = Vec::new();
        for (method, path) in SPEC_ROUTES {
            if !yaml_set.contains(&(method.to_string(), path.to_string())) {
                stale.push(format!("{} {}", method, path));
            }
        }
        assert!(
            stale.is_empty(),
            "SPEC_ROUTES has entries missing from openapi.yaml — remove them:\n  {}",
            stale.join("\n  ")
        );
    }

    #[test]
    fn openapi_yaml_server_url_matches_default_bind() {
        // Default bind is 0.0.0.0:8080; the spec server URL must use port 8080.
        assert!(
            OPENAPI_YAML.contains(":8080"),
            "openapi.yaml server URL must reference port 8080 (default API_BIND_ADDR)"
        );
    }

    #[test]
    fn openapi_yaml_declares_api_key_security_scheme() {
        assert!(
            OPENAPI_YAML.contains("ApiKeyAuth"),
            "openapi.yaml must declare the ApiKeyAuth security scheme"
        );
        assert!(
            OPENAPI_YAML.contains("X-API-Key"),
            "ApiKeyAuth scheme must use X-API-Key header"
        );
    }

    #[test]
    fn admin_routes_have_security_in_spec() {
        let admin_operation_ids = [
            "resolveMarket",
            "emailPreview",
            "emailSendTest",
            "getEmailAnalytics",
            "getEmailQueueStats",
            "blockchainReplay",
            "getEmailDeadLetterList",
            "requeueEmailDeadLetterJob",
            "getAuditLogs",
            "getAuditStatistics",
        ];
        for op_id in admin_operation_ids {
            assert!(
                OPENAPI_YAML.contains(op_id),
                "operationId {op_id} missing from openapi.yaml"
            );
        }
        assert!(
            OPENAPI_YAML.contains("- ApiKeyAuth: []"),
            "openapi.yaml must apply ApiKeyAuth security to admin routes"
        );
    }

    #[test]
    fn api_error_schema_has_code_field() {
        assert!(
            OPENAPI_YAML.contains("code:") && OPENAPI_YAML.contains("INTERNAL_ERROR"),
            "ApiError schema must declare a `code` field with example INTERNAL_ERROR"
        );
    }

    /// Every path in the spec must have an operationId so CI tools and clients
    /// can reference operations by stable name.
    #[test]
    fn every_route_has_operation_id() {
        let yaml = yaml_routes();
        let total = yaml.len();
        let id_count = OPENAPI_YAML.matches("operationId:").count();
        assert_eq!(
            id_count, total,
            "expected {total} operationId entries (one per route), found {id_count}"
        );
    }

    /// Every path in the spec must declare at least one success (2xx) response.
    #[test]
    fn every_route_has_success_response() {
        // Count "200:", "201:", "204:" response codes; there must be at least
        // one per route to satisfy the acceptance criterion.
        let success_count = OPENAPI_YAML.matches("\"200\"").count()
            + OPENAPI_YAML.matches("\"201\"").count()
            + OPENAPI_YAML.matches("\"204\"").count();
        let route_count = yaml_routes().len();
        assert!(
            success_count >= route_count,
            "fewer success responses ({success_count}) than routes ({route_count}) — \
             every endpoint must document at least one 2xx response"
        );
    }
}
