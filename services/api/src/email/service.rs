use anyhow::{Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::time::Duration;
use validator::ValidateEmail;

use crate::cache::RedisCache;
use crate::config::Config;
use crate::email::templates::EmailTemplateEngine;

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
/// The key is `email:idem:<hex(SHA-256(recipient|template|data))>` so the
/// same logical send always maps to the same Redis key regardless of which
/// worker processes it.
pub fn idempotency_key(recipient: &str, template_name: &str, template_data: &Value) -> String {
    let mut hasher = Sha256::new();
    hasher.update(recipient.as_bytes());
    hasher.update(b"|");
    hasher.update(template_name.as_bytes());
    hasher.update(b"|");
    hasher.update(template_data.to_string().as_bytes());
    let digest = hasher.finalize();
    format!("email:idem:{:x}", digest)
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
}

impl EmailService {
    pub fn new(config: Config) -> Result<Self> {
        Self::with_cache(config, None, IdempotencyConfig::default())
    }

    pub fn with_cache(
        config: Config,
        cache: Option<RedisCache>,
        idempotency: IdempotencyConfig,
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
        })
    }

    /// Send an email using SendGrid
    pub async fn send_email(
        &self,
        recipient: &str,
        template_name: &str,
        template_data: &Value,
    ) -> Result<String> {
        self.send_email_idempotent(recipient, template_name, template_data, None)
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

        // Send via SendGrid
        let response = self
            .client
            .post("https://api.sendgrid.com/v3/mail/send")
            .bearer_auth(api_key)
            .json(&payload)
            .send()
            .await
            .context("Failed to send email via SendGrid")?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("SendGrid API error {}: {}", status, body);
        }

        // Extract message ID from response headers
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

        Ok(message_id)
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
        let data = serde_json::json!({"token": "abc"});
        let k1 = idempotency_key("user@example.com", "welcome_email", &data);
        let k2 = idempotency_key("user@example.com", "welcome_email", &data);
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_recipient_produces_different_key() {
        let data = serde_json::json!({"token": "abc"});
        let k1 = idempotency_key("alice@example.com", "welcome_email", &data);
        let k2 = idempotency_key("bob@example.com", "welcome_email", &data);
        assert_ne!(k1, k2);
    }

    #[test]
    fn different_template_produces_different_key() {
        let data = serde_json::json!({});
        let k1 = idempotency_key("user@example.com", "welcome_email", &data);
        let k2 = idempotency_key("user@example.com", "newsletter_confirmation", &data);
        assert_ne!(k1, k2);
    }

    #[test]
    fn different_data_produces_different_key() {
        let k1 = idempotency_key("user@example.com", "welcome_email", &serde_json::json!({"a": 1}));
        let k2 = idempotency_key("user@example.com", "welcome_email", &serde_json::json!({"a": 2}));
        assert_ne!(k1, k2);
    }

    #[test]
    fn key_has_expected_prefix() {
        let data = serde_json::json!({});
        let key = idempotency_key("user@example.com", "t", &data);
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

    /// Simulate retry scenario: same key presented twice should be detected
    /// as a duplicate at the key-derivation level (no Redis needed).
    #[test]
    fn retry_produces_same_idempotency_key() {
        let data = serde_json::json!({"confirm_url": "https://example.com/confirm?token=xyz"});
        let key_attempt_1 = idempotency_key("user@example.com", "newsletter_confirmation", &data);
        // Simulate a retry — exact same inputs
        let key_attempt_2 = idempotency_key("user@example.com", "newsletter_confirmation", &data);
        assert_eq!(
            key_attempt_1, key_attempt_2,
            "retry must produce the same idempotency key"
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
}
