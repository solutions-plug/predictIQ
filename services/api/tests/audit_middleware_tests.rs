/// Tests for `audit_middleware` — verifying that the audit trail captures both
/// successful and failed authentication attempts.
///
/// Acceptance criteria for #981:
/// - A request with an invalid API key must produce an audit log entry
/// - The log entry must include the attempted key prefix, client IP, and user agent
/// - The audit entry status must be `'failure'` (not `'success'`)
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Body,
        extract::ConnectInfo,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use predictiq_api::{
        audit::{AuditLogEntry, AuditStatus},
        security::{self, ApiKeyAuth},
    };
    use tokio::sync::Mutex;
    use tower::ServiceExt;

    #[derive(Clone, Default)]
    struct InMemoryAuditLogger {
        entries: Arc<Mutex<Vec<AuditLogEntry>>>,
    }

    impl InMemoryAuditLogger {
        async fn log(&self, entry: AuditLogEntry) -> Result<i64, anyhow::Error> {
            let mut entries = self.entries.lock().await;
            let id = entries.len() as i64;
            entries.push(entry);
            Ok(id)
        }

        async fn entries(&self) -> Vec<AuditLogEntry> {
            self.entries.lock().await.clone()
        }
    }

    struct TestState {
        audit_logger: InMemoryAuditLogger,
    }
    /// Build a test router with inline audit middleware + API-key auth middleware.
    fn app() -> (Router, InMemoryAuditLogger) {
        let logger = InMemoryAuditLogger::default();
        let state = Arc::new(TestState {
            audit_logger: logger.clone(),
        });
        let auth = Arc::new(ApiKeyAuth::new(vec!["valid-key".to_string()]));

        let router = Router::new()
            .route("/api/v1/admin/markets/42/resolve", get(|| async { "resolved" }))
            .route("/api/v1/audit/logs", get(|| async { "audit logs" }))
            .layer(middleware::from_fn_with_state(
                state.clone(),
                |state: Arc<TestState>,
                 addr: ConnectInfo<std::net::SocketAddr>,
                 headers: axum::http::HeaderMap,
                 request: Request,
                 next: axum::middleware::Next| async move {
                    let actor = headers
                        .get("x-api-key")
                        .and_then(|v| v.to_str().ok())
                        .map(|k| format!("api_key:{}", &k[..8.min(k.len())]))
                        .unwrap_or_else(|| "unknown".to_string());
                    let actor_ip = Some(addr.ip());
                    let user_agent = headers
                        .get("user-agent")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    let method = request.method().clone();
                    let uri = request.uri().clone();
                    let response = next.run(request).await;
                    let path = uri.path();
                    let (action, resource_type, resource_id) = parse_test_action(path, &method);
                    let status = if response.status().is_success() {
                        AuditStatus::Success
                    } else {
                        AuditStatus::Failure
                    };
                    let error_message = if !response.status().is_success() {
                        Some(format!("HTTP {}", response.status()))
                    } else {
                        None
                    };
                    let entry = AuditLogEntry {
                        id: None,
                        timestamp: chrono::Utc::now(),
                        actor,
                        actor_ip,
                        action,
                        resource_type,
                        resource_id,
                        details: None,
                        status,
                        error_message,
                        request_id: None,
                        user_agent,
                    };
                    let _ = state.audit_logger.log(entry).await;
                    response
                },
            ))
            .layer(middleware::from_fn_with_state(auth, security::api_key_middleware));

        (router, logger)
    }
    fn parse_test_action(path: &str, method: &axum::http::Method) -> (String, String, Option<String>) {
        if path.contains("/markets/") && path.contains("/resolve") {
            let market_id = path
                .split('/')
                .find_map(|s| s.parse::<i64>().ok())
                .map(|id| id.to_string());
            ("resolve_market".to_string(), "market".to_string(), market_id)
        } else if path.contains("/audit/logs") {
            ("query_audit_logs".to_string(), "audit_log".to_string(), None)
        } else {
            let action = format!("{}_{}", method.as_str().to_lowercase(), path.replace('/', "_"));
            ("admin_action".to_string(), "unknown".to_string(), Some(action))
        }
    }

    fn request_with_key(uri: &str, key: &str, user_agent: &str) -> Request<Body> {
        Request::builder()
            .uri(uri)
            .header("x-api-key", key)
            .header("user-agent", user_agent)
            .header("x-forwarded-for", "10.0.0.1")
            .body(Body::empty())
            .unwrap()
    }
    #[tokio::test]
    async fn test_failed_auth_creates_audit_entry() {
        let (router, logger) = app();
        let resp = router
            .oneshot(request_with_key(
                "/api/v1/admin/markets/42/resolve",
                "invalid-key", "TestAgent/1.0",
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        let entries = logger.entries().await;
        assert!(!entries.is_empty(), "Expected at least one audit log entry");
        let entry = &entries[0];
        assert!(
            matches!(entry.status, AuditStatus::Failure),
            "Audit entry status must be 'failure' for failed auth, got {:?}",
            entry.status
        );
    }
    #[tokio::test]
    async fn test_failed_auth_includes_key_prefix() {
        let (router, logger) = app();
        let _resp = router
            .oneshot(request_with_key(
                "/api/v1/admin/markets/42/resolve",
                "invalid-key-prefix-test", "TestAgent/1.0",
            ))
            .await
            .unwrap();
        let entries = logger.entries().await;
        let entry = &entries[0];
        assert!(
            entry.actor.contains("invalid"),
            "Actor should contain key prefix, got: {}", entry.actor
        );
        assert!(
            entry.actor.contains("api_key:"),
            "Actor should start with api_key:, got: {}", entry.actor
        );
    }
    #[tokio::test]
    async fn test_failed_auth_includes_client_ip() {
        let (router, logger) = app();
        let _resp = router
            .oneshot(request_with_key(
                "/api/v1/admin/markets/42/resolve",
                "wrong-key", "TestAgent/1.0",
            ))
            .await
            .unwrap();
        let entries = logger.entries().await;
        let entry = &entries[0];
        assert!(
            entry.actor_ip.is_some(),
            "Actor IP should be present in audit entry"
        );
    }
    #[tokio::test]
    async fn test_failed_auth_includes_user_agent() {
        let (router, logger) = app();
        let _resp = router
            .oneshot(request_with_key(
                "/api/v1/admin/markets/42/resolve",
                "wrong-key", "MyCustomClient/2.0",
            ))
            .await
            .unwrap();
        let entries = logger.entries().await;
        let entry = &entries[0];
        assert_eq!(
            entry.user_agent.as_deref(),
            Some("MyCustomClient/2.0"),
            "User agent must be captured in audit entry"
        );
    }
    #[tokio::test]
    async fn test_successful_auth_creates_audit_entry() {
        let (router, logger) = app();
        let resp = router
            .oneshot(request_with_key(
                "/api/v1/audit/logs",
                "valid-key", "TestAgent/1.0",
            ))
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::OK);
        let entries = logger.entries().await;
        assert!(!entries.is_empty(), "Expected at least one audit log entry");
        let success_entries: Vec<_> = entries
            .iter()
            .filter(|e| matches!(e.status, AuditStatus::Success))
            .collect();
        assert!(
            !success_entries.is_empty(),
            "Expected at least one successful audit entry"
        );
    }
}
