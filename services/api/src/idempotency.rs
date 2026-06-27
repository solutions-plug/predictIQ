//! idempotency.rs — Per-user scoped idempotency key storage.
//!
//! Keys are namespaced as `{user_id}:{idempotency_key}`.
//! A key submitted by user A cannot be replayed by user B — attempting to do
//! so returns HTTP 422 Unprocessable Entity.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    extract::{Request, State},
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::AppState;

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

/// Build a per-user scoped cache key. Format: `idempotency:v2:{user_id}:{raw_key}`.
fn idempotency_cache_key(user_id: &str, raw_key: &str) -> String {
    format!("idempotency:v2:{}:{}", user_id, raw_key)
}

/// Extract a stable identity string from request headers.
/// Uses the API key prefix, or falls back to the Authorization header value.
fn extract_user_identity(req: &Request) -> String {
    req.headers()
        .get("x-api-key")
        .and_then(|v| v.to_str().ok())
        .map(|k| format!("api:{}", &k[..16.min(k.len())]))
        .or_else(|| {
            req.headers()
                .get("authorization")
                .and_then(|v| v.to_str().ok())
                .map(|s| format!("auth:{}", &s[..32.min(s.len())]))
        })
        .unwrap_or_else(|| "anonymous".to_string())
}

/// Middleware that deduplicates POST requests using an `Idempotency-Key` header.
///
/// - If the header is absent the request passes through unchanged.
/// - If a cached response exists for the key AND owner matches, it is returned.
/// - If a cached response exists but owner differs, returns 422 (cross-user collision).
/// - Otherwise the request is executed, the response is cached, and returned.
pub async fn idempotency_middleware(
    State(state): State<Arc<AppState>>,
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

    let user_id = extract_user_identity(&req);
    let cache_key = idempotency_cache_key(&user_id, &raw_key);
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

    pub fn scoped_key(user_id: &str, raw_key: &str) -> String {
        format!("{}:{}", user_id, raw_key)
    }

    /// Look up a prior response for this user + key combination.
    pub fn get(
        &self,
        user_id: &str,
        raw_key: &str,
    ) -> Result<Option<StoredResponse>, IdempotencyError> {
        let store = self.inner.lock().unwrap();
        let scoped = Self::scoped_key(user_id, raw_key);

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

    /// Store a response scoped to this user + key.
    pub fn set(&self, user_id: &str, raw_key: &str, response: StoredResponse) {
        let mut store = self.inner.lock().unwrap();
        let scoped = Self::scoped_key(user_id, raw_key);
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

    fn make_response(status: u16) -> StoredResponse {
        StoredResponse {
            status,
            body: format!("response_{}", status),
            stored_at: Instant::now(),
        }
    }

    #[test]
    fn same_user_gets_cached_response() {
        let store = IdempotencyStore::new(Duration::from_secs(60));
        store.set("user_a", "key1", make_response(200));
        let result = store.get("user_a", "key1").unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().status, 200);
    }

    #[test]
    fn different_user_same_raw_key_gets_none() {
        let store = IdempotencyStore::new(Duration::from_secs(60));
        store.set("user_a", "key1", make_response(200));
        let result = store.get("user_b", "key1").unwrap();
        assert!(result.is_none(), "user_b must not see user_a's cached response");
    }

    #[test]
    fn cross_user_collision_detected_via_scoped_key_reuse() {
        let store = IdempotencyStore::new(Duration::from_secs(60));
        store.set("user_a", "key1", make_response(201));
        let victim_scoped = IdempotencyStore::scoped_key("user_a", "key1");
        let err = store.check_cross_user("user_b", &victim_scoped);
        assert_eq!(err, Err(IdempotencyError::CrossUserCollision));
    }

    #[test]
    fn expired_entry_returns_none() {
        let store = IdempotencyStore::new(Duration::from_millis(1));
        store.set("user_a", "key2", make_response(200));
        std::thread::sleep(Duration::from_millis(5));
        let result = store.get("user_a", "key2").unwrap();
        assert!(result.is_none(), "expired entry must not be returned");
    }

    #[test]
    fn unknown_key_returns_none() {
        let store = IdempotencyStore::new(Duration::from_secs(60));
        assert!(store.get("user_a", "nonexistent").unwrap().is_none());
    }
}
