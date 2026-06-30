pub use crate::body_redact::{body_logging_enabled, redact_sensitive, truncate_body};

use std::sync::Arc;

use axum::{
    extract::{ConnectInfo, Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

use crate::{audit::{create_audit_entry, AuditStatus}, AppState};

/// Middleware to automatically log admin operations
pub async fn audit_logging_middleware(
    State(state): State<Arc<AppState>>,
    ConnectInfo(addr): ConnectInfo<std::net::SocketAddr>,
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    // Extract actor from API key or auth header
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
    
    // Store request context for handler access
    request.extensions_mut().insert(actor.clone());
    request.extensions_mut().insert(request_id);
    
    let method = request.method().clone();
    let uri = request.uri().clone();
    
    // Execute the request
    let response = next.run(request).await;
    
    // Determine action and resource from path
    let path = uri.path();
    let (action, resource_type, resource_id) = parse_admin_action(path, &method);
    
    // Determine status from response
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
    
    // Create audit log entry
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
    
    // Log asynchronously (don't block response)
    let audit_logger = state.audit_logger.clone();
    tokio::spawn(async move {
        if let Err(e) = audit_logger.log(entry).await {
            tracing::error!("Failed to write audit log: {}", e);
        }
    });
    
    response
}

/// Parse admin action from request path and method
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
        ("view_email_analytics".to_string(), "email_analytics".to_string(), None)
    } else if path.contains("/email/queue/stats") {
        ("view_queue_stats".to_string(), "email_queue".to_string(), None)
    } else if path.contains("/email/queue/dead-letter") && path.contains("/requeue") {
        let job_id = path.split('/').nth_back(1).map(|s| s.to_string());
        ("requeue_dead_letter".to_string(), "email_queue".to_string(), job_id)
    } else if path.contains("/email/queue/dead-letter") {
        ("list_dead_letter".to_string(), "email_queue".to_string(), None)
    } else if path.contains("/audit/logs") {
        ("query_audit_logs".to_string(), "audit_log".to_string(), None)
    } else {
        let action = format!("{}_{}", method.as_str().to_lowercase(), path.replace('/', "_"));
        ("admin_action".to_string(), "unknown".to_string(), Some(action))
    }
}
