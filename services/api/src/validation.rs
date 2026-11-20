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

// ── Outcome index validation ──────────────────────────────────────────────────

/// Validate a market `outcome_index` before it is cast to `i32` and submitted
/// on-chain.
///
/// Guards against two failure modes:
/// - Overflow: `outcome_index` must fit in `i32` (the on-chain type).
/// - Out-of-range: `outcome_index` must be strictly less than the market's
///   outcome count, so a market cannot be resolved to a nonexistent outcome.
///
/// Returns the validated `i32` index on success, or a `ValidationError`
/// (rendered as HTTP 400) describing the offending field.
pub fn validate_outcome_index(outcome_index: u32, outcome_count: u32) -> Result<i32, ValidationError> {
    if outcome_index > i32::MAX as u32 {
        return Err(ValidationError {
            error:   "invalid_outcome_index",
            field:   "outcome_index".to_string(),
            message: format!(
                "outcome_index {} exceeds the maximum supported value of {}.",
                outcome_index,
                i32::MAX
            ),
        });
    }

    if outcome_index >= outcome_count {
        return Err(ValidationError {
            error:   "invalid_outcome_index",
            field:   "outcome_index".to_string(),
            message: format!(
                "outcome_index {} is out of range for a market with {} outcome(s).",
                outcome_index, outcome_count
            ),
        });
    }

    Ok(outcome_index as i32)
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
        

/* … truncated 3328 chars — edit only what you need near the top … */
