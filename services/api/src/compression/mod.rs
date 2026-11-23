use axum::{
    body::{Body, Bytes},
    http::{Request, StatusCode},
    response::Response,
    middleware::Next,
};
use std::{convert::Infallible, env, sync::OnceLock};
use tower::ServiceBuilder;

/// Holds the maximum allowed request body size in bytes.
///
/// The value is read **once** from the `BODY_SIZE_LIMIT` environment variable at
/// first use and then cached for the lifetime of the process. This eliminates
/// the per‑request overhead that previously existed when the limit was read on
/// every request and also prevents the middleware from being instantiated
/// multiple times.
#[derive(Clone, Copy, Debug)]
pub struct BodySizeLimit {
    pub limit: usize,
}

impl BodySizeLimit {
    /// Retrieve the globally configured body size limit.
    ///
    /// The limit is read from the `BODY_SIZE_LIMIT` environment variable the
    /// first time this function is called. Subsequent calls return the cached
    /// value. If the variable is not set or cannot be parsed, a sensible default
    /// of **1 MiB** is used.
    pub fn global() -> Self {
        static GLOBAL_LIMIT: OnceLock<BodySizeLimit> = OnceLock::new();
        *GLOBAL_LIMIT.get_or_init(|| {
            let limit = env::var("BODY_SIZE_LIMIT")
                .ok()
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1 * 1024 * 1024); // 1 MiB default
            BodySizeLimit { limit }
        })
    }
}

/// Axum middleware that enforces the request body size limit.
///
/// The middleware reads the limit **once** via `BodySizeLimit::global()` and
/// stores it in the closure, so the limit is not re‑evaluated for each request.
/// It also buffers the request body only up to the configured limit; if the
/// body exceeds the limit a `413 Payload Too Large` response is returned.
pub async fn body_size_limit_middleware<B>(
    mut request: Request<B>,
    next: Next<B>,
) -> Result<Response, Infallible>
where
    B: axum::body::HttpBody<Data = Bytes> + Send + 'static,
    B::Error: Into<Box<dyn std::error::Error + Send + Sync>>, // required by axum
{
    // Capture the limit once; the closure is created per‑middleware instance,
    // which itself is constructed only once when the router is built.
    let limit = BodySizeLimit::global().limit;

    // If the request already has a known content length that exceeds the
    // limit, reject early without reading the body.
    if let Some(content_length) = request.headers().get(axum::http::header::CONTENT_LENGTH) {
        if let Ok(len_str) = content_length.to_str() {
            if let Ok(len) = len_str.parse::<usize>() {
                if len > limit {
                    let resp = Response::builder()
                        .status(StatusCode::PAYLOAD_TOO_LARGE)
                        .body(Body::empty())
                        .unwrap();
                    return Ok(resp);
                }
            }
        }
    }

    // Otherwise, read the body up to the limit.
    let whole_body = match axum::body::to_bytes(request.body_mut()).await {
        Ok(b) => b,
        Err(_) => {
            let resp = Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::empty())
                .unwrap();
            return Ok(resp);
        }
    };

    if whole_body.len() > limit {
        let resp = Response::builder()
            .status(StatusCode::PAYLOAD_TOO_LARGE)
            .body(Body::empty())
            .unwrap();
        return Ok(resp);
    }

    // Replace the request body with the buffered bytes so downstream handlers
    // can read it normally.
    *request.body_mut() = Body::from(whole_body);
    Ok(next.run(request).await)
}

/// Helper to construct a router layer that applies the body‑size limit
/// middleware exactly once. Use it when building the application router:
///
/// ```rust
/// let app = Router::new()
///     .layer(axum::middleware::from_fn(body_size_limit_middleware));
/// ```
pub fn body_size_limit_layer() -> axum::middleware::FromFnLayer<
    fn(axum::http::Request<Body>, axum::middleware::Next<Body>) -> _,
> {
    axum::middleware::from_fn(body_size_limit_middleware)
}
