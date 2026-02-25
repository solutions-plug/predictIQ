use anyhow::Result;
use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::Database;
use crate::email::types::SuppressionType;

/// SendGrid webhook event structure
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct SendGridEvent {
    pub email: String,
    pub event: String,
    pub timestamp: i64,
    #[serde(rename = "sg_message_id")]
    pub message_id: Option<String>,
    pub reason: Option<String>,
    pub status: Option<String>,
    pub response: Option<String>,
    #[serde(rename = "bounce_classification")]
    pub bounce_classification: Option<String>,
    pub url: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Value,
}

#[derive(Clone)]
pub struct WebhookHandler {
    db: Database,
}

impl WebhookHandler {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Process SendGrid webhook events
    pub async fn handle_sendgrid_webhook(
        &self,
        events: Vec<SendGridEvent>,
    ) -> Result<WebhookResponse> {
        let mut processed = 0;
        let mut errors = Vec::new();

        for event in events {
            match self.process_event(event.clone()).await {
                Ok(_) => processed += 1,
                Err(e) => {
                    tracing::error!("Error processing webhook event: {}", e);
                    errors.push(format!("Event {}: {}", event.event, e));
                }
            }
        }

        Ok(WebhookResponse {
            processed,
            errors: if errors.is_empty() {
                None
            } else {
                Some(errors)
            },
        })
    }

    async fn process_event(&self, event: SendGridEvent) -> Result<()> {
        let event_type = event.event.as_str();
        let email = event.email.as_str();
        let message_id = event.message_id.as_deref();

        tracing::info!(
            "Processing SendGrid event: {} for {} (message_id: {:?})",
            event_type,
            email,
            message_id
        );

        // Record event in database
        self.db
            .email_create_event(
                None,
                message_id,
                event_type,
                email,
                serde_json::to_value(&event)?,
            )
            .await?;

        // Handle specific event types
        match event_type {
            "delivered" => {
                // Update analytics
                self.db.email_increment_analytics_counter("delivered", None).await?;
            }
            "open" => {
                self.db.email_increment_analytics_counter("opened", None).await?;
            }
            "click" => {
                self.db.email_increment_analytics_counter("clicked", None).await?;
            }
            "bounce" => {
                self.handle_bounce(&event).await?;
            }
            "dropped" => {
                self.handle_bounce(&event).await?;
            }
            "spamreport" => {
                self.handle_complaint(&event).await?;
            }
            "unsubscribe" => {
                self.handle_unsubscribe(&event).await?;
            }
            _ => {
                tracing::debug!("Unhandled event type: {}", event_type);
            }
        }

        Ok(())
    }

    async fn handle_bounce(&self, event: &SendGridEvent) -> Result<()> {
        let bounce_type = event
            .bounce_classification
            .as_deref()
            .or(event.status.as_deref())
            .unwrap_or("unknown");

        let reason = event
            .reason
            .as_deref()
            .or(event.response.as_deref())
            .unwrap_or("No reason provided");

        // Add to suppression list
        self.db
            .email_add_suppression(
                &event.email,
                SuppressionType::Bounce.as_str(),
                Some(reason),
                Some(bounce_type),
            )
            .await?;

        // Update analytics
        self.db.email_increment_analytics_counter("bounced", None).await?;

        tracing::warn!(
            "Email bounced: {} (type: {}, reason: {})",
            event.email,
            bounce_type,
            reason
        );

        Ok(())
    }

    async fn handle_complaint(&self, event: &SendGridEvent) -> Result<()> {
        let reason = event.reason.as_deref().unwrap_or("Spam complaint");

        // Add to suppression list
        self.db
            .email_add_suppression(&event.email, SuppressionType::Complaint.as_str(), Some(reason), None)
            .await?;

        // Update analytics
        self.db.email_increment_analytics_counter("complained", None).await?;

        tracing::warn!("Spam complaint received for: {}", event.email);

        Ok(())
    }

    async fn handle_unsubscribe(&self, event: &SendGridEvent) -> Result<()> {
        // Add to suppression list
        self.db
            .email_add_suppression(
                &event.email,
                SuppressionType::Unsubscribe.as_str(),
                Some("User unsubscribed via email link"),
                None,
            )
            .await?;

        // Also unsubscribe from newsletter if applicable
        let _ = self.db.newsletter_unsubscribe(&event.email).await;

        // Update analytics
        self.db.email_increment_analytics_counter("unsubscribed", None).await?;

        tracing::info!("User unsubscribed: {}", event.email);

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub processed: usize,
    pub errors: Option<Vec<String>>,
}

/// Axum handler for SendGrid webhooks
pub async fn sendgrid_webhook_handler(
    State(handler): State<Arc<WebhookHandler>>,
    Json(events): Json<Vec<SendGridEvent>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    match handler.handle_sendgrid_webhook(events).await {
        Ok(response) => Ok((StatusCode::OK, Json(response))),
        Err(e) => {
            tracing::error!("Webhook processing error: {}", e);
            Err((
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error processing webhook: {}", e),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_sendgrid_event() {
        let json = r#"{
            "email": "test@example.com",
            "event": "delivered",
            "timestamp": 1234567890,
            "sg_message_id": "msg-123"
        }"#;

        let event: SendGridEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.email, "test@example.com");
        assert_eq!(event.event, "delivered");
    }
}
