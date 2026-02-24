use anyhow::{Context, Result};
use handlebars::Handlebars;
use serde_json::Value;

#[derive(Clone)]
pub struct EmailTemplateEngine {
    handlebars: Handlebars<'static>,
}

impl EmailTemplateEngine {
    pub fn new() -> Result<Self> {
        let mut handlebars = Handlebars::new();
        handlebars.set_strict_mode(true);

        // Register built-in templates
        handlebars.register_template_string(
            "newsletter_confirmation",
            include_str!("../../templates/newsletter_confirmation.html"),
        )?;

        handlebars.register_template_string(
            "waitlist_confirmation",
            include_str!("../../templates/waitlist_confirmation.html"),
        )?;

        handlebars.register_template_string(
            "contact_form_auto_response",
            include_str!("../../templates/contact_form_auto_response.html"),
        )?;

        handlebars.register_template_string(
            "welcome_email",
            include_str!("../../templates/welcome_email.html"),
        )?;

        Ok(Self { handlebars })
    }

    pub fn render(&self, template_name: &str, data: &Value) -> Result<String> {
        self.handlebars
            .render(template_name, data)
            .with_context(|| format!("Failed to render template: {}", template_name))
    }

    pub fn get_subject(&self, template_name: &str, data: &Value) -> String {
        match template_name {
            "newsletter_confirmation" => "Confirm your newsletter subscription".to_string(),
            "waitlist_confirmation" => "You're on the waitlist!".to_string(),
            "contact_form_auto_response" => {
                format!(
                    "We received your message: {}",
                    data.get("subject")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Your inquiry")
                )
            }
            "welcome_email" => "Welcome to PredictIQ!".to_string(),
            _ => "Message from PredictIQ".to_string(),
        }
    }

    pub fn render_text(&self, template_name: &str, data: &Value) -> String {
        // Simplified text version for email clients that don't support HTML
        match template_name {
            "newsletter_confirmation" => {
                let confirm_url = data
                    .get("confirm_url")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                format!(
                    "Please confirm your newsletter subscription by visiting: {}\n\nIf you didn't request this, please ignore this email.",
                    confirm_url
                )
            }
            "waitlist_confirmation" => {
                format!(
                    "Thank you for joining the PredictIQ waitlist!\n\nWe'll notify you at {} when we're ready to launch.\n\nStay tuned!",
                    data.get("email").and_then(|v| v.as_str()).unwrap_or("")
                )
            }
            "contact_form_auto_response" => {
                format!(
                    "Thank you for contacting PredictIQ!\n\nWe've received your message and will get back to you soon.\n\nYour message:\n{}\n\nBest regards,\nThe PredictIQ Team",
                    data.get("message").and_then(|v| v.as_str()).unwrap_or("")
                )
            }
            "welcome_email" => {
                format!(
                    "Welcome to PredictIQ!\n\nWe're excited to have you on board. Get started by exploring our prediction markets.\n\nBest regards,\nThe PredictIQ Team"
                )
            }
            _ => "Message from PredictIQ".to_string(),
        }
    }
}

impl Default for EmailTemplateEngine {
    fn default() -> Self {
        Self::new().expect("Failed to initialize email template engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_newsletter_confirmation() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({
            "confirm_url": "https://example.com/confirm?token=abc123",
            "email": "test@example.com"
        });

        let result = engine.render("newsletter_confirmation", &data);
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("confirm"));
        assert!(html.contains("abc123"));
    }

    #[test]
    fn test_get_subject() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({});

        let subject = engine.get_subject("newsletter_confirmation", &data);
        assert_eq!(subject, "Confirm your newsletter subscription");
    }
}
