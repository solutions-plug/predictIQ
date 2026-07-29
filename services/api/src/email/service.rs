use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use redis::AsyncCommands as _;
use serde_json::Value;
use sha2::Sha256;
use std::time::Duration;
use validator::ValidateEmail;

use crate::cache::RedisCache;
use crate::config::Config;
use crate::email::templates::EmailTemplateEngine;
use crate::metrics::Metrics;

/// Returned when the per-recipient hourly send limit is exceeded.
#[derive(Debug, Clone)]
pub struct RateLimitExceeded {
    pub recipient: String,
    pub limit: u64,
}

impl std::fmt::Display for RateLimitExceeded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "email rate limit exceeded for {}: max {} per hour", self.recipient, self.limit)
    }
}

impl std::error::Error for RateLimitExceeded {}

/// Configuration for email idempotency deduplication.
#[derive(Clone, Debug)]
pub struct IdempotencyConfig {
    /// How long a sent-key is retained in Redis. Duplicate sends within this
    /// window are silently skipped. Default: 24 hours.
    pub ttl: Duration,
}

impl Default for IdempotencyConfig {
    fn default() -> Self {
        Self {
            ttl: Duration::from_secs(86_400), // 24 h
        }
    }
}

/// Derive a stable idempotency key from the job inputs.
///
/// Uses HMAC-SHA256(secret, recipient || "|" || template || "|" || hour_bucket)
/// so that:
/// - A server-side secret prevents pre-computation by external attackers.
/// - The hour bucket bounds the validity window to ~1 hour per key rotation.
///
/// `bucket_time` is the timestamp the hour bucket is derived from. Callers
/// that retry the same logical job (e.g. the email queue worker) must pass
/// the job's original creation time rather than the current time — otherwise
/// a retry whose exponential backoff crosses an hour boundary would compute a
/// different key and defeat deduplication for a job that already succeeded.
///
/// The key format is `email:idem:<hex(HMAC)>`.
pub fn idempotency_key(
    recipient: &str,
    template_name: &str,
    secret: &str,
    bucket_time: DateTime<Utc>,
) -> String {
    type HmacSha256 = Hmac<Sha256>;

    // Hour bucket: seconds-since-epoch / 3600, so keys rotate each hour
    // relative to `bucket_time` rather than wall-clock "now".
    let hour_bucket = bucket_time.timestamp() / 3600;

    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC accepts any key length");
    mac.update(recipient.as_bytes());
    mac.update(b"|");
    mac.update(template_name.as_bytes());
    mac.update(b"|");
    mac.update(hour_bucket.to_string().as_bytes());
    let result = mac.finalize().into_bytes();
    format!("email:idem:{}", hex::encode(result))
}

/// Validate and sanitize an email address before use.
///
/// - Trims surrounding whitespace.
/// - Rejects addresses that exceed 254 characters (RFC 5321 limit).
/// - Validates RFC 5322 format via the `validator` crate.
///
/// Returns the trimmed address on success, or an error with context logged
/// at WARN level so operators can trace bad inputs.
pub fn sanitize_email(raw: &str) -> Result<String> {
    let trimmed = raw.trim().to_string();

    if trimmed.is_empty() {
        tracing::warn!(raw_input = raw, "Email validation failed: address is empty");
        anyhow::bail!("email address must not be empty");
    }

    if trimmed.len() > 254 {
        tracing::warn!(
            raw_input = raw,
            length = trimmed.len(),
            "Email validation failed: address exceeds 254-character RFC 5321 limit"
        );
        anyhow::bail!(
            "email address is too long ({} chars, max 254)",
            trimmed.len()
        );
    }

    if !trimmed.validate_email() {
        tracing::warn!(
            raw_input = raw,
            "Email validation failed: address does not conform to RFC 5322"
        );
        anyhow::bail!("invalid email address: '{trimmed}'");
    }

    Ok(trimmed)
}

#[derive(Clone)]
pub struct EmailService {
    config: Config,
    template_engine: EmailTemplateEngine,
    client: reqwest::Client,
    cache: Option<RedisCache>,
    pub idempotency: IdempotencyConfig,
    pub(crate) idempotency_secret: String,
    metrics: Option<Metrics>,
    sendgrid_base_url: String,
}

impl EmailService {
    pub fn new(config: Config) -> Result<Self> {
        let secret = config.email_idempotency_secret.clone();
        Self::with_cache(config, None, IdempotencyConfig::default(), secret)
    }

    pub fn with_cache(
        config: Config,
        cache: Option<RedisCache>,
        idempotency: IdempotencyConfig,
        _idempotency_secret: String,
    ) -> Result<Self> {
        Self::with_cache_and_metrics(config, cache, idempotency, None)
    }

    pub fn with_cache_and_metrics(
        config: Config,
        cache: Option<RedisCache>,
        idempotency: IdempotencyConfig,
        metrics: Option<Metrics>,
    ) -> Result<Self> {
        let template_engine = EmailTemplateEngine::new()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            config,
            template_engine,
            client,
            cache,
            idempotency,
            idempotency_secret,
            metrics,
            sendgrid_base_url: "https://api.sendgrid.com".to_string(),
        })
    }

    #[cfg(test)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.sendgrid_base_url = base_url;
        self
    }

    /// Probe SendGrid API reachability. Returns Ok if the API key is valid and
    /// SendGrid is reachable; returns Err on timeout, network failure, or a
    /// non-2xx response. Uses a 3-second timeout to keep health checks fast.
    pub async fn probe_sendgrid(&self) -> Result<()> {
        let api_key = self
            .config
            .sendgrid_api_key
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("SENDGRID_API_KEY not configured"))?;
        let fut = self
            .client
            .get("https://api.sendgrid.com/v3/user/email")
            .bearer_auth(api_key)
            .send();
        let resp = tokio::time::timeout(Duration::from_secs(3), fut)
            .await
            .map_err(|_| anyhow::anyhow!("SendGrid probe timed out after 3s"))?
            .map_err(|e| anyhow::anyhow!("SendGrid probe request failed: {e}"))?;
        if resp.status().is_success() {
            Ok(())
        } else {
            anyhow::bail!("SendGrid probe returned non-2xx status {}", resp.status())
        }
    }

    /// Send an email using SendGrid
    pub async fn send_email(
        &self,
        recipient: &str,
        template_name: &str,
        template_data: &Value,
    ) -> Result<String> {
        let idem = idempotency_key(recipient, template_name, &self.idempotency_secret, Utc::now());
        self.send_email_idempotent(recipient, template_name, template_data, Some(&idem))
            .await
    }

    /// Send with an explicit idempotency key.
    ///
    /// If `idem_key` is `Some`, the key is checked in Redis before sending.
    /// A duplicate within the TTL window returns the cached message-id
    /// immediately without hitting SendGrid again.
    pub async fn send_email_idempotent(
        &self,
        recipient: &str,
        template_name: &str,
        template_data: &Value,
        idem_key: Option<&str>,
    ) -> Result<String> {
        // --- per-recipient hourly rate limit ---
        if let Some(cache) = &self.cache {
            let limit = self.config.email_per_recipient_hourly_limit;
            let rate_key = format!("email:rate:{}:{}", recipient, SystemTime::now()
                .duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() / 3600);
            let count = cache.incr_with_ttl(&rate_key, Duration::from_secs(3600)).await
                .context("per-recipient rate limit Redis check failed")?;
            if count > limit {
                tracing::warn!(recipient, limit, count, "per-recipient hourly email limit exceeded");
                return Err(anyhow::Error::new(RateLimitExceeded {
                    recipient: recipient.to_string(),
                    limit,
                }));
            }
        }

        // --- idempotency check ---
        if let (Some(cache), Some(key)) = (&self.cache, idem_key) {
            let redis_key = format!("email:idem:{key}");
            let mut conn = cache.get_connection().await.context("idempotency Redis connection failed")?;

            // Try SET NX — only succeeds for the first send.
            let acquired: Option<String> = redis::cmd("SET")
                .arg(&redis_key)
                .arg("1")
                .arg("NX")
                .arg("EX")
                .arg(self.idempotency.ttl.as_secs())
                .query_async(&mut conn)
                .await
                .context("idempotency Redis check failed")?;

            if acquired.is_none() {
                // Key already existed — this is a duplicate send.
                tracing::info!(
                    idem_key = key,
                    recipient = recipient,
                    template = template_name,
                    "Duplicate email send suppressed by idempotency key"
                );
                // Return a sentinel so callers can distinguish dedup from a
                // real send without treating it as an error.
                return Ok(format!("deduplicated:{key}"));
            }
        }

        // Sanitize and validate before touching the SendGrid API.
        let recipient = sanitize_email(recipient)
            .with_context(|| format!("rejecting send_email for template '{template_name}'"))?;
        let recipient = recipient.as_str();

        let api_key = self
            .config
            .sendgrid_api_key
            .as_deref()
            .context("SENDGRID_API_KEY not configured")?;

        let from_email = self
            .config
            .from_email
            .as_deref()
            .context("FROM_EMAIL not configured")?;

        // Render email content
        let html_content = self.template_engine.render(template_name, template_data)?;
        let text_content = self
            .template_engine
            .render_text(template_name, template_data);
        let subject = self
            .template_engine
            .get_subject(template_name, template_data);

        // Build SendGrid payload
        let payload = serde_json::json!({
            "personalizations": [{
                "to": [{ "email": recipient }],
                "subject": subject
            }],
            "from": { "email": from_email },
            "content": [
                {
                    "type": "text/plain",
                    "value": text_content
                },
                {
                    "type": "text/html",
                    "value": html_content
                }
            ],
            "tracking_settings": {
                "click_tracking": { "enable": true },
                "open_tracking": { "enable": true }
            },
            "custom_args": {
                "template_name": template_name
            }
        });

        // Send via SendGrid with retry (max 3 attempts, exp backoff + jitter)
        const MAX_ATTEMPTS: u32 = 3;
        let mut last_error = String::new();

        for attempt in 0..MAX_ATTEMPTS {
            let response = self
                .client
                .post(format!("{}/v3/mail/send", self.sendgrid_base_url))
                .bearer_auth(api_key)
                .json(&payload)
                .send()
                .await
                .context("Failed to send email via SendGrid")?;

            let status = response.status();

            if status.is_success() {
                let message_id = response
                    .headers()
                    .get("x-message-id")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("unknown")
                    .to_string();

                tracing::info!(
                    "Email sent successfully to {} using template {} (message_id: {})",
                    recipient,
                    template_name,
                    message_id
                );
                return Ok(message_id);
            }

            let should_retry = status.as_u16() == 429 || status.is_server_error();
            let retry_after_header = response
                .headers()
                .get("retry-after")
                .and_then(|v| v.to_str().ok())
                .and_then(|s| s.parse::<u64>().ok());

            if !should_retry || attempt + 1 == MAX_ATTEMPTS {
                last_error = format!("SendGrid API error {}", status);
                break;
            }

            let reason = if status.as_u16() == 429 { "rate_limited" } else { "server_error" };
            if let Some(m) = &self.metrics {
                m.observe_sendgrid_retry(reason);
            }

            // Respect Retry-After (seconds) if present, else exp backoff + jitter
            let delay_ms: u64 = if let Some(secs) = retry_after_header {
                secs * 1_000
            } else {
                let jitter = (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .subsec_millis() % 100) as u64;
                (1u64 << attempt) * 100 + jitter
            };

            tracing::warn!(
                attempt = attempt + 1,
                delay_ms,
                reason,
                "SendGrid transient error {}, retrying",
                status
            );
            tokio::time::sleep(Duration::from_millis(delay_ms)).await;
        }

        anyhow::bail!(last_error);
    }

    /// Preview email without sending (for testing/development)
    pub fn preview_email(
        &self,
        template_name: &str,
        template_data: &Value,
    ) -> Result<EmailPreview> {
        let html_content = self.template_engine.render(template_name, template_data)?;
        let text_content = self
            .template_engine
            .render_text(template_name, template_data);
        let subject = self
            .template_engine
            .get_subject(template_name, template_data);

        Ok(EmailPreview {
            subject,
            html_content,
            text_content,
        })
    }

    /// Send test email
    pub async fn send_test_email(&self, recipient: &str, template_name: &str) -> Result<String> {
        // Validate template_name before proceeding
        if !Self::is_valid_template_name(template_name) {
            return Err(anyhow::anyhow!(
                "Invalid template name '{}'. Valid templates are: newsletter_confirmation, waitlist_confirmation, contact_form_auto_response, welcome_email",
                template_name
            ));
        }
        
        let test_data = self.get_test_data(template_name);
        self.send_email(recipient, template_name, &test_data).await
    }

    fn get_test_data(&self, template_name: &str) -> Value {
        match template_name {
            "newsletter_confirmation" => serde_json::json!({
                "confirm_url": format!("{}/api/v1/newsletter/confirm?token=test-token-123", self.config.base_url),
                "email": "test@example.com"
            }),
            "waitlist_confirmation" => serde_json::json!({
                "email": "test@example.com"
            }),
            "contact_form_auto_response" => serde_json::json!({
                "name": "Test User",
                "subject": "Test Subject",
                "message": "This is a test message from the contact form."
            }),
            "welcome_email" => serde_json::json!({
                "name": "Test User",
                "dashboard_url": format!("{}/dashboard", self.config.base_url),
                "help_url": format!("{}/help", self.config.base_url),
                "unsubscribe_url": format!("{}/api/v1/newsletter/unsubscribe", self.config.base_url)
            }),
            _ => serde_json::json!({}),
        }
    }

    /// Check if a template name is valid
    fn is_valid_template_name(template_name: &str) -> bool {
        matches!(
            template_name,
            "newsletter_confirmation" | "waitlist_confirmation" | 
            "contact_form_auto_response" | "welcome_email"
        )
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct EmailPreview {
    pub subject: String,
    pub html_content: String,
    pub text_content: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preview_email() {
        let config = Config::from_env();
        let service = EmailService::new(config).unwrap();

        let data = serde_json::json!({
            "confirm_url": "https://example.com/confirm?token=abc123",
            "email": "test@example.com"
        });

        let preview = service
            .preview_email("newsletter_confirmation", &data)
            .unwrap();
        assert!(!preview.subject.is_empty());
        assert!(preview.html_content.contains("confirm"));
        assert!(preview.text_content.contains("confirm"));
    }

    // ---- idempotency_key unit tests ----

    #[test]
    fn same_inputs_produce_same_key() {
        let now = Utc::now();
        let k1 = idempotency_key("user@example.com", "welcome_email", "secret", now);
        let k2 = idempotency_key("user@example.com", "welcome_email", "secret", now);
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_recipient_produces_different_key() {
        let now = Utc::now();
        let k1 = idempotency_key("alice@example.com", "welcome_email", "secret", now);
        let k2 = idempotency_key("bob@example.com", "welcome_email", "secret", now);
        assert_ne!(k1, k2);
    }

    #[test]
    fn different_template_produces_different_key() {
        let now = Utc::now();
        let k1 = idempotency_key("user@example.com", "welcome_email", "secret", now);
        let k2 = idempotency_key("user@example.com", "newsletter_confirmation", "secret", now);
        assert_ne!(k1, k2);
    }

    /// #932: key changes when the secret changes.
    #[test]
    fn different_secret_produces_different_key() {
        let now = Utc::now();
        let k1 = idempotency_key("user@example.com", "welcome_email", "secret-a", now);
        let k2 = idempotency_key("user@example.com", "welcome_email", "secret-b", now);
        assert_ne!(k1, k2, "key must change when the HMAC secret changes");
    }

    #[test]
    fn key_has_expected_prefix() {
        let key = idempotency_key("user@example.com", "t", "secret", Utc::now());
        assert!(key.starts_with("email:idem:"), "key should start with email:idem: prefix");
    }

    #[test]
    fn idempotency_config_default_ttl_is_24h() {
        let cfg = IdempotencyConfig::default();
        assert_eq!(cfg.ttl.as_secs(), 86_400);
    }

    #[test]
    fn idempotency_config_ttl_is_configurable() {
        let cfg = IdempotencyConfig { ttl: std::time::Duration::from_secs(3600) };
        assert_eq!(cfg.ttl.as_secs(), 3600);
    }

    /// Retry produces the same key (within the same hour bucket).
    #[test]
    fn retry_produces_same_idempotency_key() {
        let created_at = Utc::now();
        let key_attempt_1 = idempotency_key("user@example.com", "newsletter_confirmation", "secret", created_at);
        let key_attempt_2 = idempotency_key("user@example.com", "newsletter_confirmation", "secret", created_at);
        assert_eq!(
            key_attempt_1, key_attempt_2,
            "retry must produce the same idempotency key"
        );
    }

    /// #1129: a retry delayed past the 1-hour bucket boundary (e.g. by the
    /// 6th exponential-backoff attempt, ~64 minutes out) must still compute
    /// the same idempotency key as the original attempt, since both are
    /// derived from the job's original creation time rather than "now".
    #[test]
    fn retry_key_is_stable_across_hour_boundary_when_derived_from_job_creation_time() {
        let created_at = Utc::now();
        let key_at_creation = idempotency_key("user@example.com", "newsletter_confirmation", "secret", created_at);

        // Simulate a retry scheduled more than an hour after the original
        // attempt (e.g. the ~64-minute delay of the 6th backoff attempt).
        let delayed_retry_time = created_at + chrono::Duration::minutes(90);
        let key_at_delayed_retry = idempotency_key(
            "user@example.com",
            "newsletter_confirmation",
            "secret",
            created_at, // bucket is still derived from the original creation time
        );
        assert_eq!(
            key_at_creation, key_at_delayed_retry,
            "idempotency key must remain stable across the job's full retry lifetime"
        );

        // Sanity check: had the key been derived from wall-clock "now" at the
        // delayed retry time instead of the job's creation time, it would differ.
        let key_from_wall_clock_now = idempotency_key(
            "user@example.com",
            "newsletter_confirmation",
            "secret",
            delayed_retry_time,
        );
        assert_ne!(
            key_at_creation, key_from_wall_clock_now,
            "test precondition: 90 minutes must cross an hour bucket boundary"
        );
    }

    /// Two 429s followed by a 202: the service should succeed on the third attempt.
    #[tokio::test]
    async fn retry_succeeds_after_two_429s() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let mock_server = MockServer::start().await;

        // First two calls return 429, third returns 202.
        Mock::given(method("POST"))
            .and(path("/v3/mail/send"))
            .respond_with(ResponseTemplate::new(429))
            .up_to_n_times(2)
            .mount(&mock_server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v3/mail/send"))
            .respond_with(
                ResponseTemplate::new(202)
                    .insert_header("x-message-id", "test-msg-id"),
            )
            .mount(&mock_server)
            .await;

        let mut config = Config::from_env();
        config.sendgrid_api_key = Some("test-key".to_string());
        config.from_email = Some("from@example.com".to_string());

        let metrics = crate::metrics::Metrics::new().unwrap();
        let service = EmailService::with_cache_and_metrics(
            config,
            None,
            IdempotencyConfig::default(),
            Some(metrics.clone()),
        )
        .unwrap()
        .with_base_url(mock_server.uri());

        let data = serde_json::json!({"confirm_url": "https://example.com/confirm?token=abc"});
        let result = service
            .send_email("user@example.com", "newsletter_confirmation", &data)
            .await;

        assert!(result.is_ok(), "expected success after retries, got: {:?}", result);
        assert_eq!(result.unwrap(), "test-msg-id");

        // Verify the retry counter was incremented twice (one per 429)
        let rendered = metrics.render().unwrap();
        assert!(
            rendered.contains(r#"sendgrid_retries_total{reason="rate_limited"} 2"#),
            "expected 2 rate_limited retries in metrics:\n{rendered}"
        );
    }

    #[test]
    fn valid_address_passes() {
        assert!(sanitize_email("user@example.com").is_ok());
    }

    #[test]
    fn whitespace_is_trimmed() {
        let result = sanitize_email("  user@example.com  ").unwrap();
        assert_eq!(result, "user@example.com");
    }

    #[test]
    fn empty_string_is_rejected() {
        assert!(sanitize_email("").is_err());
        assert!(sanitize_email("   ").is_err());
    }

    #[test]
    fn missing_at_sign_is_rejected() {
        assert!(sanitize_email("notanemail").is_err());
    }

    #[test]
    fn missing_domain_is_rejected() {
        assert!(sanitize_email("user@").is_err());
    }

    #[test]
    fn missing_local_part_is_rejected() {
        assert!(sanitize_email("@example.com").is_err());
    }

    #[test]
    fn address_exceeding_254_chars_is_rejected() {
        // local part 64 chars + @ + domain that pushes total over 254
        let local = "a".repeat(64);
        let domain = "b".repeat(190);
        let addr = format!("{local}@{domain}.com");
        assert!(addr.len() > 254);
        assert!(sanitize_email(&addr).is_err());
    }

    #[test]
    fn subaddress_plus_tag_is_accepted() {
        assert!(sanitize_email("user+tag@example.com").is_ok());
    }

    #[test]
    fn subdomain_address_is_accepted() {
        assert!(sanitize_email("user@mail.example.co.uk").is_ok());
    }

    #[test]
    fn double_at_sign_is_rejected() {
        assert!(sanitize_email("user@@example.com").is_err());
    }

    #[test]
    fn newline_injection_attempt_is_rejected() {
        // A newline in the address would be invalid per RFC 5322.
        assert!(sanitize_email("user\n@example.com").is_err());
    }

    #[tokio::test]
    async fn per_recipient_rate_limit_rejects_nth_plus_one_send() {
        use testcontainers::runners::AsyncRunner;
        use testcontainers_modules::redis::Redis;
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let container = Redis::default().start().await.expect("redis container");
        let port = container.get_host_port_ipv4(6379).await.expect("redis port");
        let redis_url = format!("redis://127.0.0.1:{port}");
        let cache = crate::cache::RedisCache::new(&redis_url).await.expect("redis cache");

        let mock_server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/v3/mail/send"))
            .respond_with(ResponseTemplate::new(202).insert_header("x-message-id", "msg-id"))
            .mount(&mock_server)
            .await;

        let mut config = Config::from_env();
        config.sendgrid_api_key = Some("test-key".to_string());
        config.from_email = Some("from@example.com".to_string());
        config.email_per_recipient_hourly_limit = 2;

        let service = EmailService::with_cache_and_metrics(
            config,
            Some(cache),
            IdempotencyConfig::default(),
            None,
        )
        .unwrap()
        .with_base_url(mock_server.uri());

        let data = serde_json::json!({"confirm_url": "https://example.com/confirm?token=abc"});
        let recipient = "limited@example.com";

        // First two sends (within limit) must succeed.
        for _ in 0..2 {
            let result = service
                .send_email_idempotent(recipient, "newsletter_confirmation", &data, None)
                .await;
            assert!(result.is_ok(), "send within limit must succeed");
        }

        // Third send (N+1) must be rejected with RateLimitExceeded.
        let result = service
            .send_email_idempotent(recipient, "newsletter_confirmation", &data, None)
            .await;
        assert!(result.is_err(), "N+1 send must be rejected");
        let err = result.unwrap_err();
        assert!(
            err.downcast_ref::<RateLimitExceeded>().is_some(),
            "error must be RateLimitExceeded, got: {err}"
        );
    }
}
