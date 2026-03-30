use std::time::Duration;
use crate::cache::RedisCache;

use anyhow::Context;
use serde_json::json;
use tokio::sync::Mutex;

use crate::config::Config;

#[derive(Clone)]
pub struct IpRateLimiter {
    pub cache: RedisCache,
}

impl IpRateLimiter {
    pub fn new(cache: RedisCache) -> Self {
        Self { cache }
    }

    /// Returns true if allowed, false if rate limited.
    pub async fn allow(&self, key: &str, max_requests: usize, window: Duration) -> bool {
        let redis_key = format!("newsletter:ratelimit:{}", key);
        let mut conn = self.cache.manager.clone();
        let script = r#"
            local current = redis.call('INCR', KEYS[1])
            if tonumber(current) == 1 then
                redis.call('EXPIRE', KEYS[1], ARGV[1])
            end
            return current
        "#;
        let ttl_secs = window.as_secs() as usize;
        let result: Result<u64, _> = redis::Script::new(script)
            .key(&redis_key)
            .arg(ttl_secs)
            .invoke_async(&mut conn)
            .await;
        match result {
            Ok(count) => count as usize <= max_requests,
            Err(_) => true, // fail open if Redis is unavailable
        }
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

        let entry = self.pending.remove(&email).unwrap();

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
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("sendgrid returned {status}: {body}");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[tokio::test]
    async fn limiter_blocks_when_max_requests_reached() {
        let limiter = IpRateLimiter::default();
        let key = "203.0.113.1";
        let window = Duration::from_secs(60);

        assert!(limiter.allow(key, 2, window).await);
        assert!(limiter.allow(key, 2, window).await);
        assert!(!limiter.allow(key, 2, window).await);
    }

    #[tokio::test]
    async fn limiter_allows_after_window_expires() {
        let limiter = IpRateLimiter::default();
        let key = "198.51.100.42";
        let window = Duration::from_millis(20);

        assert!(limiter.allow(key, 1, window).await);
        assert!(!limiter.allow(key, 1, window).await);

        tokio::time::sleep(Duration::from_millis(25)).await;

        assert!(limiter.allow(key, 1, window).await);
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
