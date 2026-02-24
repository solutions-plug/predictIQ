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

/// Rate limiter configuration
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
pub struct RateLimiter {
    limits: Arc<RwLock<HashMap<String, RateLimitEntry>>>,
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

/// Extract client IP from request
pub fn extract_client_ip(headers: &HeaderMap, connect_info: Option<&ConnectInfo<std::net::SocketAddr>>) -> String {
    // Check X-Forwarded-For header (from proxy/load balancer)
    if let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        if let Some(ip) = forwarded_for.split(',').next() {
            let ip = ip.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    // Check X-Real-IP header
    if let Some(real_ip) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
        let ip = real_ip.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }

    // Fallback to connection info
    if let Some(conn_info) = connect_info {
        return conn_info.0.ip().to_string();
    }

    "unknown".to_string()
}

/// Global rate limiting middleware (100 req/min per IP)
pub async fn global_rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&headers, connect_info.as_ref());
    let config = RateLimitConfig::new(100, Duration::from_secs(60));

    if !limiter.check(&format!("global:{}", ip), &config).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Security headers middleware
pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Response {
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
    headers.insert(
        "x-frame-options",
        HeaderValue::from_static("DENY"),
    );

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
            .filter(|c| !c.is_control() || c.is_whitespace())
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
        self.valid_keys.iter().any(|k| k == key)
    }
}

/// API key authentication middleware
pub async fn api_key_middleware(
    State(auth): State<Arc<ApiKeyAuth>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let api_key = headers
        .get("x-api-key")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("");

    if !auth.verify(api_key) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(request).await)
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
        if let Ok(addr) = ip.parse::<IpAddr>() {
            return self.allowed_ips.contains(&addr);
        }
        false
    }
}

/// IP whitelist middleware
pub async fn ip_whitelist_middleware(
    State(whitelist): State<Arc<IpWhitelist>>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&headers, connect_info.as_ref());

    if !whitelist.is_allowed(&ip) {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(next.run(request).await)
}

/// Request signing verification for sensitive operations
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

    pub fn generate_signature(payload: &[u8], secret: &str) -> String {
        let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(payload);
        let result = mac.finalize();
        BASE64.encode(result.into_bytes())
    }
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
