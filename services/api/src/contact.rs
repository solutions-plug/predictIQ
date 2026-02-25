use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::ValidateEmail;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContactSubmission {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub subject: String,
    pub message: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub recaptcha_score: Option<f64>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContactFormRequest {
    pub name: String,
    pub email: String,
    pub subject: String,
    pub message: String,
    #[serde(rename = "recaptchaToken")]
    pub recaptcha_token: String,
    #[serde(default)]
    pub honeypot: String, // Honeypot field for spam protection
}

#[derive(Debug, Clone, Serialize)]
pub struct ContactFormResponse {
    pub success: bool,
    pub message: String,
    pub submission_id: Option<Uuid>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecaptchaVerifyResponse {
    pub success: bool,
    pub score: Option<f64>,
    pub action: Option<String>,
    #[serde(rename = "challenge_ts")]
    pub challenge_ts: Option<String>,
    pub hostname: Option<String>,
    #[serde(rename = "error-codes")]
    pub error_codes: Option<Vec<String>>,
}

impl ContactFormRequest {
    pub fn validate(&self) -> Result<(), String> {
        // Validate name
        let name_len = self.name.trim().len();
        if name_len < 2 || name_len > 100 {
            return Err("Name must be between 2 and 100 characters".to_string());
        }

        // Validate email
        if !validator::ValidateEmail::validate_email(&self.email.trim().to_lowercase()) {
            return Err("Invalid email format".to_string());
        }

        // Validate subject
        if self.subject.trim().is_empty() {
            return Err("Subject is required".to_string());
        }

        // Validate message
        let message_len = self.message.trim().len();
        if message_len < 10 || message_len > 1000 {
            return Err("Message must be between 10 and 1000 characters".to_string());
        }

        // Validate recaptcha token
        if self.recaptcha_token.trim().is_empty() {
            return Err("reCAPTCHA token is required".to_string());
        }

        // Check honeypot (should be empty)
        if !self.honeypot.is_empty() {
            return Err("Spam detected".to_string());
        }

        Ok(())
    }
}
