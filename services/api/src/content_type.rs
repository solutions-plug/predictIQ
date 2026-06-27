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

    if !content_type.starts_with("application/json") {
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
}
