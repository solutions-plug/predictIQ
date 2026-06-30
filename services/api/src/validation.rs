//! Input validation and sanitization for predictIQ API.
//!
//! ## XSS prevention
//! String fields are sanitized before storage using an allowlist-based approach:
//! - HTML tags are stripped entirely
//! - Script / event-handler patterns are rejected outright
//! - Null bytes and control characters are removed
//!
//! This is a defence-in-depth layer; the frontend MUST also escape output.

use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

const MAX_REQUEST_BODY_BYTES: u64 = 1 * 1024 * 1024; // 1 MB

#[derive(Serialize)]
struct RequestTooLargeError {
    error:      &'static str,
    message:    String,
    max_bytes:  u64,
}

pub async fn content_type_validation_middleware(req: Request<Body>, next: Next) -> Response {
    crate::content_type::require_json_content_type(req, next).await
}

pub async fn request_size_validation_middleware(req: Request<Body>, next: Next) -> Response {
    if let Some(content_length) = req
        .headers()
        .get(axum::http::header::CONTENT_LENGTH)
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
    {
        if content_length > MAX_REQUEST_BODY_BYTES {
            let body = RequestTooLargeError {
                error:     "request_too_large",
                message:   format!(
                    "Request body exceeds the {MAX_REQUEST_BODY_BYTES}-byte limit."
                ),
                max_bytes: MAX_REQUEST_BODY_BYTES,
            };
            return (StatusCode::PAYLOAD_TOO_LARGE, Json(body)).into_response();
        }
    }
    next.run(req).await
}

pub async fn request_validation_middleware(req: Request<Body>, next: Next) -> Response {
    request_size_validation_middleware(req, next).await
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

    // ── Property-based tests ──────────────────────────────────────────────────
    //
    // Run with at least 1 000 cases in CI:
    //   PROPTEST_CASES=1000 cargo test prop_
    #[cfg(test)]
    mod property_tests {
        use super::*;
        use proptest::prelude::*;

        // Maximum field length used in the property tests below.
        const MAX_LEN: usize = 200;

        proptest! {
            // Empty string should be accepted by sanitize_string (length checks
            // happen in validate_string) and produce an empty result.
            #[test]
            fn prop_sanitize_empty_string_is_ok(_ignored in Just(())) {
                let result = sanitize_string("field", "");
                prop_assert!(result.is_ok());
                prop_assert_eq!(result.unwrap(), "");
            }

            // Strings longer than MAX_LEN must be rejected by validate_string.
            #[test]
            fn prop_validate_rejects_over_max_len(
                extra in 1usize..=256,
                ch in '[' ..= '~', // printable ASCII, no injection chars
            ) {
                let s: String = std::iter::repeat(ch).take(MAX_LEN + extra).collect();
                // Skip if the character happens to form an injection pattern — we're
                // testing the length gate, not the injection gate.
                prop_assume!(!contains_injection(&s));
                let result = validate_string("field", &s, 1, MAX_LEN);
                prop_assert!(result.is_err());
                prop_assert_eq!(result.unwrap_err().error, "too_long");
            }

            // Zero-length input must be rejected by validate_string when min_len > 0.
            #[test]
            fn prop_validate_rejects_empty_when_min_len_positive(_ignored in Just(())) {
                let result = validate_string("field", "", 1, MAX_LEN);
                prop_assert!(result.is_err());
                prop_assert_eq!(result.unwrap_err().error, "too_short");
            }

            // All-whitespace strings collapse to "" after trim and should fail
            // the min-length gate when min_len > 0.
            #[test]
            fn prop_all_whitespace_is_rejected(
                spaces in 1usize..=50,
            ) {
                let s: String = " ".repeat(spaces);
                let result = validate_string("field", &s, 1, MAX_LEN);
                prop_assert!(result.is_err());
                prop_assert_eq!(result.unwrap_err().error, "too_short");
            }

            // Null bytes must never appear in sanitized output.
            #[test]
            fn prop_null_bytes_stripped_from_output(
                prefix in "[a-zA-Z0-9 ]{0,20}",
                suffix in "[a-zA-Z0-9 ]{0,20}",
            ) {
                let input = format!("{prefix}\0{suffix}");
                prop_assume!(!contains_injection(&input));
                if let Ok(out) = sanitize_string("field", &input) {
                    prop_assert!(!out.contains('\0'), "null byte survived sanitization");
                }
            }

            // Control characters (except tab/newline/CR) must not appear in output.
            #[test]
            fn prop_control_chars_stripped(
                ctrl in 1u8..=8u8, // \x01–\x08 are stripped
                filler in "[a-z]{1,10}",
            ) {
                let input = format!("{filler}{}{filler}", ctrl as char);
                prop_assume!(!contains_injection(&input));
                if let Ok(out) = sanitize_string("field", &input) {
                    prop_assert!(
                        !out.chars().any(|c| c.is_control() && c != '\t' && c != '\n' && c != '\r'),
                        "control character survived sanitization"
                    );
                }
            }

            // Known-safe strings within length bounds must always pass.
            #[test]
            fn prop_valid_market_titles_pass(
                // Alphanumeric + common punctuation; deliberately no HTML/script chars
                title in "[a-zA-Z0-9 .,!?'\\-]{5,100}",
            ) {
                prop_assume!(!contains_injection(&title));
                let result = validate_string("title", &title, 1, MAX_LEN);
                prop_assert!(result.is_ok(), "valid title was rejected: {:?}", result.err());
            }

            // Unicode non-ASCII (including homograph characters) must never panic
            // and must not produce null bytes in output.
            #[test]
            fn prop_unicode_does_not_panic_or_produce_null(
                s in "\\PC{0,50}", // any non-control Unicode up to 50 chars
            ) {
                // Ignore inputs that trigger the injection guard — we test that
                // separately; here we only care that the function doesn't panic or
                // corrupt output.
                if let Ok(out) = sanitize_string("field", &s) {
                    prop_assert!(!out.contains('\0'));
                }
            }
        }
    }
}
