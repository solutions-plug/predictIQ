//! idempotency.rs — Per-user scoped idempotency key storage.
//!
//! ## Cache key derivation
//!
//! The idempotency cache key is derived from **three** components, never from
//! the raw `Idempotency-Key` header value alone:
//!
//! ```text
//! idempotency:v2:{user_id}:{route_path}:{raw_key}
//! ```
//!
//! - `user_id` — the caller identity, derived from the API key (hashed), the
//!   `Authorization` header, or, for unauthenticated callers, a hash of the
//!   client IP + `User-Agent` (see [`extract_user_identity`]).
//! - `route_path` — the request path, so the same key string used on two
//!   different routes never collides.
//! - `raw_key` — the trimmed `Idempotency-Key` header value.
//!
//! Because the key is scoped by caller identity and route, a key submitted by
//! user A cannot be replayed by user B (nor on a different route): attempting
//! to do so returns HTTP 422 Unprocessable Entity.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::{ConnectInfo, Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{security::extract_client_ip_cidrs, AppState};

const IDEMPOTENCY_HEADER: &str = "Idempotency-Key";
const MAX_KEY_LEN: usize = 128;

#[derive(Serialize, Deserialize, Clone)]
struct CachedResponse {
    status: u16,
    body: Vec<u8>,
    content_type: Option<String>,
    /// The user/API-key identity that originally created this entry.
    owner: String,
}

/// Build a per-user, per-route scoped cache key.
///
/// Format: `idempotency:v2:{user_id}:{route_path}:{raw_key}`. Scoping by both
/// caller identity and route path ensures two distinct actors (or the same
/// actor on different routes) reusing the same raw key never share a cache
/// entry.
fn idempotency_cache_key(user_id: &str, route_path: &str, raw_key: &str) -> String {
    format!("idempotency:v2:{}:{}:{}", user_id, route_path, raw_key)
}

/// Extract a stable identity string from request headers.
///
/// Uses the API key prefix, or falls back to the Authorization header value.
/// If neither credential is present, the caller is unauthenticated: identity
/// is derived from the client IP + User-Agent rather than a single fixed
/// `"anonymous"` bucket, so unrelated unauthenticated clients never collide
/// on the same scoped cache key (issue #1104).
fn extract_user_identity(req: &Request, client_ip: &str) -> String {
    req.headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|k| {
            let hash = hex::encode(Sha256::digest(k.as_bytes()));
            format!("api:{}", hash)
        })
        .or_else(|| {
            req.headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .map(|s| format!("auth:{}", s.chars().take(32).collect::<String>()))
        })
        .unwrap_or_else(|| {
            let user_agent = req
                .headers()
                .get("user-agent")
                .and_then(|v| v.to_str().ok())
                .unwrap_or("");
            let hash = hex::encode(Sha256::digest(
                format!("{client_ip}|{user_agent}").as_bytes(),
            ));
            format!("anon:{}", hash)
        })
}

/// Middleware that deduplicates POST requests using an `Idempotency-Key` header.
///
/// - If the header is absent the request passes through unchanged.
/// - If a cached response exists for the key AND owner matches, it is returned.
/// - If a cached response exists but owner differs, returns 422 (cross-user collision).
/// - Otherwise the request is executed, the response is cached, and returned.
pub async fn idempotency_middleware(
    State(state): State<Arc<AppState>>,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    req: Request,
    next: Next,
) -> Response {
    let raw_key = match req
        .headers()
        .get(IDEMPOTENCY_HEADER)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty() && s.len() <= MAX_KEY_LEN)
    {
        Some(k) => k,
        None => return next.run(req).await,
    };

    let client_ip = extract_client_ip_cidrs(
        req.headers(),
        connect_info.as_ref(),
        state.config.trust_proxy,
        &state.config.trusted_proxy_cidrs,
    );
    let user_id = extract_user_identity(&req, &client_ip);
    let route_path = req.uri().path().to_string();
    let cache_key = idempotency_cache_key(&user_id, &route_path, &raw_key);
    let ttl = Duration::from_secs(state.config.idempotency_window_secs);

    // Return cached response if present and owned by this user
    if let Ok(Some(cached)) = state.cache.get_json::<CachedResponse>(&cache_key).await {
        if cached.owner != user_id {
            // Cross-user collision: same scoped key with different owner (should not happen
            // with scoped keys, but defensive check against key-format changes)
            return StatusCode::UNPROCESSABLE_ENTITY.into_response();
        }
        let status = StatusCode::from_u16(cached.status).unwrap_or(StatusCode::OK);
        let mut resp = Response::builder().status(status);
        if let Some(ct) = cached.content_type {
            resp = resp.header(axum::http::header::CONTENT_TYPE, ct);
        }
        resp = resp.header("Idempotency-Replayed", "true");
        return resp
            .body(Body::from(cached.body))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response());
    }

    // Execute the request
    let response = next.run(req).await;
    let (parts, body) = response.into_parts();

    let bytes = match axum::body::to_bytes(body, usize::MAX).await {
        Ok(b) => b,
        Err(_) => return StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    };

    // Cache only successful responses (2xx)
    if parts.status.is_success() {
        let content_type = parts
            .headers
            .get(axum::http::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let cached = CachedResponse {
            status: parts.status.as_u16(),
            body: bytes.to_vec(),
            content_type,
            owner: user_id,
        };
        let _ = state.cache.set_json(&cache_key, &cached, ttl).await;
    }

    let mut resp = Response::from_parts(parts, Body::from(bytes));
    resp.headers_mut()
        .insert("Idempotency-Replayed", HeaderValue::from_static("false"));
    resp
}

// ── Standalone testable IdempotencyStore ─────────────────────────────────────

/// Cached response stored against a scoped idempotency key.
#[derive(Clone, Debug)]
pub struct StoredResponse {
    pub status: u16,
    pub body: String,
    pub stored_at: Instant,
}

#[derive(Debug, PartialEq)]
pub enum IdempotencyError {
    /// The key was previously used by a different user — reject with 422.
    CrossUserCollision,
}

/// In-memory idempotency store (replace with Redis for multi-instance deployments).
#[derive(Default, Clone)]
pub struct IdempotencyStore {
    inner: Arc<Mutex<HashMap<String, (String, StoredResponse)>>>,
    ttl: Duration,
}

impl IdempotencyStore {
    pub fn new(ttl: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(HashMap::new())),
            ttl,
        }
    }

    /// Scoped key including caller identity and route path.
    ///
    /// Format: `{user_id}:{route_path}:{raw_key}`. Mirrors the middleware's
    /// `idempotency_cache_key` derivation so the two stay consistent.
    pub fn scoped_key(user_id: &str, route_path: &str, raw_key: &str) -> String {
        format!("{}:{}:{}", user_id, route_path, raw_key)
    }

    /// Look up a prior response for this user + route + key combination.
    pub fn get(
        &self,
        user_id: &str,
        route_path: &str,
        raw_key: &str,
    ) -> Result<Option<StoredResponse>, IdempotencyError> {
        let store = self.inner.lock().unwrap();
        let scoped = Self::scoped_key(user_id, route_path, raw_key);

        if let Some((owner, cached)) = store.get(&scoped) {
            if owner != user_id {
                return Err(IdempotencyError::CrossUserCollision);
            }
            if cached.stored_at.elapsed() < self.ttl {
                return Ok(Some(cached.clone()));
            }
        }
        Ok(None)
    }

    /// Store a response scoped to this user + route + key.
    pub fn set(
        &self,
        user_id: &str,
        route_path: &str,
        raw_key: &str,
        response: StoredResponse,
    ) {
        let mut store = self.inner.lock().unwrap();
        let scoped = Self::scoped_key(user_id, route_path, raw_key);
        store.insert(scoped, (user_id.to_owned(), response));
    }

    /// Detect cross-user replay: attacker submits victim's scoped key verbatim.
    pub fn check_cross_user(
        &self,
        attacker_id: &str,
        victim_key_scoped: &str,
    ) -> Result<Option<StoredResponse>, IdempotencyError> {
        let store = self.inner.lock().unwrap();
        if let Some((owner, _)) = store.get(victim_key_scoped) {
            if owner != attacker_id {
                return Err(IdempotencyError::CrossUserCollision);
            }
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resp(body: &str) -> StoredResponse {
        StoredResponse {
            status: 200,
            body: body.to_string(),
            stored_at: Instant::now(),
        }
    }

    #[test]
    fn cache_key_includes_owner_and_route() {
        let a = idempotency_cache_key("user-a", "/v1/admin/things", "key-1");
        let b = idempotency_cache_key("user-b", "/v1/admin/things", "key-1");
        let c = idempotency_cache_key("user-a", "/v1/admin/other", "key-1");

        // Same raw key, different owner => different cache key.
        assert_ne!(a, b);
        // Same raw key, same owner, different route => different cache key.
        assert_ne!(a, c);
        // Key is not derived from the raw key alone.
        assert_ne!(a, "key-1");
    }

    #[test]
    fn distinct_actors_same_key_same_route_are_independent() {
        let store = IdempotencyStore::new(Duration::from_secs(300));
        let route = "/v1/admin/things";
        let key = "shared-key";

        store.set("user-a", route, key, resp("a-result"));
        store.set("user-b", route, key, resp("b-result"));

        let a = store.get("user-a", route, key).unwrap().unwrap();
        let b = store.get("user-b", route, key).unwrap().unwrap();

        assert_eq!(a.body, "a-result");
        assert_eq!(b.body, "b-result");
        assert_ne!(a.body, b.body);
    }

    #[test]
    fn same_actor_same_key_different_route_are_independent() {
        let store = IdempotencyStore::new(Duration::from_secs(300));
        let key = "shared-key";

        store.set("user-a", "/v1/admin/things", key, resp("things"));
        store.set("user-a", "/v1/admin/other", key, resp("other"));

        assert_eq!(
            store.get("user-a", "/v1/admin/things", key).unwrap().unwrap().body,
            "things"
        );
        assert_eq!(
            store.get("user-a", "/v1/admin/other", key).unwrap().unwrap().body,
            "other"
        );
    }
}
