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
