use axum::{
    extract::Request,
    http::HeaderValue,
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

/// Maximum allowed header length for the correlation/correlation ID.
///
/// UUIDs in canonical string form are 36 bytes (e.g. `550e8400-e29b-41d4-a716-446655440000`).
pub const REQUEST_ID_MAX_LEN: usize = 64;

fn parse_valid_request_id(header_value: &str) -> Option<String> {
    if header_value.len() > REQUEST_ID_MAX_LEN {
        return None;
    }

    // Validate as UUID v4.
    let uuid = Uuid::parse_str(header_value).ok()?;
    if uuid.get_version_num() != 4 {
        return None;
    }

    Some(uuid.to_string())
}

/// Middleware that attaches a correlation ID to every request.
///
/// - Reads `X-Request-ID` from the incoming request if present and validates it as UUID v4.
///   Otherwise generates a new UUID v4.
/// - Records the ID as a `request_id` field on the current tracing span so
///   every log line emitted within the request carries it automatically.
/// - Echoes the ID back in the `X-Request-ID` response header.
pub async fn correlation_id_middleware(mut req: Request, next: Next) -> Response {
    let id = req
        .headers()
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .and_then(parse_valid_request_id)
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // Normalise: ensure the header is present on the request for downstream handlers.
    // (If we ever failed to create a HeaderValue, fall back to not inserting.)
    if let Ok(val) = HeaderValue::from_str(&id) {
        req.headers_mut().insert(REQUEST_ID_HEADER, val);
    }

    let span = tracing::Span::current();
    span.record("request_id", &id.as_str());

    let mut response = next.run(req).await;

    if let Ok(val) = HeaderValue::from_str(&id) {
        response.headers_mut().insert(REQUEST_ID_HEADER, val);
    }

    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_uuid_v4_is_accepted() {
        let header = "550e8400-e29b-41d4-a716-446655440000"; // version 4
        let parsed = parse_valid_request_id(header);
        assert_eq!(parsed.as_deref(), Some(header));
    }

    #[test]
    fn malformed_is_rejected_and_replaced() {
        assert!(parse_valid_request_id("not-a-uuid").is_none());
        assert!(parse_valid_request_id("550e8400-e29b").is_none());
    }

    #[test]
    fn uuid_non_v4_is_rejected() {
        // Version 1 UUID string example
        let header = "6ba7b810-9dad-11d1-80b4-00c04fd430c8";
        assert!(parse_valid_request_id(header).is_none());
    }

    #[test]
    fn too_long_is_rejected() {
        let long = format!("{}{}", "550e8400-e29b-41d4-a716-446655440000", "x".repeat(100));
        assert!(parse_valid_request_id(&long).is_none());
    }
}

