use tower_http::compression::predicate::{NotForContentType, Predicate};
use tower_http::compression::CompressionLayer;

fn should_compress_text_based(content_type: Option<&str>) -> bool {
    let Some(ct) = content_type else {
        // If we can't determine content type, avoid wasting CPU.
        return false;
    };

    // Remove common parameters like `charset=utf-8`.
    let ct = ct.split(';').next().unwrap_or(ct).trim();

    // Only compress text-ish payloads.
    // Note: application/json is explicitly included.
    ct == "application/json" || ct.starts_with("text/")
}

pub fn compression_layer() -> CompressionLayer {
    // Exclude already-compressed/binary formats to avoid CPU waste.
    // (This primarily protects against cases where `content_type` might be
    // missing/incorrect while still keeping the middleware safe.)
    let not_for_binary = NotForContentType::new(vec![
        "application/zip",
        "application/gzip",
        "application/x-gzip",
        "application/x-zip-compressed",
        "application/pdf",
        "image/jpeg",
        "image/png",
        "image/webp",
        "image/gif",
        "image/svg+xml",
        "audio/mpeg",
        "audio/mp4",
        "video/mp4",
        "application/octet-stream",
        "application/x-bzip2",
        "application/x-7z-compressed",
    ]);

    CompressionLayer::new()
        .gzip(true)
        .br(true)
        // Only apply compression to text-based responses.
        .compress_when(Predicate::from_fn(should_compress_text_based))
        .filter(not_for_binary)
}

