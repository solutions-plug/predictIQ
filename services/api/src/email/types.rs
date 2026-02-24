use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailJobType {
    NewsletterConfirmation,
    WaitlistConfirmation,
    ContactFormAutoResponse,
    WelcomeEmail,
    Custom(String),
}

impl EmailJobType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::NewsletterConfirmation => "newsletter_confirmation",
            Self::WaitlistConfirmation => "waitlist_confirmation",
            Self::ContactFormAutoResponse => "contact_form_auto_response",
            Self::WelcomeEmail => "welcome_email",
            Self::Custom(s) => s,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailJobStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Cancelled,
}

impl EmailJobStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Pending => "pending",
            Self::Processing => "processing",
            Self::Completed => "completed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EmailEventType {
    Sent,
    Delivered,
    Opened,
    Clicked,
    Bounced,
    Complained,
    Unsubscribed,
}

impl EmailEventType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Sent => "sent",
            Self::Delivered => "delivered",
            Self::Opened => "opened",
            Self::Clicked => "clicked",
            Self::Bounced => "bounced",
            Self::Complained => "complained",
            Self::Unsubscribed => "unsubscribed",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SuppressionType {
    Bounce,
    Complaint,
    Unsubscribe,
    Manual,
}

impl SuppressionType {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Bounce => "bounce",
            Self::Complaint => "complaint",
            Self::Unsubscribe => "unsubscribe",
            Self::Manual => "manual",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailJob {
    pub id: Uuid,
    pub job_type: String,
    pub recipient_email: String,
    pub template_name: String,
    pub template_data: serde_json::Value,
    pub status: String,
    pub priority: i32,
    pub attempts: i32,
    pub max_attempts: i32,
    pub scheduled_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub failed_at: Option<DateTime<Utc>>,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailEvent {
    pub id: Uuid,
    pub email_job_id: Option<Uuid>,
    pub message_id: Option<String>,
    pub event_type: String,
    pub recipient_email: String,
    pub timestamp: DateTime<Utc>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailSuppression {
    pub id: Uuid,
    pub email: String,
    pub suppression_type: String,
    pub reason: Option<String>,
    pub bounce_type: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailAnalytics {
    pub template_name: String,
    pub variant_name: Option<String>,
    pub date: chrono::NaiveDate,
    pub sent_count: i32,
    pub delivered_count: i32,
    pub opened_count: i32,
    pub clicked_count: i32,
    pub bounced_count: i32,
    pub complained_count: i32,
    pub unsubscribed_count: i32,
}
