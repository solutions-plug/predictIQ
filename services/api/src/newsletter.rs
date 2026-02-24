use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use anyhow::Context;
use serde_json::json;
use tokio::sync::Mutex;

use crate::config::Config;

#[derive(Clone, Default)]
pub struct IpRateLimiter {
    entries: Arc<Mutex<HashMap<String, Vec<Instant>>>>,
}

impl IpRateLimiter {
    pub async fn allow(&self, key: &str, max_requests: usize, window: Duration) -> bool {
        let now = Instant::now();
        let mut map = self.entries.lock().await;
        let entry = map.entry(key.to_string()).or_default();

        entry.retain(|instant| now.duration_since(*instant) <= window);
        if entry.len() >= max_requests {
            return false;
        }

        entry.push(now);
        true
    }
}

pub async fn send_confirmation_email(config: &Config, email: &str, token: &str) -> anyhow::Result<()> {
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
    use super::IpRateLimiter;
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
}
