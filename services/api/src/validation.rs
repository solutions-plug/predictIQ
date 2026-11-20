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

/// Read once at first use and cached for the process lifetime.
fn body_limit() -> usize {
    static LIMIT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *LIMIT.get_or_init(|| {
        parse_request_body_max_bytes(std::env::var("REQUEST_BODY_MAX_BYTES").ok().as_deref())
    })
}

/// Maximum time allowed to read a request body.
const BODY_READ_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

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
    let bytes = match tokio::time::timeout(BODY_READ_TIMEOUT, axum::body::to_bytes(body, limit + 1)).await {
        Ok(Ok(b)) => b,
        Ok(Err(_)) => return payload_too_large(limit),
        Err(_) => return StatusCode::REQUEST_TIMEOUT.into_response(),
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

// ── Path validation ───────────────────────────────────────────────────────────

static SUSPICIOUS_PATH_PATTERNS: &[&str] = &["../", "..\\", "%2e%2e"];

/// Reject requests with path-traversal patterns in the path.
///
/// Query strings are intentionally not pattern-matched: all SQL uses bound
/// parameters, and substring blocklists reject valid input (e.g. base64url
/// cursors containing `--`) while being bypassed by percent-encoding.
pub async fn request_validation_middleware(req: Request, next: Next) -> Response {
    let lower_path = req.uri().path().to_lowercase();
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

/// True when `input` contains something that parses as an HTML tag, comment or
/// doctype (`<` followed by a letter, `/`, `!` or `?`). A bare `<` such as in
/// `5 < 6` is not markup.
fn contains_markup(input: &str) -> bool {
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '<' && matches!(chars.peek(), Some(n) if n.is_ascii_alphabetic() || matches!(n, '/' | '!' | '?')) {
            return true;
        }
    }
    false
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
    if contains_injection(value) || contains_markup(value) {
        return Err(ValidationError {
            error:   "invalid_content",
            field:   field_name.to_string(),
            message: format!(
                "Field '{}' contains disallowed content (markup, script tags or event handlers).",
                field_name
            ),
        });
    }

    let clean = strip_control_chars(value);
    Ok(clean.trim().to_string())
}

pub fn validate_string(
    field_name: &str,
    value: &str,
    min_len: usize,
    max_len: usize,
) -> Result<String, ValidationError> {
    let sanitized = sanitize_string(field_name, value)?;

    if sanitized.chars().count() < min_len {
        return Err(ValidationError {
            error:   "too_short",
            field:   field_name.to_string(),
            message: format!(
                "Field '{}' must be at least {} characters.",
                field_name, min_len
            ),
        });
    }

    if sanitized.chars().count() > max_len {
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
    fn sanitize_keeps_comparisons_and_rejects_markup() {
        assert_eq!(sanitize_string("f", "a < b and c > d").unwrap(), "a < b and c > d");
        assert_eq!(sanitize_string("f", "5 < 6").unwrap(), "5 < 6");
        assert!(sanitize_string("f", "<b>hi</b>").is_err());
        assert!(sanitize_string("f", "x <img src=1>").is_err());
    }

    #[test]
    fn validate_string_counts_chars_not_bytes() {
        assert!(validate_string("f", "ééééé", 1, 5).is_ok());
        assert!(validate_string("f", "éééééé", 1, 5).is_err());
    }

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

    #[test]
    fn parse_request_body_max_bytes_table() {
        let cases: &[(Option<&str>, usize)] = &[
            (None, DEFAULT_REQUEST_BODY_MAX_BYTES),
            (Some(""), DEFAULT_REQUEST_BODY_MAX_BYTES),
            (Some("   "), DEFAULT_REQUEST_BODY_MAX_BYTES),
            (Some("0"), DEFAULT_REQUEST_BODY_MAX_BYTES),
            (Some("-1"), DEFAULT_REQUEST_BODY_MAX_BYTES),
            (Some("1.5"), DEFAULT_REQUEST_BODY_MAX_BYTES),
            (Some("12abc"), DEFAULT_REQUEST_BODY_MAX_BYTES),
            (Some("1"), 1),
            (Some(" 2048 "), 2048),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_request_body_max_bytes(*input), *expected, "input {:?}", input);
        }
    }

    #[test]
    fn webhook_max_events_whitespace_and_negative_use_default() {
        for input in ["  ", "-5", "1e3"] {
            assert_eq!(parse_webhook_max_events(Some(input)), DEFAULT_WEBHOOK_MAX_EVENTS, "input {:?}", input);
        }
        assert_eq!(parse_webhook_max_events(Some("1")), 1);
    }

    #[test]
    fn webhook_batch_zero_events_is_within_limit() {
        assert!(webhook_batch_within_limit(0));
    }

    #[test]
    fn validate_string_length_boundaries() {
        // (input, min, max, ok)
        let cases: &[(&str, usize, usize, bool)] = &[
            ("abc", 3, 5, true),   // exactly min
            ("ab", 3, 5, false),   // just below min
            ("abcde", 3, 5, true), // exactly max
            ("abcdef", 3, 5, false), // just above max
            ("", 0, 5, true),      // empty allowed when min is 0
            ("", 1, 5, false),     // empty rejected when min >= 1
        ];
        for (input, min, max, ok) in cases {
            assert_eq!(validate_string("f", input, *min, *max).is_ok(), *ok, "input {:?}", input);
        }
    }

    #[test]
    fn validate_string_error_codes() {
        assert_eq!(validate_string("f", "ab", 3, 5).unwrap_err().error, "too_short");
        assert_eq!(validate_string("f", "abcdef", 3, 5).unwrap_err().error, "too_long");
        let e = validate_string("name", "<script>x</script>", 0, 100).unwrap_err();
        assert_eq!(e.error, "invalid_content");
        assert_eq!(e.field, "name");
    }

    #[test]
    fn validate_string_whitespace_only_is_rejected_when_min_is_one() {
        for input in ["   ", "\t\n", " \r\n "] {
            assert_eq!(validate_string("f", input, 1, 10).unwrap_err().error, "too_short", "input {:?}", input);
        }
    }

    #[test]
    fn sanitize_trims_and_strips_control_chars() {
        assert_eq!(sanitize_string("f", "  hi  ").unwrap(), "hi");
        assert_eq!(sanitize_string("f", "a\0b\u{7}c").unwrap(), "abc");
        assert_eq!(sanitize_string("f", "").unwrap(), "");
        assert_eq!(sanitize_string("f", "   ").unwrap(), "");
    }

    #[test]
    fn sanitize_rejects_malformed_content() {
        for input in ["<script>alert(1)</script>", "<img src=x onerror=alert(1)>", "javascript:alert(1)", "<div>x</div>"] {
            assert!(sanitize_string("f", input).is_err(), "input {:?}", input);
        }
    }
}
