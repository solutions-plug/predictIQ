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

        let engine = Self { handlebars };

        // Validate all templates at startup by rendering with representative data.
        // This catches missing/misspelled variable references before the first send.
        engine.validate_all_templates()?;

        Ok(engine)
    }

    /// Render each registered template with representative data to catch syntax
    /// errors and missing variable references at startup rather than at send time.
    fn validate_all_templates(&self) -> Result<()> {
        let fixtures: &[(&str, Value)] = &[
            ("newsletter_confirmation", serde_json::json!({
                "confirm_url": "https://example.com/confirm?token=startup-check",
                "email": "startup@example.com"
            })),
            ("waitlist_confirmation", serde_json::json!({
                "email": "startup@example.com"
            })),
            ("contact_form_auto_response", serde_json::json!({
                "name": "Startup Check",
                "subject": "Startup Check",
                "message": "Startup validation render."
            })),
            ("welcome_email", serde_json::json!({
                "name": "Startup Check",
                "dashboard_url": "https://example.com/dashboard",
                "help_url": "https://example.com/help",
                "unsubscribe_url": "https://example.com/unsubscribe"
            })),
        ];

        for (name, data) in fixtures {
            self.handlebars
                .render(name, data)
                .with_context(|| format!("Template validation failed for '{name}': invalid syntax or missing variable"))?;
        }

        Ok(())
    }

    pub fn render(&self, template_name: &str, data: &Value) -> Result<String> {
        // Guard: reject oversized context data before allocating in the renderer.
        // A malicious or buggy caller could pass a multi-MB JSON object; serialising
        // it inside Handlebars would cause excessive memory allocation.
        const MAX_CONTEXT_BYTES: usize = 64 * 1024; // 64 KB
        let serialized_len = data.to_string().len();
        if serialized_len > MAX_CONTEXT_BYTES {
            anyhow::bail!(
                "template context for '{}' exceeds the 64 KB limit ({} bytes)",
                template_name,
                serialized_len
            );
        }

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

    // ── Context size limit tests (Issue 3) ────────────────────────────────────

    #[test]
    fn oversized_context_is_rejected() {
        let engine = EmailTemplateEngine::new().unwrap();
        // Build a context that exceeds 64 KB when serialised.
        let big_string = "x".repeat(70_000);
        let data = json!({
            "confirm_url": "https://example.com/confirm",
            "email": big_string
        });
        let err = engine.render("newsletter_confirmation", &data).unwrap_err();
        assert!(
            err.to_string().contains("64 KB limit"),
            "error should mention the 64 KB limit, got: {err}"
        );
    }

    #[test]
    fn context_at_limit_boundary_is_accepted() {
        let engine = EmailTemplateEngine::new().unwrap();
        // Build a context just under 64 KB.
        let padding = "a".repeat(60_000);
        let data = json!({
            "confirm_url": "https://example.com/confirm?token=abc",
            "email": padding
        });
        // Should not return a size error (may fail rendering due to the raw value
        // appearing in the template, but we just want the size check to pass).
        let result = engine.render("newsletter_confirmation", &data);
        // Size check passes — it may succeed or fail for other reasons, but NOT
        // because of the 64 KB limit.
        if let Err(ref e) = result {
            assert!(
                !e.to_string().contains("64 KB limit"),
                "should not hit size limit at {}, err: {e}",
                serde_json::to_string(&data).unwrap().len()
            );
        }
    }

    // ── Boundary-value tests per template (Issue 4) ───────────────────────────

    // newsletter_confirmation

    #[test]
    fn newsletter_confirmation_empty_strings() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({ "confirm_url": "", "email": "" });
        // Strict mode is on; empty strings are valid values — should render.
        assert!(engine.render("newsletter_confirmation", &data).is_ok());
    }

    #[test]
    fn newsletter_confirmation_special_chars() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({
            "confirm_url": "https://example.com/confirm?token=<script>alert(1)</script>",
            "email": "user+tag@example.com"
        });
        let html = engine.render("newsletter_confirmation", &data).unwrap();
        // Handlebars HTML-escapes by default; angle brackets must be escaped.
        assert!(!html.contains("<script>"), "XSS payload must be escaped");
    }

    #[test]
    fn newsletter_confirmation_long_strings() {
        let engine = EmailTemplateEngine::new().unwrap();
        let long_url = format!("https://example.com/confirm?token={}", "a".repeat(2000));
        let data = json!({ "confirm_url": long_url, "email": "user@example.com" });
        assert!(engine.render("newsletter_confirmation", &data).is_ok());
    }

    // waitlist_confirmation

    #[test]
    fn waitlist_confirmation_empty_email() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({ "email": "" });
        assert!(engine.render("waitlist_confirmation", &data).is_ok());
    }

    #[test]
    fn waitlist_confirmation_special_chars_in_email() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({ "email": "user+test&special=<chars>@example.com" });
        let html = engine.render("waitlist_confirmation", &data).unwrap();
        assert!(!html.contains("<chars>"), "angle brackets must be HTML-escaped");
    }

    #[test]
    fn waitlist_confirmation_long_email() {
        let engine = EmailTemplateEngine::new().unwrap();
        let local = "a".repeat(64);
        let data = json!({ "email": format!("{local}@example.com") });
        assert!(engine.render("waitlist_confirmation", &data).is_ok());
    }

    // contact_form_auto_response

    #[test]
    fn contact_form_empty_fields() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({ "name": "", "subject": "", "message": "" });
        assert!(engine.render("contact_form_auto_response", &data).is_ok());
    }

    #[test]
    fn contact_form_special_chars() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({
            "name": "<script>alert('xss')</script>",
            "subject": "Re: Test & <HTML>",
            "message": "Hello \"world\" & <world>"
        });
        let html = engine.render("contact_form_auto_response", &data).unwrap();
        assert!(!html.contains("<script>"), "script tags must be escaped");
        assert!(!html.contains("<HTML>"), "angle brackets must be escaped");
    }

    #[test]
    fn contact_form_long_message() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({
            "name": "Test User",
            "subject": "Long message test",
            "message": "a".repeat(5000)
        });
        assert!(engine.render("contact_form_auto_response", &data).is_ok());
    }

    // welcome_email

    #[test]
    fn welcome_email_empty_strings() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({
            "name": "",
            "dashboard_url": "",
            "help_url": "",
            "unsubscribe_url": ""
        });
        assert!(engine.render("welcome_email", &data).is_ok());
    }

    #[test]
    fn welcome_email_special_chars_in_name() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({
            "name": "O'Brien & <Test>",
            "dashboard_url": "https://example.com/dashboard",
            "help_url": "https://example.com/help",
            "unsubscribe_url": "https://example.com/unsubscribe"
        });
        let html = engine.render("welcome_email", &data).unwrap();
        assert!(!html.contains("<Test>"), "angle brackets must be escaped");
    }

    #[test]
    fn welcome_email_long_name() {
        let engine = EmailTemplateEngine::new().unwrap();
        let data = json!({
            "name": "A".repeat(200),
            "dashboard_url": "https://example.com/dashboard",
            "help_url": "https://example.com/help",
            "unsubscribe_url": "https://example.com/unsubscribe"
        });
        assert!(engine.render("welcome_email", &data).is_ok());
    }

    // ── Startup validation sanity check ──────────────────────────────────────

    #[test]
    fn engine_init_validates_all_templates_at_startup() {
        // If any template has a syntax error or missing variable, new() will fail.
        assert!(
            EmailTemplateEngine::new().is_ok(),
            "All templates must be valid at startup"
        );
    }
}
