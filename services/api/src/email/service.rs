use anyhow::{Context, Result};
use serde_json::Value;

use crate::config::Config;
use crate::email::templates::EmailTemplateEngine;

#[derive(Clone)]
pub struct EmailService {
    config: Config,
    template_engine: EmailTemplateEngine,
    client: reqwest::Client,
}

impl EmailService {
    pub fn new(config: Config) -> Result<Self> {
        let template_engine = EmailTemplateEngine::new()?;
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()?;

        Ok(Self {
            config,
            template_engine,
            client,
        })
    }

    /// Send an email using SendGrid
    pub async fn send_email(
        &self,
        recipient: &str,
        template_name: &str,
        template_data: &Value,
    ) -> Result<String> {
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
        let text_content = self.template_engine.render_text(template_name, template_data);
        let subject = self.template_engine.get_subject(template_name, template_data);

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
    pub fn preview_email(&self, template_name: &str, template_data: &Value) -> Result<EmailPreview> {
        let html_content = self.template_engine.render(template_name, template_data)?;
        let text_content = self.template_engine.render_text(template_name, template_data);
        let subject = self.template_engine.get_subject(template_name, template_data);

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

        let preview = service.preview_email("newsletter_confirmation", &data).unwrap();
        assert!(!preview.subject.is_empty());
        assert!(preview.html_content.contains("confirm"));
        assert!(preview.text_content.contains("confirm"));
    }
}
