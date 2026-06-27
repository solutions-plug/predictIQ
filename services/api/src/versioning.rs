use axum::{
    extract::Request,
    http::{header, HeaderValue},
    middleware::Next,
    response::Response,
};

pub const CURRENT_VERSION: &str = "v1";
pub const SUPPORTED_VERSIONS: &[&str] = &["v1"];
/// Versions that are deprecated and will be removed on the scheduled sunset date.
pub const DEPRECATED_VERSIONS: &[&str] = &["v1"];

/// Injects the resolved API version into request extensions.
/// Reads `API-Version` header; defaults to current version.
#[derive(Clone, Debug)]
pub struct ApiVersion(pub String);

pub async fn versioning_middleware(mut req: Request, next: Next) -> Response {
    let version = req
        .headers()
        .get("API-Version")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_lowercase())
        .filter(|v| SUPPORTED_VERSIONS.contains(&v.as_str()))
        .unwrap_or_else(|| CURRENT_VERSION.to_string());

    if DEPRECATED_VERSIONS.contains(&version.as_str()) {
        tracing::warn!(
            version = %version,
            "Request used deprecated API version; clients should migrate before the sunset date"
        );
    }

    req.extensions_mut().insert(ApiVersion(version));
    next.run(req).await
}

/// Adds `Deprecation` and `Sunset` headers to responses for v1 routes per RFC 8594.
pub async fn v1_deprecation_middleware(req: Request, next: Next) -> Response {
    tracing::warn!(
        version = "v1",
        sunset = "Sat, 25 Apr 2026 00:00:00 GMT",
        "Deprecated API version v1 called; clients must migrate before sunset"
    );

    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    // RFC 8594: boolean "true" signals the resource is deprecated
    headers.insert(
        "Deprecation",
        HeaderValue::from_static("true"),
    );
    // Sunset date per RFC 8594: when v1 will be removed
    headers.insert(
        "Sunset",
        HeaderValue::from_static("Sat, 25 Apr 2026 00:00:00 GMT"),
    );
    headers.insert(
        header::LINK,
        HeaderValue::from_static(
            "</api/v1>; rel=\"deprecation\"; type=\"text/html\"",
        ),
    );
    response
}
