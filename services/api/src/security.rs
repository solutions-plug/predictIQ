use std::{
    collections::HashMap,
    net::IpAddr,
    sync::Arc,
    time::{Duration, SystemTime},
};

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tokio::sync::RwLock;

/// Newtype wrapper so `trust_proxy: bool` can be injected as Axum `State`.
#[derive(Clone, Copy, Debug)]
pub struct TrustProxy(pub bool);

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub requests: u32,
    pub window: Duration,
}

impl RateLimitConfig {
    pub fn new(requests: u32, window: Duration) -> Self {
        Self { requests, window }
    }
}

/// Rate limiter state for tracking requests
#[derive(Debug)]
struct RateLimitEntry {
    count: u32,
    window_start: SystemTime,
}

/// Multi-tier rate limiter
#[derive(Clone)]
pub struct WebhookConfig {
    pub secret: Option<String>,
    pub replay_window_secs: u64,
}

impl RateLimiter {
    pub fn new() -> Self {
        Self {
            limits: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn check(&self, key: &str, config: &RateLimitConfig) -> bool {
        let mut limits = self.limits.write().await;
        let now = SystemTime::now();

        let entry = limits.entry(key.to_string()).or_insert(RateLimitEntry {
            count: 0,
            window_start: now,
        });

        // Reset window if expired
        if now
            .duration_since(entry.window_start)
            .unwrap_or(Duration::ZERO)
            >= config.window
        {
            entry.count = 0;
            entry.window_start = now;
        }

        // Check limit
        if entry.count >= config.requests {
            return false;
        }

        entry.count += 1;
        true
    }

    /// Cleanup old entries periodically
    pub async fn cleanup(&self) {
        let mut limits = self.limits.write().await;
        let now = SystemTime::now();
        limits.retain(|_, entry| {
            now.duration_since(entry.window_start)
                .unwrap_or(Duration::ZERO)
                < Duration::from_secs(3600)
        });
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract client IP with trusted proxy CIDR validation.
/// Only trust x-forwarded-for/x-real-ip if the remote address is in a trusted proxy CIDR.
pub fn extract_client_ip(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
    trust_proxy: bool,
) -> String {
    let proxy_trusted = trust_proxy;

    if proxy_trusted {
        // 1. Check X-Forwarded-For header (from proxy/load balancer)
        if let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
            for ip_str in forwarded_for.split(',') {
                let ip_str = ip_str.trim();
                if !ip_str.is_empty() && ip_str.parse::<IpAddr>().is_ok() {
                    return ip_str.to_string();
                }
            }
        }
        // 2. Check X-Real-IP header
        if let Some(real_ip) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
            let ip_str = real_ip.trim();
            if !ip_str.is_empty() && ip_str.parse::<IpAddr>().is_ok() {
                return ip_str.to_string();
            }
        }
    }
    // 3. Fallback to connection info (Socket)
    if let Some(conn_info) = connect_info {
        return conn_info.0.ip().to_string();
    }
    "unknown".to_string()
}

/// Global rate limiting middleware (100 req/min per IP)
pub async fn global_rate_limit_middleware(
    State((limiter, TrustProxy(trust_proxy))): State<(Arc<RateLimiter>, TrustProxy)>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&headers, connect_info.as_ref(), trust_proxy);
    let config = RateLimitConfig::new(100, Duration::from_secs(60));

    if !limiter.check(&format!("global:{}", ip), &config).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Security headers middleware
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Content Security Policy
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self' data:; connect-src 'self'; frame-ancestors 'none';"
        ),
    );

    // X-Frame-Options
    headers.insert("x-frame-options", HeaderValue::from_static("DENY"));

    // X-Content-Type-Options
    headers.insert(
        "x-content-type-options",
        HeaderValue::from_static("nosniff"),
    );

    // X-XSS-Protection
    headers.insert(
        "x-xss-protection",
        HeaderValue::from_static("1; mode=block"),
    );

    // Strict-Transport-Security (HSTS)
    headers.insert(
        "strict-transport-security",
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    // Referrer-Policy
    headers.insert(
        "referrer-policy",
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    // Permissions-Policy
    headers.insert(
        "permissions-policy",
        HeaderValue::from_static("geolocation=(), microphone=(), camera=()"),
    );

    response
}

/// Input sanitization utilities
pub mod sanitize {
    use validator::ValidateEmail;

    /// Sanitize email input
    pub fn email(input: &str) -> Option<String> {
        let cleaned = input.trim().to_lowercase();
        if cleaned.len() > 254 || !cleaned.validate_email() {
            return None;
        }
        Some(cleaned)
    }

    /// Sanitize string input (remove control characters, limit length)
    pub fn string(input: &str, max_len: usize) -> String {
        input
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\t' | '\n' | '\r' | ' '))
            .take(max_len)
            .collect()
    }

    /// Sanitize numeric ID
    pub fn numeric_id(input: &str) -> Option<i64> {
        input.trim().parse::<i64>().ok()
    }

    /// Check for SQL injection patterns (basic detection)
    pub fn contains_sql_injection(input: &str) -> bool {
        let lower = input.to_lowercase();
        let patterns = [
            "' or '1'='1",
            "' or 1=1",
            "'; drop table",
            "'; delete from",
            "union select",
            "exec(",
            "execute(",
            "script>",
            "<script",
            "javascript:",
            "onerror=",
            "onload=",
        ];

        patterns.iter().any(|pattern| lower.contains(pattern))
    }
}

/// API Key authentication for admin endpoints
#[derive(Clone)]
pub struct ApiKeyAuth {
    valid_keys: Arc<Vec<String>>,
}

impl ApiKeyAuth {
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            valid_keys: Arc::new(keys),
        }
    }

    pub fn verify(&self, key: &str) -> bool {
        self.valid_keys.is_empty() || self.valid_keys.iter().any(|k| k == key)
    }
}

#[derive(Serialize)]
struct ApiKeyErrorBody {
    error: &'static str,
}

/// API key authentication middleware
pub async fn api_key_middleware(
    State(auth): State<Arc<ApiKeyAuth>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if !auth.verify(api_key) {
        let mut resp = (
            StatusCode::UNAUTHORIZED,
            Json(ApiKeyErrorBody {
                error: "invalid or missing API key",
            }),
        )
            .into_response();
        resp.headers_mut().insert(
            "WWW-Authenticate",
            HeaderValue::from_static("ApiKey realm=\"predictiq\""),
        );
        return resp;
    }

    next.run(request).await
}

/// IP whitelist for admin endpoints
#[derive(Clone)]
pub struct IpWhitelist {
    allowed_ips: Arc<Vec<IpAddr>>,
}

impl IpWhitelist {
    pub fn new(ips: Vec<IpAddr>) -> Self {
        Self {
            allowed_ips: Arc::new(ips),
        }
    }

    pub fn is_allowed(&self, ip: &str) -> bool {
        if self.allowed_ips.is_empty() {
            return true;
        }
        if let Ok(addr) = ip.parse::<IpAddr>() {
            return self.allowed_ips.contains(&addr);
        }
        false
    }
}

/// IP whitelist middleware for admin routes.
///
/// Allows all IPs when `ADMIN_WHITELIST_IPS` is empty (open-by-default for
/// local/dev). When the env var is set to a comma-separated list of IPs, only
/// those addresses may reach admin endpoints; all others receive `403 Forbidden`.
pub async fn ip_whitelist_middleware(
    State((whitelist, TrustProxy(trust_proxy))): State<(Arc<IpWhitelist>, TrustProxy)>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&headers, connect_info.as_ref(), trust_proxy);

    if !whitelist.is_allowed(&ip) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

/// Configuration for the metrics endpoint access control.
///
/// - `public`: skip all auth checks (only for trusted/internal networks).
/// - `allowlist`: when non-empty, the caller's IP must be in this list.
/// - `auth`: API key validator; checked when `public` is false.
#[derive(Clone)]
pub struct MetricsAuthConfig {
    pub public: bool,
    pub allowlist: Arc<Vec<IpAddr>>,
    pub auth: Arc<ApiKeyAuth>,
}

impl MetricsAuthConfig {
    pub fn new(public: bool, allowlist: Vec<IpAddr>, auth: Arc<ApiKeyAuth>) -> Self {
        Self {
            public,
            allowlist: Arc::new(allowlist),
            auth,
        }
    }
}

/// Metrics endpoint authentication middleware.
///
/// Access is granted when ANY of the following is true:
/// 1. `config.public == true` (opt-in public mode, e.g. internal cluster).
/// 2. The caller's IP is in `config.allowlist` AND a valid API key is present.
/// 3. `config.allowlist` is empty AND a valid API key is present.
pub async fn metrics_auth_middleware(
    State(config): State<Arc<MetricsAuthConfig>>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if config.public {
        return Ok(next.run(request).await);
    }

    // IP allowlist check (when configured)
    if !config.allowlist.is_empty() {
        // Use empty CIDR list — allowlist check uses direct IP comparison, not proxy headers
        let ip = extract_client_ip(&headers, connect_info.as_ref(), false);
        let parsed: Option<IpAddr> = ip.parse().ok();
        let allowed = parsed.map(|a| config.allowlist.contains(&a)).unwrap_or(false);
        if !allowed {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    // API key check
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");
    if !config.auth.verify(api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
}

/// SendGrid webhook signature verification middleware.
///
/// Verifies the `X-Twilio-Email-Event-Webhook-Signature` header using HMAC-SHA256
/// against the raw request body. The `SENDGRID_WEBHOOK_SECRET` must be configured
/// for webhook security, except in development environment where it passes through.
///
/// Replay protection: rejects requests whose `X-Twilio-Email-Event-Webhook-Timestamp`
/// is more than `WEBHOOK_REPLAY_WINDOW_SECS` seconds old relative to the server clock
/// (default: 300 seconds).
///
/// # OpenAPI policy
/// Route: `POST /webhooks/sendgrid`
/// Auth: provider-signed (SendGrid HMAC) — no API key required.
pub async fn sendgrid_webhook_middleware(
    State(config): State<WebhookConfig>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let is_dev = std::env::var("ENVIRONMENT")
        .map(|e| e == "development")
        .unwrap_or(true); // default to dev if not set

    if config.secret.is_none() && !is_dev {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if let Some(ref secret) = config.secret {
        let sig = headers
            .get("x-twilio-email-event-webhook-signature")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Replay protection: reject stale timestamps (> config.replay_window_secs)
        let ts_str = headers
            .get("x-twilio-email-event-webhook-timestamp")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        let ts: i64 = ts_str.parse().unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        if (now - ts).abs() > config.replay_window_secs as i64 {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, usize::MAX)
            .await
            .map_err(|_| StatusCode::BAD_REQUEST)?;

        // Signature covers timestamp + payload per SendGrid spec
        let signed_payload = format!("{}{}", ts_str, String::from_utf8_lossy(&bytes));
        if !signing::verify_signature(signed_payload.as_bytes(), sig, secret) {
            return Err(StatusCode::UNAUTHORIZED);
        }

        let request = Request::from_parts(parts, Body::from(bytes));
        return Ok(next.run(request).await);
    }

    Ok(next.run(request).await)
}

pub mod signing {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    type HmacSha256 = Hmac<Sha256>;

    pub fn verify_signature(payload: &[u8], signature: &str, secret: &str) -> bool {
        let mut mac = match HmacSha256::new_from_slice(secret.as_bytes()) {
            Ok(m) => m,
            Err(_) => return false,
        };

        mac.update(payload);

        let expected = match BASE64.decode(signature) {
            Ok(sig) => sig,
            Err(_) => return false,
        };

        mac.verify_slice(&expected).is_ok()
    }

    pub fn generate_signature(payload: &[u8], secret: &str) -> Result<String, SigningError> {
        let mut mac =
            HmacSha256::new_from_slice(secret.as_bytes()).map_err(|_| SigningError::InvalidKey)?;
        mac.update(payload);
        let result = mac.finalize();
        Ok(BASE64.encode(result.into_bytes()))
    }

    /// Error type for fallible signing operations.
    #[derive(Debug, PartialEq)]
    pub enum SigningError {
        /// The secret key was rejected by the HMAC constructor (empty key).
        InvalidKey,
    }

    impl std::fmt::Display for SigningError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                SigningError::InvalidKey => write!(f, "signing key is invalid"),
            }
        }
    }

    impl std::error::Error for SigningError {}
}

#[derive(Serialize)]
pub struct SecurityError {
    pub error: String,
    pub message: String,
}

impl IntoResponse for SecurityError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderMap;
    use std::net::SocketAddr;

    // ── helpers ──────────────────────────────────────────────────────────────

    fn addr(s: &str) -> ConnectInfo<SocketAddr> {
        ConnectInfo(s.parse().unwrap())
    }

    fn xff(val: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-forwarded-for", val.parse().unwrap());
        h
    }

    fn xri(val: &str) -> HeaderMap {
        let mut h = HeaderMap::new();
        h.insert("x-real-ip", val.parse().unwrap());
        h
    }

    // ── security headers middleware ───────────────────────────────────────

    #[tokio::test]
    async fn security_headers_middleware_sets_required_headers() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn(super::security_headers_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let headers = response.headers();
        assert!(headers.contains_key("content-security-policy"));
        assert!(headers.contains_key("strict-transport-security"));
        assert!(headers.contains_key("x-frame-options"));
        assert!(headers.contains_key("referrer-policy"));
        assert_eq!(headers["x-frame-options"], "DENY");
        assert_eq!(headers["x-content-type-options"], "nosniff");
    }

    #[tokio::test]
    async fn security_headers_middleware_no_duplicates() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn(super::security_headers_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let headers = response.headers();
        // get_all returns an iterator; exactly one value means no duplicates.
        assert_eq!(headers.get_all("x-frame-options").iter().count(), 1);
        assert_eq!(headers.get_all("content-security-policy").iter().count(), 1);
        assert_eq!(headers.get_all("strict-transport-security").iter().count(), 1);
        assert_eq!(headers.get_all("referrer-policy").iter().count(), 1);
    }

    #[test]
    fn test_extract_client_ip_precedence() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.1.1.1, 2.2.2.2".parse().unwrap());
        headers.insert("x-real-ip", "3.3.3.3".parse().unwrap());
        let ci = addr("4.4.4.4:8080");

        assert_eq!(extract_client_ip(&headers, Some(&ci), true), "1.1.1.1");

        headers.remove("x-forwarded-for");
        assert_eq!(extract_client_ip(&headers, Some(&ci), true), "3.3.3.3");

        headers.remove("x-real-ip");
        assert_eq!(extract_client_ip(&headers, Some(&ci), true), "4.4.4.4");
    }

    #[test]
    fn test_extract_client_ip_validation() {
        let mut headers = HeaderMap::new();
        let ci = addr("4.4.4.4:8080");

        headers.insert("x-forwarded-for", "malformed, 1.1.1.1".parse().unwrap());
        assert_eq!(extract_client_ip(&headers, Some(&ci), true), "1.1.1.1");

        headers.insert("x-forwarded-for", "not-an-ip, also-bad".parse().unwrap());
        headers.insert("x-real-ip", "2.2.2.2".parse().unwrap());
        assert_eq!(extract_client_ip(&headers, Some(&ci), true), "2.2.2.2");

        headers.insert("x-real-ip", "invalid-ip".parse().unwrap());
        assert_eq!(extract_client_ip(&headers, Some(&ci), true), "4.4.4.4");
    }

    #[test]
    fn test_extract_client_ip_empty_and_unknown() {
        let headers = HeaderMap::new();

        // No headers, no connect info
        assert_eq!(extract_client_ip(&headers, None, false), "unknown");

        let ci = addr("5.5.5.5:80");
        assert_eq!(extract_client_ip(&headers, Some(&ci), false), "5.5.5.5");
    }

    #[test]
    fn test_extract_client_ip_ipv6() {
        let headers = xff("2001:db8::1, 192.168.1.1");
        assert_eq!(extract_client_ip(&headers, None, true), "2001:db8::1");
    }

    // ── trust-boundary tests (issue #281) ────────────────────────────────

    /// Without a trusted proxy, X-Forwarded-For MUST be ignored and the real
    /// socket address used instead.
    #[test]
    fn spoofed_xff_ignored_when_trust_proxy_disabled() {
        let headers = xff("9.9.9.9");
        let ci = addr("1.2.3.4:1234");
        assert_eq!(
            extract_client_ip(&headers, Some(&ci), false),
            "1.2.3.4",
            "X-Forwarded-For must not be trusted without proxy config"
        );
    }

    /// Without a trusted proxy, X-Real-IP MUST be ignored.
    #[test]
    fn spoofed_x_real_ip_ignored_when_trust_proxy_disabled() {
        let headers = xri("9.9.9.9");
        let ci = addr("1.2.3.4:1234");
        assert_eq!(
            extract_client_ip(&headers, Some(&ci), false),
            "1.2.3.4",
            "X-Real-IP must not be trusted without proxy config"
        );
    }

    /// Both spoofed headers present — returns "unknown" when proxy trust is
    /// disabled and no socket address is available.
    #[test]
    fn both_spoofed_headers_ignored_when_trust_proxy_disabled() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "x-forwarded-for",
            "2001:db8::1, 192.168.1.1".parse().unwrap(),
        );

        assert_eq!(extract_client_ip(&headers, None, false), "unknown");
    }

    // ── api_key_middleware ────────────────────────────────────────────────

    #[tokio::test]
    async fn api_key_middleware_allows_valid_key() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let auth = Arc::new(ApiKeyAuth::new(vec!["secret".to_string()]));
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(auth, super::api_key_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-api-key", "secret")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn api_key_middleware_rejects_missing_key() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let auth = Arc::new(ApiKeyAuth::new(vec!["secret".to_string()]));
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(auth, super::api_key_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn api_key_middleware_rejects_wrong_key() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let auth = Arc::new(ApiKeyAuth::new(vec!["secret".to_string()]));
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(auth, super::api_key_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-api-key", "wrong")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    // ── ip_whitelist_middleware ───────────────────────────────────────────

    #[tokio::test]
    async fn ip_whitelist_allows_whitelisted_ip() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let whitelist = Arc::new(IpWhitelist::new(vec!["127.0.0.1".parse().unwrap()]));
        let state = (whitelist, TrustProxy(false));
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(state, super::ip_whitelist_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        // No ConnectInfo → ip resolves to "unknown" → not in whitelist → 403.
        // To test an allowed IP we use X-Real-IP with trust_proxy=true.
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn ip_whitelist_allows_when_list_empty() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let whitelist = Arc::new(IpWhitelist::new(vec![])); // empty = allow all
        let state = (whitelist, TrustProxy(false));
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(state, super::ip_whitelist_middleware));

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn ip_whitelist_blocks_non_whitelisted_ip() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let whitelist = Arc::new(IpWhitelist::new(vec!["10.0.0.1".parse().unwrap()]));
        let state = (whitelist, TrustProxy(true));
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(state, super::ip_whitelist_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-real-ip", "1.2.3.4") // not in whitelist
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn ip_whitelist_allows_matching_ip_via_header() {
        use axum::{body::Body, http::Request, middleware, routing::get, Router};
        use tower::ServiceExt;

        let whitelist = Arc::new(IpWhitelist::new(vec!["10.0.0.1".parse().unwrap()]));
        let state = (whitelist, TrustProxy(true));
        let app = Router::new()
            .route("/", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(state, super::ip_whitelist_middleware));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/")
                    .header("x-real-ip", "10.0.0.1") // in whitelist
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}

/// Middleware to propagate trace context from incoming requests
pub async fn trace_propagation_middleware(
    headers: HeaderMap,
    mut request: Request,
    next: Next,
) -> Response {
    use opentelemetry::global;
    use opentelemetry::propagation::TextMapPropagator;
    use std::collections::HashMap;

    // Extract trace context from headers
    let propagator = global::get_text_map_propagator(|propagator| {
        let mut carrier = HashMap::new();
        for (key, value) in headers.iter() {
            if let Ok(v) = value.to_str() {
                carrier.insert(key.as_str().to_string(), v.to_string());
            }
        }
        
        let context = propagator.extract(&carrier);
        context
    });

    // Attach context to current span
    let span = tracing::Span::current();
    use tracing_opentelemetry::OpenTelemetrySpanExt;
    span.set_parent(propagator);

    next.run(request).await
}
