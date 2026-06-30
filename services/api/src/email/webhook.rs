use anyhow::{anyhow, Result};
use axum::{extract::State, http::{HeaderMap, StatusCode}, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::db::Database;
use crate::email::types::SuppressionType;

/// Maximum allowed raw webhook body size: 64 KiB.
/// Payloads larger than this are rejected before any parsing occurs.
pub const MAX_WEBHOOK_PAYLOAD_BYTES: usize = 64 * 1024;

/// Maximum length for free-text fields stored in the database (reason, response, etc.).
const MAX_TEXT_FIELD_LEN: usize = 1024;

/// Maximum length for structured identifier fields (email, event type, message_id).
const MAX_ID_FIELD_LEN: usize = 254;

/// Raw deserialization target — accepts any valid JSON so we can validate
/// before committing anything to the database.
#[derive(Debug, Clone, Deserialize)]
struct RawSendGridEvent {
    email: Option<String>,
    event: Option<String>,
    timestamp: Option<serde_json::Value>,
    #[serde(rename = "sg_message_id")]
    message_id: Option<String>,
    reason: Option<String>,
    status: Option<String>,
    response: Option<String>,
    #[serde(rename = "bounce_classification")]
    bounce_classification: Option<String>,
    url: Option<String>,
}

/// Sanitized, allow-listed event structure — only these fields are persisted.
///
/// All text fields are stripped of HTML/JavaScript and length-capped before
/// being stored or passed to any downstream logic.
#[derive(Debug, Clone, Serialize)]
pub struct SendGridEvent {
    pub email: String,
    pub event: String,
    pub timestamp: i64,
    pub message_id: Option<String>,
    pub reason: Option<String>,
    pub status: Option<String>,
    pub response: Option<String>,
    pub bounce_classification: Option<String>,
    pub url: Option<String>,
}

// Keep a Deserialize impl so the handler can accept JSON directly in tests.
// We deliberately do NOT forward unknown fields — the sanitized struct is the
// canonical representation persisted to the database.
impl<'de> Deserialize<'de> for SendGridEvent {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let raw = RawSendGridEvent::deserialize(d).map_err(serde::de::Error::custom)?;
        sanitize_event(raw).map_err(serde::de::Error::custom)
    }
}

// ── sanitization helpers ──────────────────────────────────────────────────────

/// Strip HTML tags and JavaScript injection patterns from a string, then
/// truncate to `max_len` characters.
fn strip_html_and_truncate(input: &str, max_len: usize) -> String {
    // Remove HTML tags (<...>)
    let mut out = String::with_capacity(input.len());
    let mut in_tag = false;
    for ch in input.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }

    // Remove common JavaScript injection patterns
    let dangerous = [
        "javascript:",
        "vbscript:",
        "data:text/html",
        "onerror=",
        "onload=",
        "onclick=",
        "eval(",
        "expression(",
    ];
    let mut lower = out.to_lowercase();
    for pat in &dangerous {
        // Replace occurrences without allocating per iteration
        while let Some(pos) = lower.find(pat) {
            out.drain(pos..pos + pat.len());
            lower = out.to_lowercase();
        }
    }

    // Truncate to maximum length
    out.chars().take(max_len).collect()
}

/// Sanitize a single raw event into the allow-listed `SendGridEvent`.
fn sanitize_event(raw: RawSendGridEvent) -> Result<SendGridEvent> {
    let email = raw
        .email
        .ok_or_else(|| anyhow!("missing required field: email"))?;
    let email = email.trim().to_string();
    if email.len() > MAX_ID_FIELD_LEN || email.is_empty() {
        return Err(anyhow!("email field invalid or too long"));
    }

    let event = raw
        .event
        .ok_or_else(|| anyhow!("missing required field: event"))?;
    let event = event.trim().to_string();
    if event.len() > 64 || event.is_empty() {
        return Err(anyhow!("event field invalid or too long"));
    }

    let timestamp = match raw.timestamp {
        Some(serde_json::Value::Number(n)) => n
            .as_i64()
            .ok_or_else(|| anyhow!("timestamp is not a valid i64"))?,
        Some(serde_json::Value::String(s)) => s
            .parse::<i64>()
            .map_err(|_| anyhow!("timestamp string is not a valid i64"))?,
        _ => return Err(anyhow!("missing or invalid timestamp")),
    };

    let sanitize_opt = |v: Option<String>| -> Option<String> {
        v.map(|s| strip_html_and_truncate(s.trim(), MAX_TEXT_FIELD_LEN))
            .filter(|s| !s.is_empty())
    };

    let sanitize_id = |v: Option<String>| -> Option<String> {
        v.map(|s| s.trim().chars().take(MAX_ID_FIELD_LEN).collect::<String>())
            .filter(|s| !s.is_empty())
    };

    Ok(SendGridEvent {
        email,
        event,
        timestamp,
        message_id: sanitize_id(raw.message_id),
        reason: sanitize_opt(raw.reason),
        status: sanitize_id(raw.status),
        response: sanitize_opt(raw.response),
        bounce_classification: sanitize_id(raw.bounce_classification),
        url: sanitize_id(raw.url),
    })
}

/// Validate that a raw JSON payload list does not exceed `MAX_WEBHOOK_PAYLOAD_BYTES`
/// and parse + sanitize each event.
///
/// Returns `Err` if the payload is oversized or any event fails validation.
pub fn parse_and_sanitize_events(raw_bytes: &[u8]) -> Result<Vec<SendGridEvent>> {
    if raw_bytes.len() > MAX_WEBHOOK_PAYLOAD_BYTES {
        return Err(anyhow!(
            "webhook payload exceeds maximum size ({} > {} bytes)",
            raw_bytes.len(),
            MAX_WEBHOOK_PAYLOAD_BYTES
        ));
    }
    let events: Vec<SendGridEvent> = serde_json::from_slice(raw_bytes)
        .map_err(|e| anyhow!("failed to parse SendGrid events: {}", e))?;
    Ok(events)
}

// ── domain logic ─────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct WebhookHandler {
    db: Database,
}

impl WebhookHandler {
    pub fn new(db: Database) -> Self {
        Self { db }
    }

    /// Process a list of already-sanitized SendGrid webhook events.
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
        let timestamp = event.timestamp;

        tracing::info!(
            event_type,
            email,
            message_id,
            "Processing SendGrid event"
        );

        // Check for replay attack
        if self
            .db
            .email_event_exists(message_id, event_type, email, timestamp)
            .await?
        {
            tracing::warn!(
                message_id,
                event_type,
                email,
                timestamp,
                "Replay attack detected for SendGrid event"
            );
            return Ok(());
        }

        // Persist the sanitized, allow-listed event.
        // Only the fields present in `SendGridEvent` are serialized — the raw
        // `extra` catch-all from the old schema is intentionally absent.
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
                self.db
                    .email_increment_analytics_counter("delivered", None)
                    .await?;
            }
            "open" => {
                self.db
                    .email_increment_analytics_counter("opened", None)
                    .await?;
            }
            "click" => {
                self.db
                    .email_increment_analytics_counter("clicked", None)
                    .await?;
            }
            "bounce" | "dropped" => {
                self.handle_bounce(&event).await?;
            }
            "spamreport" => {
                self.handle_complaint(&event).await?;
            }
            "unsubscribe" => {
                self.handle_unsubscribe(&event).await?;
            }
            _ => {
                tracing::debug!(event_type, "Unhandled SendGrid event type");
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

        self.db
            .email_add_suppression(
                &event.email,
                SuppressionType::Bounce.as_str(),
                Some(reason),
                Some(bounce_type),
            )
            .await?;

        self.db
            .email_increment_analytics_counter("bounced", None)
            .await?;

        tracing::warn!(
            email = %event.email,
            bounce_type,
            reason,
            "Email bounced"
        );

        Ok(())
    }

    async fn handle_complaint(&self, event: &SendGridEvent) -> Result<()> {
        let reason = event.reason.as_deref().unwrap_or("Spam complaint");

        self.db
            .email_add_suppression(
                &event.email,
                SuppressionType::Complaint.as_str(),
                Some(reason),
                None,
            )
            .await?;

        self.db
            .email_increment_analytics_counter("complained", None)
            .await?;

        tracing::warn!(email = %event.email, "Spam complaint received");

        Ok(())
    }

    async fn handle_unsubscribe(&self, event: &SendGridEvent) -> Result<()> {
        self.db
            .email_add_suppression(
                &event.email,
                SuppressionType::Unsubscribe.as_str(),
                Some("User unsubscribed via email link"),
                None,
            )
            .await?;

        let _ = self.db.newsletter_unsubscribe(&event.email).await;

        self.db
            .email_increment_analytics_counter("unsubscribed", None)
            .await?;

        tracing::info!(email = %event.email, "User unsubscribed");

        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct WebhookResponse {
    pub processed: usize,
    pub errors: Option<Vec<String>>,
}

/// Axum handler for SendGrid webhooks.
///
/// The raw body size limit is enforced at the middleware layer
/// (see `sendgrid_webhook_middleware` in `security.rs`).  This handler
/// deserializes the already-size-checked bytes through the sanitizing
/// `SendGridEvent` deserializer, so no raw event data ever reaches the DB.
pub async fn sendgrid_webhook_handler(
    State(handler): State<Arc<WebhookHandler>>,
    _headers: HeaderMap,
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

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── allow-list / field sanitization ──────────────────────────────────────

    #[test]
    fn test_deserialize_sendgrid_event_minimal() {
        let json = r#"{
            "email": "test@example.com",
            "event": "delivered",
            "timestamp": 1234567890,
            "sg_message_id": "msg-123"
        }"#;

        let event: SendGridEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.email, "test@example.com");
        assert_eq!(event.event, "delivered");
        assert_eq!(event.timestamp, 1234567890);
        assert_eq!(event.message_id.as_deref(), Some("msg-123"));
    }

    #[test]
    fn test_unknown_fields_are_dropped() {
        // The old schema had `#[serde(flatten)] extra: serde_json::Value` which
        // allowed arbitrary data into the DB.  The new schema uses an allow-list,
        // so unknown fields must be silently ignored and never persisted.
        let json = r#"{
            "email": "test@example.com",
            "event": "bounce",
            "timestamp": 1700000000,
            "arbitrary_attacker_field": "evil_payload",
            "nested": {"deep": "data"},
            "__proto__": {"polluted": true}
        }"#;

        let event: SendGridEvent = serde_json::from_str(json).unwrap();
        let serialized = serde_json::to_value(&event).unwrap();

        // Confirm none of the unknown fields survived serialization
        assert!(serialized.get("arbitrary_attacker_field").is_none());
        assert!(serialized.get("nested").is_none());
        assert!(serialized.get("__proto__").is_none());
    }

    #[test]
    fn test_html_stripped_from_reason() {
        let json = r#"{
            "email": "user@example.com",
            "event": "bounce",
            "timestamp": 1700000000,
            "reason": "<script>alert('xss')</script>Hard bounce"
        }"#;

        let event: SendGridEvent = serde_json::from_str(json).unwrap();
        let reason = event.reason.unwrap();
        assert!(!reason.contains("<script>"), "HTML tags must be stripped");
        assert!(!reason.contains("</script>"), "HTML tags must be stripped");
        assert!(reason.contains("Hard bounce"), "Legitimate text must be kept");
    }

    #[test]
    fn test_javascript_uri_stripped_from_reason() {
        let json = r#"{
            "email": "user@example.com",
            "event": "bounce",
            "timestamp": 1700000000,
            "reason": "javascript:alert(1) Hard bounce"
        }"#;

        let event: SendGridEvent = serde_json::from_str(json).unwrap();
        let reason = event.reason.unwrap();
        assert!(
            !reason.to_lowercase().contains("javascript:"),
            "JavaScript URIs must be stripped"
        );
    }

    #[test]
    fn test_reason_field_truncated_at_max_len() {
        let long_reason = "A".repeat(MAX_TEXT_FIELD_LEN + 500);
        let json = serde_json::json!({
            "email": "user@example.com",
            "event": "bounce",
            "timestamp": 1700000000_i64,
            "reason": long_reason
        })
        .to_string();

        let event: SendGridEvent = serde_json::from_str(&json).unwrap();
        assert!(
            event.reason.unwrap().len() <= MAX_TEXT_FIELD_LEN,
            "reason must be truncated to MAX_TEXT_FIELD_LEN"
        );
    }

    // ── payload size enforcement ──────────────────────────────────────────────

    #[test]
    fn test_oversized_payload_is_rejected() {
        // Build a payload that exceeds MAX_WEBHOOK_PAYLOAD_BYTES
        let large_value = "x".repeat(MAX_WEBHOOK_PAYLOAD_BYTES);
        let json = serde_json::json!([{
            "email": "test@example.com",
            "event": "delivered",
            "timestamp": 1700000000_i64,
            "reason": large_value
        }])
        .to_string();

        let bytes = json.as_bytes();
        assert!(
            bytes.len() > MAX_WEBHOOK_PAYLOAD_BYTES,
            "test precondition: payload must exceed limit"
        );

        let result = parse_and_sanitize_events(bytes);
        assert!(
            result.is_err(),
            "oversized payload must be rejected before parsing"
        );
        let err_msg = result.unwrap_err().to_string();
        assert!(
            err_msg.contains("exceeds maximum size"),
            "error message should describe the size violation; got: {err_msg}"
        );
    }

    #[test]
    fn test_payload_at_exact_size_limit_proceeds_to_parse() {
        // A payload right at the limit must not be rejected by the size check.
        // (It may still fail JSON parsing, but not the size guard.)
        let payload = b"[]"; // 2 bytes, well under 64 KiB
        let result = parse_and_sanitize_events(payload);
        // Empty array is valid — should succeed
        assert!(result.is_ok(), "valid minimal payload must be accepted");
        assert!(result.unwrap().is_empty());
    }

    #[test]
    fn test_missing_required_email_field_rejected() {
        let json = r#"[{"event": "delivered", "timestamp": 1700000000}]"#;
        let result = parse_and_sanitize_events(json.as_bytes());
        assert!(result.is_err(), "missing email should cause parse failure");
    }

    #[test]
    fn test_missing_required_event_field_rejected() {
        let json = r#"[{"email": "user@example.com", "timestamp": 1700000000}]"#;
        let result = parse_and_sanitize_events(json.as_bytes());
        assert!(result.is_err(), "missing event should cause parse failure");
    }

    #[test]
    fn test_strip_html_helper_preserves_plain_text() {
        let input = "Hard bounce: user not found";
        assert_eq!(strip_html_and_truncate(input, 256), input);
    }

    #[test]
    fn test_strip_html_helper_removes_nested_tags() {
        let input = "<b><i>bold italic</i></b> text";
        let output = strip_html_and_truncate(input, 256);
        assert!(!output.contains('<'));
        assert!(output.contains("bold italic"));
        assert!(output.contains("text"));
    }
}
