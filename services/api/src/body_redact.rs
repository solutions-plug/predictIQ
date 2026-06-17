//! body_redact.rs — Body capture, truncation, and sensitive field redaction
//! for failed request/response logging.

use serde_json::{Map, Value};

/// Maximum bytes captured from request/response body before truncation.
pub const MAX_BODY_BYTES: usize = 4 * 1024; // 4 KB

const SENSITIVE_FIELDS: &[&str] = &[
    "password",
    "password_confirmation",
    "token",
    "access_token",
    "refresh_token",
    "secret",
    "api_key",
    "authorization",
    "credit_card",
    "cvv",
    "ssn",
];

/// Whether body logging is enabled. Reads AUDIT_BODY_LOGGING env var.
/// Defaults to `true`; set to "false" or "0" to disable.
pub fn body_logging_enabled() -> bool {
    std::env::var("AUDIT_BODY_LOGGING")
        .map(|v| v != "false" && v != "0")
        .unwrap_or(true)
}

/// Truncate raw bytes to MAX_BODY_BYTES and convert to UTF-8 string.
pub fn truncate_body(raw: &[u8]) -> String {
    let truncated = if raw.len() > MAX_BODY_BYTES {
        &raw[..MAX_BODY_BYTES]
    } else {
        raw
    };
    String::from_utf8_lossy(truncated).into_owned()
}

/// Redact sensitive fields from a JSON body string.
/// Non-JSON bodies are returned as-is.
pub fn redact_sensitive(body: &str) -> String {
    match serde_json::from_str::<Value>(body) {
        Ok(Value::Object(map)) => {
            let redacted = redact_map(map);
            serde_json::to_string(&Value::Object(redacted)).unwrap_or_else(|e| {
                tracing::warn!(error = %e, "failed to serialize redacted body; logging original");
                body.to_owned()
            })
        }
        _ => body.to_owned(),
    }
}

fn redact_map(mut map: Map<String, Value>) -> Map<String, Value> {
    for key in map.keys().cloned().collect::<Vec<_>>() {
        let lower = key.to_lowercase();
        if SENSITIVE_FIELDS.iter().any(|s| lower.contains(s)) {
            map.insert(key, Value::String("[REDACTED]".to_owned()));
        } else if let Some(Value::Object(nested)) = map.get(&key).cloned() {
            map.insert(key, Value::Object(redact_map(nested)));
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truncates_body_at_4kb() {
        let big = vec![b'x'; 5000];
        let result = truncate_body(&big);
        assert_eq!(result.len(), MAX_BODY_BYTES);
    }

    #[test]
    fn body_under_limit_not_truncated() {
        assert_eq!(truncate_body(b"hello"), "hello");
    }

    #[test]
    fn password_field_redacted() {
        let body = r#"{"email":"user@example.com","password":"s3cr3t"}"#;
        let redacted = redact_sensitive(body);
        let v: serde_json::Value = serde_json::from_str(&redacted).unwrap();
        assert_eq!(v["password"], "[REDACTED]");
        assert_eq!(v["email"], "user@example.com");
    }

    #[test]
    fn token_field_redacted() {
        let body = r#"{"access_token":"eyJhbGc...","user_id":42}"#;
        let redacted = redact_sensitive(body);
        let v: serde_json::Value = serde_json::from_str(&redacted).unwrap();
        assert_eq!(v["access_token"], "[REDACTED]");
        assert_eq!(v["user_id"], 42);
    }

    #[test]
    fn non_json_body_returned_as_is() {
        let body = "plain text error body";
        assert_eq!(redact_sensitive(body), body);
    }

    #[test]
    fn nested_sensitive_fields_redacted() {
        let body = r#"{"user":{"password":"secret","name":"Alice"}}"#;
        let redacted = redact_sensitive(body);
        let v: serde_json::Value = serde_json::from_str(&redacted).unwrap();
        assert_eq!(v["user"]["password"], "[REDACTED]");
        assert_eq!(v["user"]["name"], "Alice");
    }
}
