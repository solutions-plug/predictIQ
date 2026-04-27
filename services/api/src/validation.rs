use axum::{
    body::Body,
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

use crate::security::sanitize;

/// Default request body size limit: 1 MiB.
///
/// Override at runtime with the `REQUEST_BODY_MAX_BYTES` environment variable.
/// The value must be a positive integer (bytes).  Zero and non-numeric values
/// fall back to this default.
///
/// ```text
/// REQUEST_BODY_MAX_BYTES=524288   # 512 KiB
/// REQUEST_BODY_MAX_BYTES=2097152  # 2 MiB
/// ```
pub const DEFAULT_REQUEST_BODY_MAX_BYTES: usize = 1_048_576; // 1 MiB
const REQUEST_BODY_MAX_BYTES_ENV: &str = "REQUEST_BODY_MAX_BYTES";

#[derive(Serialize)]
pub struct ValidationError {
    pub error: String,
    pub message: String,
}

impl IntoResponse for ValidationError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

pub fn parse_request_body_max_bytes(raw: Option<&str>) -> usize {
    raw.and_then(|value| value.trim().parse::<usize>().ok())
        .filter(|bytes| *bytes > 0)
        .unwrap_or(DEFAULT_REQUEST_BODY_MAX_BYTES)
}

pub fn request_body_max_bytes_from_env() -> usize {
    parse_request_body_max_bytes(std::env::var(REQUEST_BODY_MAX_BYTES_ENV).ok().as_deref())
}

/// Request validation middleware — applied globally to all routes.
///
/// Rejects requests with `400 Bad Request` when:
/// - Query string or path contains SQL injection patterns (detected via [`sanitize::contains_sql_injection`])
/// - Query string exceeds 2 048 characters
/// - Path contains `..` (directory traversal) or `//` (double-slash)
///
/// Safe traffic passes through unmodified.
pub async fn request_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, ValidationError> {
    // Extract and validate query parameters
    let uri = request.uri();
    let query = uri.query().unwrap_or("");

    // Check for SQL injection patterns in query
    if sanitize::contains_sql_injection(query) {
        return Err(ValidationError {
            error: "invalid_input".to_string(),
            message: "Invalid characters detected in request".to_string(),
        });
    }

    // Check for excessively long query strings
    if query.len() > 2048 {
        return Err(ValidationError {
            error: "invalid_input".to_string(),
            message: "Query string too long".to_string(),
        });
    }

    // Validate path parameters
    let path = uri.path();
    if sanitize::contains_sql_injection(path) {
        return Err(ValidationError {
            error: "invalid_input".to_string(),
            message: "Invalid characters detected in path".to_string(),
        });
    }

    // Check for path traversal attempts
    if path.contains("..") || path.contains("//") {
        return Err(ValidationError {
            error: "invalid_input".to_string(),
            message: "Invalid path format".to_string(),
        });
    }

    Ok(next.run(request).await)
}

/// Content-Type validation for POST/PUT/PATCH requests on JSON endpoints.
///
/// Accepts `application/json` with optional parameters (e.g. `charset=utf-8`).
/// Any other content type — or a missing header — is rejected with **415 Unsupported
/// Media Type** so that non-JSON bodies never reach JSON handlers.
///
/// This middleware must be placed *before* body-parsing extractors in the layer
/// stack so that invalid requests are short-circuited early.
pub async fn content_type_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method();

    // Only validate mutating methods; GET/HEAD/DELETE/OPTIONS pass through.
    if matches!(method.as_str(), "POST" | "PUT" | "PATCH") {
        let ct = request
            .headers()
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        // Accept "application/json" with any optional parameters such as
        // "; charset=utf-8".  Everything else — including an absent header
        // (ct == "") — is rejected with 415.
        let mime_type = ct.split(';').next().unwrap_or("").trim();
        if !mime_type.eq_ignore_ascii_case("application/json") {
            return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
        }
    }

    Ok(next.run(request).await)
}

/// Request body size guard.
///
/// Enforces `REQUEST_BODY_MAX_BYTES` (default: 1 MiB) against the **actual
/// body stream**, not just the `Content-Length` header.  This prevents both:
///
/// * Chunked-transfer requests that carry no `Content-Length` header.
/// * Clients that send a small `Content-Length` but stream a larger body.
///
/// The `Content-Length` header is still checked first as a cheap fast-path so
/// that obviously oversized requests are rejected before any bytes are read.
///
/// Returns **413 Payload Too Large** when the limit is exceeded.
pub async fn request_size_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let max_bytes = request_body_max_bytes_from_env();

    // ── Fast-path: reject on Content-Length before touching the body ──────────
    if let Some(content_length) = request.headers().get(axum::http::header::CONTENT_LENGTH) {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                if length > max_bytes {
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
            }
        }
    }

    // ── Stream guard: buffer up to max_bytes + 1 and reject on overflow ───────
    //
    // `axum::body::to_bytes` with a limit returns `LengthLimitError` when the
    // body exceeds `max_bytes`, covering chunked-transfer and any body that
    // omits or lies about `Content-Length`.  We then reconstruct the request
    // with the buffered bytes so downstream handlers can read it normally.
    let (parts, body) = request.into_parts();

    let bytes = axum::body::to_bytes(body, max_bytes)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;

    let request = Request::from_parts(parts, Body::from(bytes));
    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request, middleware, routing::get, Router};
    use tower::ServiceExt;

    async fn validation_app() -> Router {
        Router::new()
            .route("/api/v1/items/:id", get(|| async { "ok" }))
            .layer(middleware::from_fn(request_validation_middleware))
    }

    // ── request_validation_middleware ─────────────────────────────────────

    #[tokio::test]
    async fn allows_clean_request() {
        let response = validation_app()
            .await
            .oneshot(
                Request::builder()
                    .uri("/api/v1/items/42?sort=asc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn blocks_sql_injection_in_query() {
        let response = validation_app()
            .await
            .oneshot(
                Request::builder()
                    .uri("/api/v1/items/42?id=1%20OR%201%3D1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn blocks_sql_injection_in_path() {
        let response = validation_app()
            .await
            .oneshot(
                Request::builder()
                    .uri("/api/v1/items/1%20UNION%20SELECT%201")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn blocks_path_traversal() {
        let response = validation_app()
            .await
            .oneshot(
                Request::builder()
                    .uri("/api/v1/items/../secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn blocks_query_string_too_long() {
        let long_query = "a=".to_string() + &"x".repeat(2048);
        let uri = format!("/api/v1/items/1?{long_query}");
        let response = validation_app()
            .await
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ── request_size_validation_middleware ────────────────────────────────

    #[tokio::test]
    async fn allows_request_within_size_limit() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn(request_size_validation_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("content-length", "100")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn blocks_request_exceeding_size_limit() {
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn(request_size_validation_middleware));

        let over_limit = DEFAULT_REQUEST_BODY_MAX_BYTES + 1;
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("content-length", over_limit.to_string())
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }
}
