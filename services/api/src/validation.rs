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

/// Request validation middleware
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

/// Content-Type validation for POST/PUT requests
pub async fn content_type_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let method = request.method();
    let headers = request.headers();

    // Only validate POST, PUT, PATCH requests
    if matches!(
        method.as_str(),
        "POST" | "PUT" | "PATCH"
    ) {
        if let Some(content_type) = headers.get("content-type") {
            let ct = content_type.to_str().unwrap_or("");
            
            // Allow only JSON and form data
            if !ct.starts_with("application/json") 
                && !ct.starts_with("application/x-www-form-urlencoded")
                && !ct.starts_with("multipart/form-data") {
                return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
            }
        } else {
            // Require Content-Type header for mutation requests
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    Ok(next.run(request).await)
}

/// Request size validation
pub async fn request_size_validation_middleware(
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let headers = request.headers();

    // Check Content-Length header
    if let Some(content_length) = headers.get("content-length") {
        if let Ok(length_str) = content_length.to_str() {
            if let Ok(length) = length_str.parse::<usize>() {
                // Limit request body to 1MB
                if length > 1_048_576 {
                    return Err(StatusCode::PAYLOAD_TOO_LARGE);
                }
            }
        }
    }

    Ok(next.run(request).await)
}
