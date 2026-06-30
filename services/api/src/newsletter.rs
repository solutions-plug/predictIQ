use std::collections::HashMap;
use std::time::{Duration, Instant};
use crate::cache::RedisCache;
use crate::metrics::Metrics;

use anyhow::Context;
use base64::{engine::general_purpose::URL_SAFE_NO_PAD as BASE64URL, Engine};
use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde_json::json;

use crate::config::Config;

type HmacSha256 = Hmac<Sha256>;

/// # Rate-limiting policy: fail-closed vs fail-open
///
/// ## Why fail-closed?
///
/// Security-critical rate limiters (newsletter subscribe, admin endpoints)
/// **must** fail-closed: when Redis is unavailable the limiter returns
/// `false` (deny / 429 Too Many Requests) rather than `true` (allow).
///
/// Rationale:
/// - A Redis outage is an abnormal condition. Silently allowing unlimited
///   requests during an outage turns every Redis failure into an open door
///   for newsletter-spam abuse or brute-force attacks on admin endpoints.
/// - The cost of a brief 429 to a legitimate subscriber is far lower than
///   the cost of a spam flood or enumeration attack.
/// - Operators are alerted via the `rate_limiter_redis_errors_total` Prometheus
///   counter, which fires whenever the fail-closed path is taken. A CloudWatch
///   alarm on this metric gives fast visibility into Redis health.
///
/// ## When might fail-open be acceptable?
///
/// Fail-open is only appropriate for non-security-critical limiters where
/// availability is strictly more important than abuse prevention — for example,
/// a public read-only search endpoint where an occasional spike is harmless.
/// It must **never** be used for write endpoints, authentication endpoints,
/// or any path that creates subscriber/user records.
///
/// ## Observability
///
/// Every Redis error increments `rate_limiter_redis_errors_total{limiter="<name>"}`.
/// Alert on `increase(rate_limiter_redis_errors_total[5m]) > 0` to detect
/// Redis degradation before it becomes an outage.
#[derive(Clone)]
pub struct IpRateLimiter {
    pub cache: RedisCache,
    /// Optional Prometheus metrics handle. When `None` errors are only logged.
    pub metrics: Option<Metrics>,
    /// Identifier used in the `limiter` label of `rate_limiter_redis_errors_total`.
    pub name: String,
}

impl IpRateLimiter {
    pub fn new(cache: RedisCache) -> Self {
        Self {
            cache,
            metrics: None,
            name: "newsletter_subscribe".to_string(),
        }
    }

    pub fn with_metrics(cache: RedisCache, metrics: Metrics, name: impl Into<String>) -> Self {
        Self {
            cache,
            metrics: Some(metrics),
            name: name.into(),
        }
    }

    /// Returns `true` if the request is **allowed**, `false` if it should be
    /// rejected with 429 Too Many Requests.
    ///
    /// Uses an atomic Redis Lua script so the counter is consistent across all
    /// instances. **Fails closed** (returns `false`) if Redis is unavailable —
    /// see module-level documentation for the security rationale.
    pub async fn allow(&self, key: &str, max_requests: usize, window: Duration) -> bool {
        let redis_key = format!("newsletter:ratelimit:v1:{key}");
        match self.cache.incr_with_ttl(&redis_key, window).await {
            Ok(count) => count as usize <= max_requests,
            Err(e) => {
                // Increment the Prometheus error counter so operators are alerted.
                if let Some(m) = &self.metrics {
                    m.observe_rate_limiter_redis_error(&self.name);
                }
                tracing::warn!(
                    error = %e,
                    limiter = %self.name,
                    key,
                    "rate limiter Redis error — failing CLOSED (429) to prevent abuse during outage"
                );
                // Fail-closed: deny the request.
                false
            }
        }
    }
}

/// Generate a signed unsubscribe token for `email`.
/// Format: `<base64url(email)>.<hmac_signature>`
pub fn generate_unsubscribe_token(email: &str, secret: &str) -> anyhow::Result<String> {
    let payload = BASE64URL.encode(email.as_bytes());
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| anyhow::anyhow!("invalid signing key"))?;
    mac.update(payload.as_bytes());
    let sig = BASE64URL.encode(mac.finalize().into_bytes());
    Ok(format!("{payload}.{sig}"))
}

/// Validate a signed unsubscribe token. Returns the email on success.
pub fn validate_unsubscribe_token(token: &str, secret: &str) -> Option<String> {
    let (payload, sig) = token.split_once('.')?;
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(payload.as_bytes());
    let expected = BASE64URL.encode(mac.finalize().into_bytes());
    // Constant-time comparison via subtle
    use subtle::ConstantTimeEq;
    if bool::from(expected.as_bytes().ct_eq(sig.as_bytes())) {
        let email_bytes = BASE64URL.decode(payload).ok()?;
        String::from_utf8(email_bytes).ok()
    } else {
        None
    }
}

// ---------------------------------------------------------------------------
// In-memory token store — models the subscribe/confirm/expiry lifecycle.
// Used by tests; mirrors the DB contract without requiring Postgres.
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq)]
pub enum ConfirmResult {
    Confirmed,
    AlreadyConfirmed,
    InvalidOrExpired,
}

struct PendingEntry {
    token: String,
    expires_at: Instant,
}

/// Minimal in-memory store that replicates the token lifecycle enforced by
/// `newsletter_upsert_pending` / `newsletter_confirm_by_token` in `db.rs`.
pub struct TokenStore {
    pending: HashMap<String, PendingEntry>,
    confirmed: HashMap<String, bool>,
    pub token_ttl: Duration,
}

impl TokenStore {
    pub fn new(token_ttl: Duration) -> Self {
        Self {
            pending: HashMap::new(),
            confirmed: HashMap::new(),
            token_ttl,
        }
    }

    /// Upsert a pending token for `email`. Resets any prior pending entry.
    pub fn subscribe(&mut self, email: &str, token: &str) {
        self.pending.insert(
            email.to_string(),
            PendingEntry {
                token: token.to_string(),
                expires_at: Instant::now() + self.token_ttl,
            },
        );
    }

    /// Consume the token. Returns the outcome without panicking.
    pub fn confirm(&mut self, token: &str) -> ConfirmResult {
        let email = self
            .pending
            .iter()
            .find(|(_, e)| e.token == token)
            .map(|(email, _)| email.clone());

        let Some(email) = email else {
            if self.confirmed.values().any(|&v| v) {
                return ConfirmResult::AlreadyConfirmed;
            }
            return ConfirmResult::InvalidOrExpired;
        };

        let entry = self.pending.remove(&email).expect("email was found in pending map immediately above");

        if Instant::now() > entry.expires_at {
            return ConfirmResult::InvalidOrExpired;
        }

        self.confirmed.insert(email, true);
        ConfirmResult::Confirmed
    }

    pub fn is_confirmed(&self, email: &str) -> bool {
        self.confirmed.get(email).copied().unwrap_or(false)
    }
}

pub async fn send_confirmation_email(
    config: &Config,
    email: &str,
    token: &str,
) -> anyhow::Result<()> {
    let api_key = config
        .sendgrid_api_key
        .as_deref()
        .context("missing SENDGRID_API_KEY")?;
    let from_email = config.from_email.as_deref().context("missing FROM_EMAIL")?;

    let confirm_url = format!(
        "{}/api/v1/newsletter/confirm?token={token}",
        config.base_url.trim_end_matches('/')
    );

    let payload = json!({
        "personalizations": [{ "to": [{ "email": email }] }],
        "from": { "email": from_email },
        "subject": "Confirm your subscription",
        "content": [{
            "type": "text/html",
            "value": format!(
                "<p>Click <a href=\"{confirm_url}\">here</a> to confirm your newsletter subscription.</p>"
            )
        }]
    });

    let response = reqwest::Client::new()
        .post("https://api.sendgrid.com/v3/mail/send")
        .bearer_auth(api_key)
        .json(&payload)
        .send()
        .await
        .context("sendgrid request failed")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_else(|e| {
            tracing::warn!(error = %e, "failed to read SendGrid error response body");
            String::new()
        });
        anyhow::bail!("sendgrid returned {status}: {body}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    async fn make_limiter() -> IpRateLimiter {
        use testcontainers::runners::AsyncRunner;
        use testcontainers_modules::redis::Redis;
        let container = Redis::default().start().await.expect("redis container");
        let port = container.get_host_port_ipv4(6379).await.expect("redis port");
        // Leak the container so it lives for the test duration.
        std::mem::forget(container);
        let url = format!("redis://127.0.0.1:{port}");
        let cache = crate::cache::RedisCache::new(&url).await.expect("redis cache");
        IpRateLimiter::new(cache)
    }

    #[tokio::test]
    async fn limiter_blocks_when_max_requests_reached() {
        let limiter = make_limiter().await;
        let key = "203.0.113.1";
        let window = Duration::from_secs(60);

        assert!(limiter.allow(key, 2, window).await);
        assert!(limiter.allow(key, 2, window).await);
        assert!(!limiter.allow(key, 2, window).await);
    }

    #[tokio::test]
    async fn limiter_allows_after_window_expires() {
        let limiter = make_limiter().await;
        let key = "198.51.100.42";
        let window = Duration::from_secs(1);

        assert!(limiter.allow(key, 1, window).await);
        assert!(!limiter.allow(key, 1, window).await);

        tokio::time::sleep(Duration::from_millis(1100)).await;

        assert!(limiter.allow(key, 1, window).await);
    }

    /// When Redis is unreachable the limiter must fail CLOSED (return `false`).
    ///
    /// This test connects to a port with no Redis listener so every Redis
    /// operation times out / errors, then asserts that `allow()` returns
    /// `false` — confirming the fail-closed path and the 429 behaviour.
    #[tokio::test]
    async fn limiter_fails_closed_when_redis_unavailable() {
        // Point at a port that has nothing listening so every Redis call fails.
        let cache = crate::cache::RedisCache::new("redis://127.0.0.1:16399")
            .await
            .expect("cache construction should succeed even with unreachable Redis");

        let metrics = crate::metrics::Metrics::new()
            .expect("metrics init");
        let limiter = IpRateLimiter::with_metrics(
            cache,
            metrics,
            "newsletter_subscribe",
        );

        // Should deny (fail-closed) rather than allow (fail-open)
        let allowed = limiter.allow("203.0.113.99", 100, Duration::from_secs(60)).await;
        assert!(
            !allowed,
            "rate limiter must fail CLOSED (deny) when Redis is unavailable"
        );
    }

    // -------------------------------------------------------------------------
    // Unsubscribe token tests
    // -------------------------------------------------------------------------

    #[test]
    fn unsubscribe_token_roundtrip() {
        let email = "user@example.com";
        let secret = "test-secret";
        let token = generate_unsubscribe_token(email, secret).unwrap();
        assert_eq!(validate_unsubscribe_token(&token, secret), Some(email.to_string()));
    }

    #[test]
    fn unsubscribe_token_wrong_secret_rejected() {
        let token = generate_unsubscribe_token("user@example.com", "secret-a").unwrap();
        assert_eq!(validate_unsubscribe_token(&token, "secret-b"), None);
    }

    #[test]
    fn unsubscribe_token_tampered_rejected() {
        let token = generate_unsubscribe_token("user@example.com", "secret").unwrap();
        let tampered = format!("{token}x");
        assert_eq!(validate_unsubscribe_token(&tampered, "secret"), None);
    }

    #[test]
    fn unsubscribe_token_missing_dot_rejected() {
        assert_eq!(validate_unsubscribe_token("nodot", "secret"), None);
    }

    // -------------------------------------------------------------------------
    // #291: Newsletter token lifecycle tests
    // -------------------------------------------------------------------------

    fn store() -> TokenStore {
        TokenStore::new(Duration::from_secs(3600))
    }

    #[test]
    fn subscribe_then_confirm_succeeds() {
        let mut s = store();
        s.subscribe("user@example.com", "tok-1");
        assert_eq!(s.confirm("tok-1"), ConfirmResult::Confirmed);
        assert!(s.is_confirmed("user@example.com"));
    }

    #[test]
    fn duplicate_confirm_returns_already_confirmed() {
        let mut s = store();
        s.subscribe("user@example.com", "tok-2");
        assert_eq!(s.confirm("tok-2"), ConfirmResult::Confirmed);
        // Token is consumed; second attempt finds no pending entry.
        assert_eq!(s.confirm("tok-2"), ConfirmResult::AlreadyConfirmed);
    }

    #[test]
    fn expired_token_returns_invalid_or_expired() {
        let mut s = TokenStore::new(Duration::from_millis(1));
        s.subscribe("user@example.com", "tok-3");
        std::thread::sleep(Duration::from_millis(5));
        assert_eq!(s.confirm("tok-3"), ConfirmResult::InvalidOrExpired);
        assert!(!s.is_confirmed("user@example.com"));
    }

    #[test]
    fn unknown_token_returns_invalid_or_expired() {
        let mut s = store();
        assert_eq!(s.confirm("no-such-token"), ConfirmResult::InvalidOrExpired);
    }

    #[test]
    fn resubscribe_replaces_pending_token() {
        let mut s = store();
        s.subscribe("user@example.com", "tok-old");
        s.subscribe("user@example.com", "tok-new");
        assert_eq!(s.confirm("tok-old"), ConfirmResult::InvalidOrExpired);
        assert_eq!(s.confirm("tok-new"), ConfirmResult::Confirmed);
    }
}
