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
                "Field '{}' must be at least {} characters (got {}).",
                field_name, min_len, sanitized.len()
            ),
        });
    }

    if sanitized.len() > max_len {
        return Err(ValidationError {
            error:   "too_long",
            field:   field_name.to_string(),
            message: format!(
                "Field '{}' must not exceed {} characters (got {}).",
                field_name, max_len, sanitized.len()
            ),
        });
    }

    Ok(sanitized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_removes_simple_tags() {
        assert_eq!(strip_html_tags("<b>hello</b>"), "hello");
    }

    #[test]
    fn strip_preserves_plain_text() {
        let s = "Market closes at 3 PM on Friday.";
        assert_eq!(strip_html_tags(s), s);
    }

    #[test]
    fn detects_script_tag() {
        assert!(contains_injection("<script>alert(1)</script>"));
    }

    #[test]
    fn detects_event_handler() {
        assert!(contains_injection(r#"<img onerror="alert(1)">"#));
    }

    #[test]
    fn detects_javascript_protocol() {
        assert!(contains_injection("javascript:void(0)"));
    }

    #[test]
    fn clean_input_passes() {
        assert!(!contains_injection("Will the S&P 500 close above 5000?"));
    }

    #[test]
    fn sanitize_rejects_script_tags() {
        let err = sanitize_string("title", "<script>evil()</script>").unwrap_err();
        assert_eq!(err.error, "invalid_content");
        assert_eq!(err.field, "title");
    }

    #[test]
    fn sanitize_strips_html_from_clean_html() {
        let result = sanitize_string("title", "<b>Bold Market</b>").unwrap();
        assert_eq!(result, "Bold Market");
    }

    #[test]
    fn sanitize_strips_null_bytes() {
        let result = sanitize_string("title", "Hello\0World").unwrap();
        assert!(!result.contains('\0'));
    }

    #[test]
    fn sanitize_trims_whitespace() {
        let result = sanitize_string("title", "  trimmed  ").unwrap();
        assert_eq!(result, "trimmed");
    }

    #[test]
    fn validate_rejects_too_short() {
        let err = validate_string("title", "hi", 5, 100).unwrap_err();
        assert_eq!(err.error, "too_short");
    }

    #[test]
    fn validate_rejects_too_long() {
        let long = "x".repeat(101);
        let err = validate_string("title", &long, 1, 100).unwrap_err();
        assert_eq!(err.error, "too_long");
    }

    #[test]
    fn validate_accepts_valid_input() {
        let result = validate_string("title", "Will BTC hit 100k?", 5, 200).unwrap();
        assert_eq!(result, "Will BTC hit 100k?");
    }

    #[test]
    fn validate_rejects_encoded_script_tag() {
        let err = validate_string("desc", "&lt;script&gt;", 1, 200).unwrap_err();
        assert_eq!(err.error, "invalid_content");
    }
}
