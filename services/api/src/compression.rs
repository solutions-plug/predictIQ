use axum::http::{header, Extensions, HeaderMap, StatusCode, Version};
use tower_http::compression::CompressionLayer;

type CompressFn = fn(StatusCode, Version, &HeaderMap, &Extensions) -> bool;

fn should_compress(
    _: StatusCode,
    _: Version,
    headers: &HeaderMap,
    _: &Extensions,
) -> bool {
    let ct = headers
        .get(header::CONTENT_TYPE)
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    let ct = ct.split(';').next().unwrap_or(ct).trim();
    ct == "application/json" || ct.starts_with("text/")
}

pub fn compression_layer() -> CompressionLayer<CompressFn> {
    CompressionLayer::new()
        .gzip(true)
        .br(true)
        .compress_when(should_compress as CompressFn)
}
