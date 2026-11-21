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
///
/// This is the fallback used when no limit has been installed in shared state
/// (e.g. in unit tests). Production code installs the limit once at startup via
/// [`init_body_limit`] so the middleware never touches the environment.
fn body_limit() -> usize {
    static LIMIT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *LIMIT.get_or_init(|| {
        parse_request_body_max_bytes(std::env::var("REQUEST_BODY_MAX_BYTES").ok().as_deref())
    })
}

/// Shared, process-wide body-size limit, loaded once at startup.
static BODY_LIMIT: std::sync::OnceLock<usize> = std::sync::OnceLock::new();

/// Install the body-size limit into shared state. Subsequent calls are no-ops,
/// so the first value wins and the limit is read exactly once per process.
pub fn init_body_limit(limit: usize) {
    let _ = BODY_LIMIT.set(limit);
}

/// Resolve the configured body-size limit from shared state, falling back to
/// the lazily-read environment value if startup did not install one.
pub fn configured_body_limit() -> usize {
    *BODY_LIMIT.get_or_init(body_limit)
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
    let limit = configured_body_limit();

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

// ── Path parameter validation ─────────────────────────────────────────────────

/// Expected length of a hex-encoded user address (e.g. a 20-byte EVM address).
const USER_ADDRESS_HEX_LEN: usize = 40;

/// Maximum accepted length of a market id.
const MARKET_ID_MAX_LEN: usize = 64;

/// Validate a user address path parameter.
///
/// Accepts a `0x`-prefixed, 40-character hex string (case-insensitive). The
/// value is returned unchanged on success so callers can use it directly as a
/// cache key or RPC argument.
pub fn validate_user_address(value: &str) -> Result<String, ValidationError> {
    let trimmed = value.trim();
    let hex = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"));

    let valid = match hex {
        Some(h) => h.len() == USER_ADDRESS_HEX_LEN && h.chars().all(|c| c.is_ascii_hexdigit()),
        None => false,
    };

    if !valid {
        return Err(ValidationError {
            error:   "invalid_user_address",
            field:   "user_address".to_string(),
            message: "User address must be a 0x-prefixed 40-character hex string.".to_string(),
        });
    }

    Ok(trimmed.to_string())
}

/// Validate a market id path parameter.
///
/// Accepts 1..=`MARKET_ID_MAX_LEN` ASCII alphanumeric characters plus `-` and
/// `_`. The value is returned unchanged on success.
pub fn validate_market_id(value: &str) -> Result<String, ValidationError> {
    let trimmed = value.trim();
    let valid = !trimmed.is_empty()
        && trimmed.len() <= MARKET_ID_MAX_LEN
        && trimmed
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');

    if !valid {
        return Err(ValidationError {
            error:   "invalid_market_id",
            field:   "market_id".to_string(),
            message: format!(
                "Market id must be 1-{} characters of ASCII alphanumerics, '-' or '_'.",
                MARKET_ID_MAX_LEN
            ),
        });
    }

    Ok(trimmed.to_string())
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
        if c == '<' {
            if let Some(&next) = chars.peek() {
                if next.is_ascii_alphabetic() || matches!(next, '/' | '!' | '?') {
                    return true;
                }
            }
        }
    }
    false
}

// ── Disposable email blocklist ────────────────────────────────────────────────

/// Environment variable holding a comma-separated list of disposable email
/// domains to block. Loaded from configuration rather than hard-coded so the
/// list can be updated without a code change.
pub const DISPOSABLE_EMAIL_DOMAINS_ENV: &str = "DISPOSABLE_EMAIL_DOMAINS";

/// Parse a comma-separated disposable-domain list from an optional env-var
/// string. Entries are trimmed, lowercased, and empty entries are dropped.
pub fn parse_disposable_email_domains(val: Option<&str>) -> Vec<String> {
    val.map(|s| {
        s.split(',')
            .map(|d| d.trim().to_lowercase())
            .filter(|d| !d.is_empty())
            .collect()
    })
    .unwrap_or_default()
}

/// Resolve the configured disposable email blocklist.
pub fn disposable_email_domains() -> Vec<String> {
    parse_disposable_email_domains(
        std::env::var(DISPOSABLE_EMAIL_DOMAINS_ENV).ok().as_deref(),
    )
}

/// Returns `true` when the domain part of `email` is on the configured
/// disposable blocklist. Comparison is case-insensitive.
pub fn is_disposable_email(email: &str) -> bool {
    let domain = match email.rsplit_once('@') {
        Some((_, d)) => d.trim().to_lowercase(),
        None => return false,
    };
    disposable_email_domains().iter().any(|blocked| blocked == &domain)
}

/// Validate an email address. Returns a generic error for both malformed and
/// blocked (disposable) addresses so the response never reveals whether the
/// domain is on the blocklist.
pub fn validate_email(email: &str) -> Result<(), ValidationError> {
    let generic = || ValidationError {
        error: "invalid_email",
        field: "email".to_string(),
        message: "Please provide a valid email address.".to_string(),
    };

    let (local, domain) = match email.rsplit_once('@') {
        Some(parts) => parts,
        None => return Err(generic()),
    };
    if local.is_empty() || domain.is_empty() || !domain.contains('.') {
        return Err(generic());
    }
    if is_disposable_email(email) {
        return Err(generic());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn body_limit_is_read_once_and_consistent() {
        // The first installed value wins; later attempts are ignored.
        init_body_limit(4_096);
        init_body_limit(8_192);
        assert_eq!(configured_body_limit(), 4_096);
        assert_eq!(configured_body_limit(), 4_096);
    }

    #[test]
    fn user_address_accepts_valid_and_rejects_invalid() {
        let addr = "0x52908400098527886E0F7030069857D2E4169EE7";
        assert_eq!(validate_user_address(addr).unwrap(), addr);

        assert!(validate_user_address("").is_err());
        assert!(validate_user_address("0x").is_err());
        assert!(validate_user_address("52908400098527886E0F7030069857D2E4169EE7").is_err());
        assert!(validate_user_address("0xZZ908400098527886E0F7030069857D2E4169EE7").is_err());
        assert!(validate_user_address("0x52908400098527886E0F7030069857D2E4169EE").is_err());
    }

    #[test]
    fn market_id_accepts_valid_and_rejects_invalid() {
        assert_eq!(validate_market_id("market-1_A").unwrap(), "market-1_A");

        assert!(validate_market_id("").is_err());
        assert!(validate_market_id("bad id").is_err());
        assert!(validate_market_id("bad/id").is_err());
        assert!(validate_market_id(&"a".repeat(MARKET_ID_MAX_LEN + 1)).is_err());
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
