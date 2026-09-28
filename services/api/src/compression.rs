//! Response compression is limited to JSON and text payloads. SSE and responses
//! already carrying Content-Encoding are left untouched; known bodies below 32
//! bytes are also skipped to avoid spending CPU on negligible payloads.

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
    if headers.contains_key(header::CONTENT_ENCODING)
        || ct.eq_ignore_ascii_case("text/event-stream")
    {
        return false;
    }

    if headers
        .get(header::CONTENT_LENGTH)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .is_some_and(|length| length < 32)
    {
        return false;
    }

    ct.eq_ignore_ascii_case("application/json")
        || ct.get(..5).is_some_and(|prefix| prefix.eq_ignore_ascii_case("text/"))
}

pub fn compression_layer() -> CompressionLayer<CompressFn> {
    CompressionLayer::new()
        .gzip(true)
        .br(true)
        .compress_when(should_compress as CompressFn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, Response},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn response_for(body: &'static str) -> Response<Body> {
        let app = Router::new()
            .route(
                "/",
                get(move || async move {
                    Response::builder()
                        .header(header::CONTENT_TYPE, "application/json")
                        .header(header::CONTENT_LENGTH, body.len().to_string())
                        .body(Body::from(body))
                        .unwrap()
                }),
            )
            .layer(compression_layer());

        app.oneshot(
            Request::builder()
                .header(header::ACCEPT_ENCODING, "gzip")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn compresses_large_json_but_not_small_json() {
        let large = response_for(r#"{"payload":"this JSON body is large enough to compress"}"#).await;
        assert_eq!(
            large.headers().get(header::CONTENT_ENCODING).unwrap(),
            "gzip"
        );

        let small = response_for(r#"{"ok":true}"#).await;
        assert!(!small.headers().contains_key(header::CONTENT_ENCODING));
    }

    #[test]
    fn skips_sse_and_already_encoded_responses() {
        let mut headers = HeaderMap::new();
        headers.insert(header::CONTENT_TYPE, "text/event-stream".parse().unwrap());
        assert!(!should_compress(
            StatusCode::OK,
            Version::HTTP_11,
            &headers,
            &Extensions::new()
        ));

        headers.insert(header::CONTENT_TYPE, "application/json".parse().unwrap());
        headers.insert(header::CONTENT_ENCODING, "gzip".parse().unwrap());
        assert!(!should_compress(
            StatusCode::OK,
            Version::HTTP_11,
            &headers,
            &Extensions::new()
        ));
    }
}
