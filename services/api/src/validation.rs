//! Input validation and sanitization for predictIQ API.
//!
//! ## XSS prevention
//! String fields are sanitized before storage using an allowlist-based approach:
//! - HTML tags are stripped entirely
//! - Script / event-handler patterns are rejected outright
//! - Null bytes and control characters are removed
//!
//! This is a defence-in-depth layer; the frontend MUST also escape output.

use axum::body::Body;
use axum::extract::Request;
use axum::http::{Method, StatusCode};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;

// ── Request body size limit ──────────────────────────────────────────────────

/// Default request body size limit: 1 MiB.
pub const DEFAULT_REQUEST_BODY_MAX_BYTES: usize = 1_048_576;

/// Parse `REQUEST_BODY_MAX_BYTES` from an optional env-var string.
/// Returns the default on missing, zero, or unparseable values.
pub fn parse_request_body_max_bytes(val: Option<&str>) -> usize {
    val.and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_REQUEST_BODY_MAX_BYTES)
}

fn body_limit() -> usize {
    parse_request_body_max_bytes(std::env::var("REQUEST_BODY_MAX_BYTES").ok().as_deref())
}

#[derive(Serialize)]
struct PayloadTooLargeError {
    error: &'static str,
    message: String,
    limit_bytes: usize,
}

/// Tower middleware that enforces a request body size limit.
///
/// Fast-path: rejects immediately when `Content-Length` exceeds the limit.
/// Slow-path: buffers the stream and rejects once accumulated bytes exceed limit.
pub async fn request_size_validation_middleware(
    req: Request,
    next: Next,
) -> Response {
    let limit = body_limit();

    // Fast path: Content-Length header present
    if let Some(cl) = req.headers().get("content-length") {
        if let Ok(s) = cl.to_str() {
            if let Ok(n) = s.parse::<usize>() {
                if n > limit {
                    return payload_too_large(limit);
                }
            }
        }
    }

    // Slow path: buffer stream up to limit+1 bytes.
    // axum::body::to_bytes returns Err when body exceeds the cap — treat that as 413.
    let (parts, body) = req.into_parts();
    let bytes = match axum::body::to_bytes(body, limit + 1).await {
        Ok(b) => b,
        Err(_) => return payload_too_large(limit),
    };
    if bytes.len() > limit {
        return payload_too_large(limit);
    }

    let req = Request::from_parts(parts, Body::from(bytes));
    next.run(req).await
}

fn payload_too_large(limit: usize) -> Response {
    (
        StatusCode::PAYLOAD_TOO_LARGE,
        Json(PayloadTooLargeError {
            error: "payload_too_large",
            message: format!(
                "Request body exceeds the maximum allowed size of {} bytes.",
                limit
            ),
            limit_bytes: limit,
        }),
    )
        .into_response()
}

// ── Webhook event-count limit ────────────────────────────────────────────────

/// Default maximum number of events accepted in a single SendGrid webhook
/// request. SendGrid delivers webhook payloads as a JSON array of events; the
/// byte-size cap alone does not bound how many small events a single request
/// can carry, so we enforce an explicit event-count ceiling as well.
pub const DEFAULT_WEBHOOK_MAX_EVENTS: usize = 1_000;

/// Parse `WEBHOOK_MAX_EVENTS` from an optional env-var string.
/// Returns the default on missing, zero, or unparseable values.
pub fn parse_webhook_max_events(val: Option<&str>) -> usize {
    val.and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_WEBHOOK_MAX_EVENTS)
}

/// Resolve the configured maximum events-per-webhook-request.
pub fn webhook_max_events() -> usize {
    parse_webhook_max_events(std::env::var("WEBHOOK_MAX_EVENTS").ok().as_deref())
}

/// Returns `true` when `event_count` is within the configured webhook batch
/// limit. Callers should reject (or chunk) batches for which this is `false`.
pub fn webhook_batch_within_limit(event_count: usize) -> bool {
    event_count <= webhook_max_events()
}

// ── Content-Type validation ───────────────────────────────────────────────────

const JSON_REQUIRED_METHODS: &[Method] = &[Method::POST, Method::PUT, Method::PATCH];

#[derive(Serialize)]
struct UnsupportedMediaTypeError {
    error: &'static str,
    message: String,
    required: &'static str,
    received: String,
}

/// Reject POST/PUT/PATCH requests whose `Content-Type` is not `application/json`.
pub async fn content_type_validation_middleware(req: Request, next: Next) -> Response {
    if JSON_REQUIRED_METHODS.contains(req.method()) {
        let ct = req
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        if !ct.starts_with("application/json") {
            return (
                StatusCode::UNSUPPORTED_MEDIA_TYPE,
                Json(UnsupportedMediaTypeError {
                    error: "unsupported_media_type",
                    message: "Content-Type must be application/json for POST, PUT, and PATCH \
                              requests."
                        .to_string(),
                    required: "application/json",
                    received: if ct.is_empty() {
                        "not set".to_string()
                    } else {
                        ct.to_string()
                    },
                }),
            )
                .into_response();
        }
    }
    next.run(req).await
}

// ── Query / path validation ───────────────────────────────────────────────────

static SUSPICIOUS_QUERY_PATTERNS: &[&str] = &[
    "' or", "\" or", "1=1", "or 1=1", "drop table", "select ", "insert ",
    "delete ", "update ", "union ", "--", "/*", "*/", "xp_", "exec(",
];

static SUSPICIOUS_PATH_PATTERNS: &[&str] = &["//", "../", "..\\", "%2e%2e"];

/// Reject requests with SQL-injection or path-traversal patterns in query / path.
pub async fn request_validation_middleware(req: Request, next: Next) -> Response {
    let uri = req.uri();

    if let Some(query) = uri.query() {
        let lower = query.to_lowercase();
        if SUSPICIOUS_QUERY_PATTERNS.iter().any(|p| lower.contains(p)) {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({
                    "error": "invalid_request",
                    "message": "Request contains disallowed query patterns."
                })),
            )
                .into_response();
        }
    }

    let path = uri.path();
    let lower_path = path.to_lowercase();
    if SUSPICIOUS_PATH_PATTERNS.iter().any(|p| lower_path.contains(p)) {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({
                "error": "invalid_request",
                "message": "Request path contains disallowed patterns."
            })),
        )
            .into_response();
    }

    next.run(req).await
}

#[derive(Debug, Serialize)]
pub struct ValidationError {
    pub error:   &'static str,
    pub field:   String,
    pub message: String,
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

static REJECT_PATTERNS: &[&str] = &[
    "<script",
    "</script",
    "javascript:",
    "vbscript:",
    "data:text/html",
    "on error=",
    "onerror=",
    "onload=",
    "onclick=",
    "onmouseover=",
    "onfocus=",
    "expression(",
    "&#",
    "&lt;script",
];

fn contains_injection(value: &str) -> bool {
    let lower = value.to_lowercase();
    REJECT_PATTERNS.iter().any(|pat| lower.contains(pat))
}

pub fn strip_html_tags(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;

    for ch in input.chars() {
        match ch {
            '<'          => { in_tag = true; }
            '>'          => { in_tag = false; }
            _ if !in_tag => out.push(ch),
            _            => {}
        }
    }
    out
}

fn strip_control_chars(input: &str) -> String {
    input
        .chars()
        .filter(|&c| c == '\t' || c == '\n' || c == '\r' || (!c.is_control() && c != '\0'))
        .collect()
}

pub fn sanitize_string(
    field_name: &str,
    value: &str,
) -> Result<String, ValidationError> {
    if contains_injection(value) {
        return Err(ValidationError {
            error:   "invalid_content",
            field:   field_name.to_string(),
            message: format!(
                "Field '{}' contains disallowed content (script tags or event handlers).",
                field_name
            ),
        });
    }

    let stripped = strip_html_tags(value);
    let clean    = strip_control_chars(&stripped);
    Ok(clean.trim().to_string())
}

pub fn validate_string(
    field_name: &str,
    value: &str,
    min_len: usize,
    max_len: usize,
) -> Result<String, ValidationError> {
    let sanitized = sanitize_string(field_name, value)?;

    if sanitized.len() < min_len {
        return Err(ValidationError {
            error:   "too_short",
            field:   field_name.to_string(),
            message: format!(
                "Field '{}' must be at least {} characters.",
                field_name, min_len
            ),
        });
    }

    if sanitized.len() > max_len {
        return Err(ValidationError {
            error:   "too_long",
            field:   field_name.to_string(),
            message: format!(
                "Field '{}' must be at most {} characters.",
                field_name, max_len
            ),
        });
    }

    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webhook_max_events_defaults_on_missing_or_invalid() {
        assert_eq!(parse_webhook_max_events(None), DEFAULT_WEBHOOK_MAX_EVENTS);
        assert_eq!(parse_webhook_max_events(Some("")), DEFAULT_WEBHOOK_MAX_EVENTS);
        assert_eq!(parse_webhook_max_events(Some("0")), DEFAULT_WEBHOOK_MAX_EVENTS);
        assert_eq!(parse_webhook_max_events(Some("abc")), DEFAULT_WEBHOOK_MAX_EVENTS);
    }

    #[test]
    fn webhook_max_events_parses_valid_value() {
        assert_eq!(parse_webhook_max_events(Some("250")), 250);
        assert_eq!(parse_webhook_max_events(Some(" 42 ")), 42);
    }

    #[test]
    fn webhook_batch_limit_boundary() {
        // At the limit is accepted; over the limit is rejected.
        let limit = webhook_max_events();
        assert!(webhook_batch_within_limit(limit));
        assert!(webhook_batch_within_limit(limit.saturating_sub(1)));
        assert!(!webhook_batch_within_limit(limit + 1));
    }
}
