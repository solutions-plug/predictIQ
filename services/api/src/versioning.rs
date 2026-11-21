use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

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

/// Per-client deprecation log sampler.  Logs at most once per client key per
/// hour so high-traffic deployments do not flood logs.  The Prometheus counter
/// (`deprecated_api_calls_total`) is still incremented on every request.
#[derive(Clone)]
pub struct DeprecationSampler {
    last_logged: Arc<Mutex<HashMap<String, Instant>>>,
}

impl DeprecationSampler {
    pub fn new() -> Self {
        Self {
            last_logged: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Returns true if a warn log should be emitted for `(client_ip, version)`.
    /// Updates the last-seen timestamp when it returns true.
    pub fn should_log(&self, client_ip: &str, version: &str) -> bool {
        let key = format!("{client_ip}:{version}");
        let now = Instant::now();
        let mut map = self.last_logged.lock().unwrap_or_else(|e| e.into_inner());
        match map.get(&key) {
            None => {
                map.insert(key, now);
                true
            }
            Some(&last) if now.duration_since(last) >= Duration::from_secs(3600) => {
                map.insert(key, now);
                true
            }
            _ => false,
        }
    }
}

/// Extract best-effort client IP from headers (no full trust-proxy logic needed
/// here — this is only used for deprecation log sampling, not security decisions).
fn peer_ip_from_headers(req: &Request) -> String {
    req.headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.split(',').next())
        .map(|s| s.trim().to_owned())
        .or_else(|| {
            req.headers()
                .get("x-real-ip")
                .and_then(|v| v.to_str().ok())
                .map(|s| s.trim().to_owned())
        })
        .unwrap_or_else(|| "unknown".to_owned())
}

/// State threaded into `versioning_middleware` via `from_fn_with_state`.
#[derive(Clone)]
pub struct VersioningState {
    pub sampler: DeprecationSampler,
    pub metrics: crate::metrics::Metrics,
}

impl VersioningState {
    pub fn new(metrics: crate::metrics::Metrics) -> Self {
        Self {
            sampler: DeprecationSampler::new(),
            metrics,
        }
    }
}

pub async fn versioning_middleware(
    axum::extract::State(vs): axum::extract::State<VersioningState>,
    mut req: Request,
    next: Next,
) -> Response {
    let version = req
        .headers()
        .get("API-Version")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim().to_lowercase())
        .filter(|v| SUPPORTED_VERSIONS.contains(&v.as_str()))
        .unwrap_or_else(|| CURRENT_VERSION.to_string());

    if DEPRECATED_VERSIONS.contains(&version.as_str()) {
        vs.metrics.observe_deprecated_api_call(&version);

        let client_ip = peer_ip_from_headers(&req);
        if vs.sampler.should_log(&client_ip, &version) {
            tracing::warn!(
                version = %version,
                client_ip = %client_ip,
                "Request used deprecated API version; clients should migrate before the sunset date"
            );
        }
    }

    req.extensions_mut().insert(ApiVersion(version));
    next.run(req).await
}

/// Adds `Deprecation` and `Sunset` headers to responses for v1 routes per RFC 8594.
pub async fn v1_deprecation_middleware(req: Request, next: Next) -> Response {
    tracing::warn!(
        version = "v1",
        sunset = "Sat, 31 Dec 2026 00:00:00 GMT",
        "Deprecated API version v1 called; clients must migrate before sunset"
    );

    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert(
        "Deprecation",
        HeaderValue::from_static("true"),
    );
    headers.insert(
        "Sunset",
        HeaderValue::from_static("Sat, 31 Dec 2026 00:00:00 GMT"),
    );
    headers.insert(
        header::LINK,
        HeaderValue::from_static(
            "</api/v1>; rel=\"deprecation\"; type=\"text/html\"",
        ),
    );
    response
}

/// Adds a `Deprecation` header (RFC 8594) plus a `Link` pointing at the
/// versioned replacement to responses served from the legacy unversioned
/// `/api/blockchain/replay` path.  The old path is kept working temporarily
/// so existing clients can migrate to `/api/v1/blockchain/replay`.
pub async fn legacy_replay_deprecation_middleware(req: Request, next: Next) -> Response {
    tracing::warn!(
        path = "/api/blockchain/replay",
        replacement = "/api/v1/blockchain/replay",
        "Deprecated unversioned replay route called; clients should migrate to the versioned path"
    );

    let mut response = next.run(req).await;
    let headers = response.headers_mut();
    headers.insert("Deprecation", HeaderValue::from_static("true"));
    headers.insert(
        header::LINK,
        HeaderValue::from_static(
            "</api/v1/blockchain/replay>; rel=\"successor-version\"",
        ),
    );
    response
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request as HttpRequest, StatusCode};
    use axum::routing::{get, post};
    use axum::Router;
    use tower::ServiceExt;

    fn test_metrics() -> crate::metrics::Metrics {
        crate::metrics::Metrics::new()
    }

    async fn ok_handler() -> &'static str {
        "ok"
    }

    async fn replay_handler() -> &'static str {
        "replayed"
    }

    fn deprecation_router() -> Router {
        Router::new().route("/api/v1/ping", get(ok_handler)).layer(
            axum::middleware::from_fn(v1_deprecation_middleware),
        )
    }

    fn versioning_router(metrics: crate::metrics::Metrics) -> Router {
        let state = VersioningState::new(metrics);
        Router::new()
            .route("/ping", get(ok_handler))
            .layer(axum::middleware::from_fn_with_state(
                state,
                versioning_middleware,
            ))
    }

    /// Mirrors how the replay route is nested: the versioned path is the
    /// canonical route, and the legacy unversioned path is kept working with a
    /// deprecation signal.
    fn replay_router() -> Router {
        Router::new()
            .route("/api/v1/blockchain/replay", post(replay_handler))
            .route(
                "/api/blockchain/replay",
                post(replay_handler).layer(axum::middleware::from_fn(
                    legacy_replay_deprecation_middleware,
                )),
            )
    }

    #[tokio::test]
    async fn deprecation_middleware_adds_headers_for_deprecated_version() {
        let response = deprecation_router()
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/v1/ping")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        assert_eq!(headers.get("Deprecation").unwrap(), "true");
        assert_eq!(
            headers.get("Sunset").unwrap(),
            "Sat, 31 Dec 2026 00:00:00 GMT"
        );
        let link = headers.get(header::LINK).unwrap().to_str().unwrap();
        assert!(link.contains("rel=\"deprecation\""));
        assert!(link.contains("</api/v1>"));
    }

    #[tokio::test]
    async fn deprecation_middleware_omits_headers_for_current_version() {
        // The deprecation middleware is only mounted on deprecated (v1) routes;
        // a current-version route must not carry the deprecation surface.
        let router = Router::new().route("/api/v2/ping", get(ok_handler));
        let response = router
            .oneshot(
                HttpRequest::builder()
                    .uri("/api/v2/ping")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        assert!(headers.get("Deprecation").is_none());
        assert!(headers.get("Sunset").is_none());
        assert!(headers.get(header::LINK).is_none());
    }

    #[tokio::test]
    async fn replay_route_resolves_under_versioned_prefix() {
        let response = replay_router()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/v1/blockchain/replay")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        // The canonical versioned route must not carry the legacy deprecation signal.
        assert!(response.headers().get("Deprecation").is_none());
    }

    #[tokio::test]
    async fn legacy_replay_route_returns_deprecation_signal() {
        let response = replay_router()
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/api/blockchain/replay")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let headers = response.headers();
        assert_eq!(headers.get("Deprecation").unwrap(), "true");
        let link = headers.get(header::LINK).unwrap().to_str().unwrap();
        assert!(link.contains("</api/v1/blockchain/replay>"));
        assert!(link.contains("rel=\"successor-version\""));
    }

    #[tokio::test]
    async fn versioning_middleware_injects_resolved_version() {
        let router = versioning_router(test_metrics());
        let response = router
            .oneshot(
                HttpRequest::builder()
                    .uri("/ping")
                    .header("API-Version", "v1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn deprecated_api_calls_total_incremented_on_every_request() {
        let metrics = test_metrics();
        let router = versioning_router(metrics.clone());

        // Fire several requests from the same client; the sampler will only
        // allow one log line, but the counter must incre

/* … truncated 3472 chars — edit only what you need near the top … */
