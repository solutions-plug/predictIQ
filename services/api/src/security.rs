use std::{
    net::IpAddr,
    sync::Arc,
};

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use ipnet::IpNet;
use serde::Serialize;
use subtle::ConstantTimeEq;

/// Newtype wrapper so `trust_proxy: bool` can be injected as Axum `State`.
#[derive(Clone, Copy, Debug)]
pub struct TrustProxy(pub bool);

/// Webhook signature verification config
#[derive(Clone)]
pub struct WebhookConfig {
    pub secret: Option<String>,
    pub replay_window_secs: u64,
}

/// Extract client IP with trusted proxy CIDR validation.
///
/// Headers (`x-forwarded-for`, `x-real-ip`) are only trusted when:
/// - `trusted_cidrs` is non-empty AND the connecting socket address falls
///   within one of the configured CIDRs, OR
/// - `trusted_cidrs` is empty AND `trust_proxy` is `true` (legacy mode).
///
/// Pass `trusted_cidrs = &[]` and `trust_proxy = false` to always use the
/// raw socket address (safe for direct-to-internet deployments).
pub fn extract_client_ip(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
    trust_proxy: bool,
) -> String {
    extract_client_ip_cidrs(headers, connect_info, trust_proxy, &[])
}

/// CIDR-aware variant. When `trusted_cidrs` is non-empty the connecting IP
/// must be contained in one of the CIDRs before proxy headers are trusted.
/// When `trusted_cidrs` is empty, falls back to the `trust_proxy` boolean.
///
/// # IPv6 handling
/// Each candidate IP string from a proxy header is parsed via
/// `str::parse::<IpAddr>()`, which handles both IPv4 and IPv6.  On parse
/// failure a `WARN`-level log is emitted and that candidate is skipped;
/// processing continues with the next value rather than panicking or silently
/// using a potentially malicious string as a rate-limit key.
///
/// IPv6 addresses that parse successfully are stored in their **canonical**
/// (compressed, lowercase) form produced by `IpAddr::to_string()`, e.g.
/// `"2001:db8::1"` rather than the zero-expanded form sent by some proxies.
/// This guarantees that the same logical address always maps to the same
/// rate-limit bucket key.
pub fn extract_client_ip_cidrs(
    headers: &HeaderMap,
    connect_info: Option<&ConnectInfo<std::net::SocketAddr>>,
    trust_proxy: bool,
    trusted_cidrs: &[IpNet],
) -> String {
    let proxy_trusted = if !trusted_cidrs.is_empty() {
        // Only trust headers if the connecting IP is in a trusted CIDR.
        connect_info
            .and_then(|ci| ci.0.ip().to_string().parse::<IpAddr>().ok())
            .map(|connecting_ip| trusted_cidrs.iter().any(|cidr| cidr.contains(&connecting_ip)))
            .unwrap_or(false)
    } else {
        trust_proxy
    };

    if proxy_trusted {
        // 1. Check X-Forwarded-For header (may contain a comma-separated list).
        //    Iterate from left to right; return the first well-formed address.
        //    Malformed entries are logged as warnings and skipped.
        if let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
            for raw in forwarded_for.split(',') {
                let candidate = raw.trim();
                if candidate.is_empty() {
                    continue;
                }
                match candidate.parse::<IpAddr>() {
                    Ok(addr) => {
                        // Return canonical form — critical for IPv6 rate-limit keys.
                        return addr.to_string();
                    }
                    Err(_) => {
                        tracing::warn!(
                            header = "x-forwarded-for",
                            value = candidate,
                            "IP parse failure in proxy header; skipping candidate"
                        );
                    }
                }
            }
        }

        // 2. Check X-Real-IP header (single value expected).
        if let Some(real_ip) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
            let candidate = real_ip.trim();
            if !candidate.is_empty() {
                match candidate.parse::<IpAddr>() {
                    Ok(addr) => return addr.to_string(),
                    Err(_) => {
                        tracing::warn!(
                            header = "x-real-ip",
                            value = candidate,
                            "IP parse failure in proxy header; falling back to socket address"
                        );
                    }
                }
            }
        }
    }

    // 3. Fallback to the raw TCP socket address — always well-formed.
    if let Some(conn_info) = connect_info {
        return conn_info.0.ip().to_string();
    }

    "unknown".to_string()
}

/// Security headers middleware
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();

    // Content Security Policy
    // The API serves JSON only — a "null" CSP prevents browsers from rendering
    // API responses as HTML pages, blocking any injected script execution.
    headers.insert(
        "content-security-policy",
        HeaderValue::from_static(
            "default-src 'none'; script-src 'none'; style-src 'none'; img-src 'none'; font-src 'none'; connect-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'; object-src 'none';"
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
        HeaderValue::from_static("max-age=31536000; includeSubDomains; preload"),
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

/// Newtype so `require_https: bool` can be injected as Axum `State`.
#[derive(Clone, Copy, Debug)]
pub struct RequireHttps(pub bool);

/// HTTP → HTTPS redirect middleware.
///
/// When `RequireHttps(true)` is set and the incoming request arrives over plain
/// HTTP (detected via the `X-Forwarded-Proto: http` header set by the ALB),
/// the middleware issues a `301 Moved Permanently` redirect to the HTTPS
/// equivalent of the same URL.
///
/// Requests that already carry `X-Forwarded-Proto: https` or have no proto
/// header (direct connections, health checks from the VPC) are passed through
/// unchanged.
pub async fn https_redirect_middleware(
    State(RequireHttps(require)): State<RequireHttps>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    if require {
        let proto = headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");

        if proto == "http" {
            // Build the HTTPS redirect URL from the Host header + request URI.
            let host = headers
                .get("host")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let path_and_query = request
                .uri()
                .path_and_query()
                .map(|pq| pq.as_str())
                .unwrap_or("/");
            let redirect_url = format!("https://{}{}", host, path_and_query);

            let mut response = axum::response::Response::new(axum::body::Body::empty());
            *response.status_mut() = StatusCode::MOVED_PERMANENTLY;
            if let Ok(loc) = HeaderValue::from_str(&redirect_url) {
                response.headers_mut().insert("location", loc);
            }
            return response;
        }
    }

    next.run(request).await
}

/// API Key authentication for admin endpoints.
///
/// Supports two key stores that are checked in order:
/// 1. Static env-var keys (`API_KEYS`).
/// 2. Database-backed keys (table `api_keys`) when a `db` handle is provided.
///
/// This dual-store design allows zero-downtime migration from static env-var
/// keys to fully DB-managed keys with rotation support (issue #892).
#[derive(Clone)]
pub struct ApiKeyAuth {
    valid_keys: Arc<Vec<String>>,
    db: Option<Arc<crate::db::Database>>,
}

impl ApiKeyAuth {
    /// Create an auth instance backed only by static env-var keys.
    pub fn new(keys: Vec<String>) -> Self {
        Self {
            valid_keys: Arc::new(keys),
            db: None,
        }
    }

    /// Create an auth instance backed by both static keys and a database.
    pub fn new_with_db(keys: Vec<String>, db: Arc<crate::db::Database>) -> Self {
        Self {
            valid_keys: Arc::new(keys),
            db: Some(db),
        }
    }

    /// Synchronous check against static env-var keys only.
    pub fn verify(&self, key: &str) -> bool {
        use subtle::ConstantTimeEq;
        !self.valid_keys.is_empty()
            && self.valid_keys
                .iter()
                .map(|k| k.as_bytes().ct_eq(key.as_bytes()))
                .any(|eq| eq)
    }

    /// Asynchronous check that consults both static keys and the database.
    ///
    /// The static-key check is always tried first (fast path, no I/O).
    /// If that fails and a database handle is available, the key's SHA-256
    /// hash is looked up in the `api_keys` table.  Keys that are revoked or
    /// past their `expires_at` overlap window are rejected by the query.
    pub async fn verify_async(&self, key: &str) -> bool {
        // Fast path: static env-var keys.
        if self.verify(key) {
            return true;
        }

        // Slow path: database-backed keys.
        if let Some(ref db) = self.db {
            let hash = hash_api_key(key);
            match db.api_key_validate(&hash).await {
                Ok(Some(record)) => {
                    // Defence in depth: even if the query ever stops filtering
                    // revoked rows, never authenticate a revoked key.
                    if record.revoked_at.is_some() {
                        tracing::warn!(
                            key_id = record.id,
                            "rejected authentication attempt with revoked API key"
                        );
                        return false;
                    }
                    return true;
                }
                Ok(None) => {}
                Err(e) => {
                    tracing::warn!(error = %e, "api_key_validate db error");
                }
            }
        }

        false
    }

    /// Revoke a database-backed API key by its id.
    ///
    /// Sets `revoked_at` on the matching `api_keys` row so that subsequent
    /// authentication attempts with the key are rejected by
    /// [`ApiKeyAuth::verify_async`].  Revocation is idempotent: revoking an
    /// already-revoked key leaves the original `revoked_at` timestamp intact.
    ///
    /// Returns `Ok(true)` when a live key was revoked, `Ok(false)` when no
    /// matching (unrevoked) key exists, and `Err` on database failure.
    pub async fn revoke(&self, key_id: i64) -> Result<bool, crate::db::DbError> {
        let Some(ref db) = self.db else {
            return Ok(false);
        };
        db.api_key_revoke(key_id).await
    }
}

/// Hash an API key for storage/lookup using the same SHA-256 scheme as the
/// `api_keys` table.  Kept in one place so lookup and revocation agree.
pub fn hash_api_key(key: &str) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(key.as_bytes()))
}

#[derive(Serialize)]
struct ApiKeyErrorBody {
    error: &'static str,
}

/// API key authentication middleware.
///
/// Accepts keys validated by [`ApiKeyAuth::verify_async`], which checks both
/// the static `API_KEYS` env-var list and the database-backed `api_keys` table.
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

    if !auth.verify_async(api_key).await {
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
/// Verifies the `X-Twilio-Email-Event-Webhook-Signature` header as an ECDSA (P-256)
/// signature over `timestamp + body`. `SENDGRID_WEBHOOK_SECRET` holds SendGrid's
/// base64 DER public key (HMAC-SHA256 is accepted only outside production, for
/// local testing). It must be configured
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
    State((config, app_state)): State<(WebhookConfig, Arc<crate::AppState>)>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Use the canonical Config-derived production flag instead of reading
    // ENVIRONMENT directly. This ensures a single source of truth for
    // production-only safety checks (issue #1106).
    let is_dev = !app_state.config.is_production();

    if config.secret.is_none() && !is_dev {
        return Err(StatusCode::UNAUTHORIZED);
    }

    if let Some(ref secret) = config.secret {
        let sig = headers
            .get("x-twilio-email-event-webhook-signature")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");

        // Replay protection: reject stale AND future-dated timestamps.
        //
        // The previous check used .abs() which accepted future-dated timestamps
        // within the replay window. An attacker could pre-sign a request with
        // timestamp = now + window - 1 and replay it for up to 2 * window seconds.
        // The fix rejects any timestamp that is in the future at all, and any that
        // is more than replay_window_secs old.
        let ts_str = headers
            .get("x-twilio-email-event-webhook-timestamp")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("");
        let ts: i64 = ts_str.parse().unwrap_or(0);
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);
        let age_secs = now - ts;
        if age_secs < 0 || age_secs > config.replay_window_secs as i64 {
            tracing::warn!(
                ts,
                now,
                age_secs,
                "sendgrid webhook rejected: timestamp out of bounds"
            );
            return Err(StatusCode::UNAUTHORIZED);
        }

        let (parts, body) = request.into_parts();
        let bytes = axum::body::to_bytes(body, crate::email::webhook::MAX_WEBHOOK_PAYLOAD_BYTES)
            .await
            .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;

        // Signature covers timestamp + payload per SendGrid spec.
        // Build the signed payload as raw bytes (no lossy UTF-8 conversion) so
        // binary payloads are verified exactly as received.
        let mut signed_payload = Vec::with_capacity(ts_str.len() + bytes.len());
        signed_payload.extend_from_slice(ts_str.as_bytes());
        signed_payload.extend_from_slice(&bytes);
        if !signing::verify_sendgrid_signature(&signed_payload, sig, secret, is_dev) {
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
    use std::time::SystemTime;

    type HmacSha256 = Hmac<Sha256>;

    /// Verify a SendGrid Signed Event Webhook signature (base64 DER ECDSA P-256
    /// over `timestamp + payload`) against the base64 DER public key `key`.
    pub fn verify_ecdsa_signature(payload: &[u8], signature: &str, key: &str) -> bool {
        use p256::ecdsa::{signature::Verifier, Signature, VerifyingKey};
        use p256::pkcs8::DecodePublicKey;

        let Ok(key_der) = BASE64.decode(key.trim()) else { return false };
        let Ok(vk) = VerifyingKey::from_public_key_der(&key_der) else { return false };
        let Ok(sig_der) = BASE64.decode(signature.trim()) else { return false };
        let Ok(sig) = Signature::from_der(&sig_der) else { return false };
        vk.verify(payload, &sig).is_ok()
    }

    /// ECDSA verification; when `allow_hmac` (non-production only) and `key`
    /// is not a valid public key, fall back to the legacy HMAC mode.
    pub fn verify_sendgrid_signature(payload: &[u8], signature: &str, key: &str, allow_hmac: bool) -> bool {
        use p256::pkcs8::DecodePublicKey;
        let is_public_key = BASE64
            .decode(key.trim())
            .ok()
            .and_then(|d| p256::ecdsa::VerifyingKey::from_public_key_der(&d).ok())
            .is_some();
        if is_public_key {
            verify_ecdsa_signature(payload, signature, key)
        } else {
            allow_hmac && verify_signature(payload, signature, key)
        }
    }

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

    /// Verify a signature against the current key, or the previous key if provided and
    /// the token is within the grace period.
    ///
    /// # Arguments
    /// * `payload` - The signed payload
    /// * `signature` - The base64-encoded HMAC signature
    /// * `current_key` - The current HMAC secret key
    /// * `previous_key` - Optional previous HMAC secret key for rotation
    /// * `token_timestamp_secs` - The timestamp when the token was created (Unix seconds)
    /// * `grace_period_secs` - Grace period in seconds for accepting tokens signed with the previous key
    ///
    /// Returns true if the signature is valid against either key (within grace period for previous key)
    pub fn verify_signature_with_rotation(
        payload: &[u8],
        signature: &str,
        current_key: &str,
        previous_key: Option<&str>,
        token_timestamp_secs: i64,
        grace_period_secs: u64,
    ) -> bool {
        // Always try the current key first
        if verify_signature(payload, signature, current_key) {
            return true;
        }

        // If no previous key, we're done
        let Some(prev_key) = previous_key else {
            return false;
        };

        // Try the previous key only if within the grace period
        if !verify_signature(payload, signature, prev_key) {
            return false;
        }

        // Check if token is within grace period
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let age_secs = now - token_timestamp_secs;
        age_secs >= 0 && (age_secs as u64) <= grace_period_secs
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
    #[test]
    fn sendgrid_ecdsa_signature_roundtrip() {
        use base64::{engine::general_purpose::STANDARD as B64, Engine};
        use p256::ecdsa::{signature::Signer, Signature, SigningKey};
        use p256::pkcs8::EncodePublicKey;

        let sk = SigningKey::from_slice(&[7u8; 32]).unwrap();
        let pk = B64.encode(sk.verifying_key().to_public_key_der().unwrap().as_bytes());
        let payload = b"1700000000[{\"event\":\"delivered\"}]";
        let sig: Signature = sk.sign(payload);
        let sig_b64 = B64.encode(sig.to_der().as_bytes());

        assert!(super::signing::verify_sendgrid_signature(payload, &sig_b64, &pk, false));
        assert!(!super::signing::verify_sendgrid_signature(b"tampered", &sig_b64, &pk, false));
        // HMAC fallback is only honoured when explicitly allowed and key isn't a public key.
        assert!(!super::signing::verify_sendgrid_signature(payload, &sig_b64, "not-a-key", false));
    }

    use super::*;
    use axum::http::HeaderMap;
    use ipnet::IpNet;
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

    // ── #898: IPv6 parse-error handling tests ────────────────────────────

    /// Malformed IPv6 in X-Forwarded-For must be skipped; the next valid entry
    /// is used instead of returning the malformed string.
    #[test]
    fn malformed_ipv6_in_xff_is_skipped_falls_back_to_next_valid() {
        // First entry is malformed IPv6, second is valid IPv4
        let headers = xff("::gggg:1, 1.2.3.4");
        let ci = addr("5.5.5.5:80");
        assert_eq!(
            extract_client_ip(&headers, Some(&ci), true),
            "1.2.3.4",
            "malformed IPv6 must be skipped; valid subsequent entry should be used"
        );
    }

    /// All entries in X-Forwarded-For are malformed — must fall back to socket.
    #[test]
    fn all_malformed_xff_entries_fall_back_to_socket() {
        let headers = xff("not-an-ip, :::bad:::, 999.999.999.999");
        let ci = addr("10.0.0.1:8080");
        assert_eq!(
            extract_client_ip(&headers, Some(&ci), true),
            "10.0.0.1",
            "when all XFF entries are malformed, socket address must be used"
        );
    }

    /// Empty X-Forwarded-For header falls back to socket address.
    #[test]
    fn empty_xff_header_falls_back_to_socket() {
        let headers = xff("");
        let ci = addr("192.168.1.1:9090");
        assert_eq!(extract_client_ip(&headers, Some(&ci), true), "192.168.1.1");
    }

    /// Malformed X-Real-IP falls back to socket address.
    #[test]
    fn malformed_x_real_ip_falls_back_to_socket() {
        let headers = xri("::invalid-ipv6");
        let ci = addr("172.16.0.1:80");
        assert
