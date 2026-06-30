//! Redis-backed rate limiting for predictIQ API.
//!
//! Uses a sliding window algorithm implemented with Redis atomic Lua script
//! to persist counters across service restarts and horizontal scaling.
//!
//! ## Algorithm
//! For each (key, window) pair:
//!  1. ZADD with current timestamp as score
//!  2. ZREMRANGEBYSCORE to evict expired entries outside the window
//!  3. ZCARD to count requests in window
//!  4. EXPIRE to align Redis TTL with the window
//!
//! All four commands execute atomically via a Lua script.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use deadpool_redis::{Pool as RedisPool, redis::AsyncCommands};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests:   u64,
    pub window_seconds: u64,
    pub key_prefix:     String,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests:   100,
            window_seconds: 60,
            key_prefix:     "ratelimit".to_string(),
        }
    }
}

#[derive(Clone)]
pub struct RateLimitState {
    pub redis:   Arc<RedisPool>,
    pub config:  RateLimitConfig,
    /// Optional metrics sink. When present, rejections are counted under
    /// the `rate_limit_rejections_total` Prometheus counter.
    pub metrics: Option<crate::metrics::Metrics>,
}

#[derive(Serialize)]
struct RateLimitError {
    error:       &'static str,
    message:     String,
    retry_after: u64,
}

// KEYS[1] = rate limit key
// ARGV[1] = current timestamp (ms)
// ARGV[2] = window start (ms)
// ARGV[3] = window TTL (seconds)
// ARGV[4] = unique member
// Returns request count within the current window.
const SLIDING_WINDOW_SCRIPT: &str = r#"
local key          = KEYS[1]
local now          = tonumber(ARGV[1])
local window_start = tonumber(ARGV[2])
local ttl          = tonumber(ARGV[3])
local member       = ARGV[4]

redis.call('ZADD', key, now, member)
redis.call('ZREMRANGEBYSCORE', key, '-inf', window_start)
local count = redis.call('ZCARD', key)
redis.call('EXPIRE', key, ttl)
return count
"#;

pub async fn check_rate_limit(
    redis: &RedisPool,
    config: &RateLimitConfig,
    client_key: &str,
) -> Result<u64, u64> {
    let mut conn = redis
        .get()
        .await
        .map_err(|_| config.window_seconds)?;

    let now_ms: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    let window_start_ms = now_ms.saturating_sub(config.window_seconds * 1_000);
    let member = format!("{}-{}", now_ms, fastrand::u64(..));
    let redis_key = format!("{}:{}:{}", config.key_prefix, client_key, config.window_seconds);

    let script = deadpool_redis::redis::Script::new(SLIDING_WINDOW_SCRIPT);
    let count: u64 = script
        .key(&redis_key)
        .arg(now_ms)
        .arg(window_start_ms)
        .arg(config.window_seconds + 1)
        .arg(&member)
        .invoke_async(&mut conn)
        .await
        .map_err(|_| config.window_seconds)?;

    if count > config.max_requests {
        Err(config.window_seconds)
    } else {
        Ok(count)
    }
}

pub async fn rate_limit_middleware(
    State(state): State<RateLimitState>,
    headers: HeaderMap,
    req: axum::extract::Request,
    next: Next,
) -> Response {
    let client_key = client_key_from_headers(&headers);

    match check_rate_limit(&state.redis, &state.config, &client_key).await {
        Ok(_count) => next.run(req).await,
        Err(retry_after) => {
            if let Some(m) = &state.metrics {
                m.observe_rate_limit_rejection(&state.config.key_prefix);
            }
            tracing::warn!(
                client_key = %client_key,
                route = %state.config.key_prefix,
                retry_after,
                "rate limit exceeded"
            );
            let body = RateLimitError {
                error:   "rate_limit_exceeded",
                message: format!(
                    "Rate limit of {} requests per {}s exceeded. Retry after {} seconds.",
                    state.config.max_requests, state.config.window_seconds, retry_after
                ),
                retry_after,
            };
            (
                StatusCode::TOO_MANY_REQUESTS,
                [("Retry-After", retry_after.to_string())],
                Json(body),
            )
                .into_response()
        }
    }
}

fn client_key_from_headers(headers: &HeaderMap) -> String {
    headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn client_key_from_x_forwarded_for_picks_first_ip() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", "1.2.3.4, 5.6.7.8".parse().unwrap());
        assert_eq!(client_key_from_headers(&headers), "1.2.3.4");
    }

    #[test]
    fn client_key_falls_back_to_unknown() {
        let headers = HeaderMap::new();
        assert_eq!(client_key_from_headers(&headers), "unknown");
    }

    #[test]
    fn config_defaults_are_sensible() {
        let cfg = RateLimitConfig::default();
        assert_eq!(cfg.max_requests,   100);
        assert_eq!(cfg.window_seconds,  60);
        assert!(!cfg.key_prefix.is_empty());
    }

    #[test]
    fn rate_limit_state_metrics_field_is_optional() {
        let state = RateLimitState {
            redis:   std::sync::Arc::new(deadpool_redis::Config::from_url("redis://127.0.0.1")
                .create_pool(Some(deadpool_redis::Runtime::Tokio1)).unwrap()),
            config:  RateLimitConfig::default(),
            metrics: None,
        };
        assert!(state.metrics.is_none());
    }
}
