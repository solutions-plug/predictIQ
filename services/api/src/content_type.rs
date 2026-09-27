//! Content-Type validation middleware for predictIQ API.
//!
//! POST, PUT, and PATCH requests must carry `Content-Type: application/json`.
//! Requests with missing or incorrect Content-Type receive 415 Unsupported Media Type.

use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

const JSON_REQUIRED_METHODS: &[Method] = &[Method::POST, Method::PUT, Method::PATCH];

#[derive(Serialize)]
struct UnsupportedMediaTypeError {
    error:    &'static str,
    message:  String,
    required: &'static str,
    received: String,
}

/// Returns `true` when the given `Content-Type` header value denotes JSON.
///
/// The media type is compared case-insensitively (RFC 7231 §3.1.1.1) and any
/// parameters (e.g. `charset=utf-8`) are ignored, so `application/json`,
/// `Application/JSON`, and `application/json; charset=utf-8` are all accepted.
/// Malformed values such as `application/json; charset=` or a bare
/// `application/json;` are rejected.
fn is_json_content_type(content_type: &str) -> bool {
    let mut parts = content_type.split(';');
    let media_type = parts.next().unwrap_or("").trim();

    if !media_type.eq_ignore_ascii_case("application/json") {
        return false;
    }

    // Validate any parameters that follow the media type. A trailing `;` with
    // no parameter, or a parameter with an empty value, is malformed.
    for param in parts {
        let param = param.trim();
        if param.is_empty() {
            return false;
        }
        match param.split_once('=') {
            Some((name, value)) => {
                if name.trim().is_empty() || value.trim().is_empty() {
                    return false;
                }
            }
            None => return false,
        }
    }

    true
}

pub async fn require_json_content_type(
    req: Request<Body>,
    next: Next,
) -> Response {
    if !JSON_REQUIRED_METHODS.contains(req.method()) {
        return next.run(req).await;
    }

    let content_type = req
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !is_json_content_type(content_type) {
        let body = UnsupportedMediaTypeError {
            error:    "unsupported_media_type",
            message:  "Content-Type must be application/json for POST, PUT, and PATCH requests. \
                       Ensure the header is set to 'application/json' and the body is valid JSON."
                .to_string(),
            required: "application/json",
            received: if content_type.is_empty() {
                "not set".to_string()
            } else {
                content_type.to_string()
            },
        };
        return (StatusCode::UNSUPPORTED_MEDIA_TYPE, Json(body)).into_response();
    }

    next.run(req).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::Method;

    #[test]
    fn get_methods_not_in_required_list() {
        assert!(!JSON_REQUIRED_METHODS.contains(&Method::GET));
        assert!(!JSON_REQUIRED_METHODS.contains(&Method::DELETE));
    }

    #[test]
    fn post_put_patch_in_required_list() {
        assert!(JSON_REQUIRED_METHODS.contains(&Method::POST));
        assert!(JSON_REQUIRED_METHODS.contains(&Method::PUT));
        assert!(JSON_REQUIRED_METHODS.contains(&Method::PATCH));
    }

    #[test]
    fn content_type_accept_reject_table() {
        // (header value, expected accept)
        let cases: &[(&str, bool)] = &[
            // Plain JSON.
            ("application/json", true),
            // Case-insensitivity of the media type.
            ("Application/JSON", true),
            ("APPLICATION/JSON", true),
            ("application/Json", true),
            // Charset parameter variants.
            ("application/json; charset=utf-8", true),
            ("application/json;charset=utf-8", true),
            ("application/json; charset=UTF-8", true),
            ("application/json; charset=us-ascii", true),
            ("application/json; charset=iso-8859-1", true),
            ("application/json; charset=unknown-charset", true),
            ("application/json; boundary=something", true),
            ("application/json; charset=utf-8; boundary=x", true),
            // Whitespace tolerance around the media type.
            ("  application/json  ", true),
            // Malformed charset parameters must be rejected.
            ("application/json; charset=", false),
            ("application/json; charset", false),
            ("application/json;", false),
            ("application/json; ", false),
            ("application/json; charset=utf-8;", false),
            // Wrong or missing media type.
            ("application/xml", false),
            ("text/plain", false),
            ("application/json-patch+json", false),
            ("application/jsonx", false),
            ("", false),
        ];

        for (value, expected) in cases {
            assert_eq!(
                is_json_content_type(value),
                *expected,
                "unexpected decision for Content-Type {value:?}"
            );
        }
    }

    #[test]
    fn case_insensitive_media_type_is_accepted() {
        assert!(is_json_content_type("Application/JSON"));
        assert!(is_json_content_type("APPLICATION/JSON; CHARSET=UTF-8"));
    }

    #[test]
    fn empty_charset_value_is_rejected() {
        assert!(!is_json_content_type("application/json; charset="));
    }

    #[test]
    fn trailing_semicolon_is_rejected() {
        assert!(!is_json_content_type("application/json;"));
    }

    #[test]
    fn unknown_charset_is_accepted() {
        assert!(is_json_content_type("application/json; charset=made-up"));
    }
}
