pub use crate::body_redact::{body_logging_enabled, redact_sensitive, truncate_body};

use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{
    audit::{create_audit_entry, AuditStatus},
    AppState,
};

// ── auth-failure reason classification ───────────────────────────────────────

/// Classify the reason a request received a 401 response.
///
/// The classification is based solely on which credential headers were present,
/// not on their value — we never store full secrets in the audit trail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthFailureReason {
    /// An `x-api-key` header was present but the key was not recognised.
    InvalidApiKey,
    /// An `Authorization` header was present but the token was expired/invalid.
    ExpiredToken,
    /// Neither `x-api-key` nor `Authorization` was supplied.
    MissingCredentials,
}

impl AuthFailureReason {
    /// Stable label string used in Prometheus metrics and audit log action field.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::InvalidApiKey => "invalid_api_key",
            Self::ExpiredToken => "expired_token",
            Self::MissingCredentials => "missing_credentials",
        }
    }

    fn from_headers(headers: &HeaderMap) -> Self {
        if headers.contains_key("x-api-key") {
            Self::InvalidApiKey
        } else if headers.contains_key("authorization") {
            Self::ExpiredToken
        } else {
            Self::MissingCredentials
        }
    }
}

// ── key-prefix helper ─────────────────────────────────────────────────────────

/// Return the first 4 characters of an API key, padded/masked for safe logging.
///
/// Never logs or stores the full key value.
fn key_prefix(key: &str) -> String {
    let chars: String = key.chars().take(4).collect();
    format!("{}****", chars)
}

// ── middleware ────────────────────────────────────────────────────────────────

/// Middleware that automatically logs admin operations **and** authentication
/// failures to the audit trail.
///
/// For every request this middleware:
/// 1. Runs the inner handler.
/// 2. If the response is `401 Unauthorized`, creates an `auth_failure` audit
///    entry that includes the failure reason, the first 4 chars of the
///    attempted API key (if present), client IP, and user-agent.  The
///    `auth_failures_total{reason=...}` Prometheus counter is also incremented.
/// 3. For all responses, creates a standard admin-operation audit entry.
pub async fn audit_logging_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    // ── capture request metadata ─────────────────────────────────────────────
    let actor = headers
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|k| format!("api_key:{}", &k[..8.min(k.len())]))
        .or_else(|| {
            headers
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "unknown".to_string());

    let actor_ip = Some(addr.ip());
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let request_id = Uuid::new_v4();

    // Stash context for downstream handlers.
    request.extensions_mut().insert(actor.clone());
    request.extensions_mut().insert(request_id);

    let method = request.method().clone();
    let uri = request.uri().clone();

    // ── execute the request ──────────────────────────────────────────────────
    let response = next.run(request).await;

    let status_code = response.status();

    // ── auth-failure audit entry (401 only) ──────────────────────────────────
    if status_code == StatusCode::UNAUTHORIZED {
        let reason = AuthFailureReason::from_headers(&headers);

        // Build a masked actor string that never contains the full credential.
        let auth_actor = match reason {
            AuthFailureReason::InvalidApiKey => {
                let prefix = headers
                    .get("x-api-key")
                    .and_then(|v| v.to_str().ok())
                    .map(|k| key_prefix(k))
                    .unwrap_or_else(|| "****".to_string());
                format!("api_key_attempt:{}", prefix)
            }
            AuthFailureReason::ExpiredToken => "token_attempt:****".to_string(),
            AuthFailureReason::MissingCredentials => "anonymous".to_string(),
        };

        tracing::warn!(
            reason = reason.as_str(),
            actor = %auth_actor,
            client_ip = %addr.ip(),
            user_agent = ?user_agent,
            path = %uri.path(),
            "Authentication failure"
        );

        // Increment Prometheus counter.
        state.metrics.observe_auth_failure(reason.as_str());

        // Write audit entry asynchronously.
        let mut auth_entry = create_audit_entry(
            auth_actor,
            actor_ip,
            "auth_failure".to_string(),
            "authentication".to_string(),
            None,
            Some(serde_json::json!({
                "reason": reason.as_str(),
                "path": uri.path(),
                "method": method.as_str(),
            })),
            Some(request_id),
            user_agent.clone(),
        );
        auth_entry.status = AuditStatus::Failure;
        auth_entry.error_message = Some(format!(
            "Authentication failed: {}",
            reason.as_str()
        ));

        let audit_logger = state.audit_logger.clone();
        tokio::spawn(async move {
            if let Err(e) = audit_logger.log(auth_entry).await {
                tracing::error!("Failed to write auth-failure audit log: {}", e);
            }
        });
    }

    // ── standard admin-operation audit entry ─────────────────────────────────
    let path = uri.path();
    let (action, resource_type, resource_id) = parse_admin_action(path, &method);

    let status = if status_code.is_success() {
        AuditStatus::Success
    } else {
        AuditStatus::Failure
    };

    let error_message = if !status_code.is_success() {
        Some(format!("HTTP {}", status_code))
    } else {
        None
    };

    let mut entry = create_audit_entry(
        actor,
        actor_ip,
        action,
        resource_type,
        resource_id,
        None,
        Some(request_id),
        user_agent,
    );
    entry.status = status;
    entry.error_message = error_message;

    let audit_logger = state.audit_logger.clone();
    tokio::spawn(async move {
        if let Err(e) = audit_logger.log(entry).await {
            tracing::error!("Failed to write audit log: {}", e);
        }
    });

    response
}

// ── path parser ───────────────────────────────────────────────────────────────

/// Map a request path + method to a human-readable (action, resource_type, resource_id).
fn parse_admin_action(path: &str, method: &axum::http::Method) -> (String, String, Option<String>) {
    if path.contains("/markets/") && path.contains("/resolve") {
        let market_id = path
            .split('/')
            .find_map(|s| s.parse::<i64>().ok())
            .map(|id| id.to_string());
        ("resolve_market".to_string(), "market".to_string(), market_id)
    } else if path.contains("/email/preview/") {
        let template = path.split('/').last().map(|s| s.to_string());
        ("preview_email".to_string(), "email_template".to_string(), template)
    } else if path.contains("/email/test") {
        ("send_test_email".to_string(), "email".to_string(), None)
    } else if path.contains("/email/analytics") {
        (
            "view_email_analytics".to_string(),
            "email_analytics".to_string(),
            None,
        )
    } else if path.contains("/email/queue/stats") {
        (
            "view_queue_stats".to_string(),
            "email_queue".to_string(),
            None,
        )
    } else if path.contains("/email/queue/dead-letter") && path.contains("/requeue") {
        let job_id = path.split('/').nth_back(1).map(|s| s.to_string());
        (
            "requeue_dead_letter".to_string(),
            "email_queue".to_string(),
            job_id,
        )
    } else if path.contains("/email/queue/dead-letter") {
        (
            "list_dead_letter".to_string(),
            "email_queue".to_string(),
            None,
        )
    } else if path.contains("/audit/logs") {
        (
            "query_audit_logs".to_string(),
            "audit_log".to_string(),
            None,
        )
    } else {
        let action = format!(
            "{}_{}",
            method.as_str().to_lowercase(),
            path.replace('/', "_")
        );
        ("admin_action".to_string(), "unknown".to_string(), Some(action))
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;

    // ── AuthFailureReason::from_headers ──────────────────────────────────────

    #[test]
    fn reason_invalid_api_key_when_x_api_key_present() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "bad-key-value".parse().unwrap());
        assert_eq!(
            AuthFailureReason::from_headers(&headers),
            AuthFailureReason::InvalidApiKey
        );
    }

    #[test]
    fn reason_expired_token_when_authorization_present() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer expired.token.here".parse().unwrap());
        assert_eq!(
            AuthFailureReason::from_headers(&headers),
            AuthFailureReason::ExpiredToken
        );
    }

    #[test]
    fn reason_missing_credentials_when_no_auth_headers() {
        let headers = HeaderMap::new();
        assert_eq!(
            AuthFailureReason::from_headers(&headers),
            AuthFailureReason::MissingCredentials
        );
    }

    // x-api-key takes precedence over Authorization
    #[test]
    fn reason_prefers_api_key_over_authorization() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "bad".parse().unwrap());
        headers.insert("authorization", "Bearer tok".parse().unwrap());
        assert_eq!(
            AuthFailureReason::from_headers(&headers),
            AuthFailureReason::InvalidApiKey
        );
    }

    // ── key_prefix ───────────────────────────────────────────────────────────

    #[test]
    fn key_prefix_masks_all_but_first_four_chars() {
        assert_eq!(key_prefix("sk-live-abc123"), "sk-l****");
    }

    #[test]
    fn key_prefix_handles_short_key() {
        assert_eq!(key_prefix("ab"), "ab****");
    }

    #[test]
    fn key_prefix_empty_key() {
        assert_eq!(key_prefix(""), "****");
    }

    // ── AuthFailureReason::as_str ────────────────────────────────────────────

    #[test]
    fn as_str_values_are_stable() {
        assert_eq!(AuthFailureReason::InvalidApiKey.as_str(), "invalid_api_key");
        assert_eq!(AuthFailureReason::ExpiredToken.as_str(), "expired_token");
        assert_eq!(
            AuthFailureReason::MissingCredentials.as_str(),
            "missing_credentials"
        );
    }

    // ── parse_admin_action ───────────────────────────────────────────────────

    #[test]
    fn parse_resolve_market_action() {
        let (action, resource_type, resource_id) =
            parse_admin_action("/admin/markets/42/resolve", &axum::http::Method::POST);
        assert_eq!(action, "resolve_market");
        assert_eq!(resource_type, "market");
        assert_eq!(resource_id.as_deref(), Some("42"));
    }

    #[test]
    fn parse_audit_logs_action() {
        let (action, resource_type, _) =
            parse_admin_action("/admin/audit/logs", &axum::http::Method::GET);
        assert_eq!(action, "query_audit_logs");
        assert_eq!(resource_type, "audit_log");
    }

    #[test]
    fn parse_unknown_path_falls_back_to_admin_action() {
        let (action, resource_type, _) =
            parse_admin_action("/admin/something-new", &axum::http::Method::GET);
        assert_eq!(action, "admin_action");
        assert_eq!(resource_type, "unknown");
    }
}
