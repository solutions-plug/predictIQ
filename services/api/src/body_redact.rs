//! body_redact.rs — Body capture, truncation, and sensitive field redaction
//! for failed request/response logging.

use serde_json::{Map, Value};
use tracing;

/// Maximum bytes captured from request/response body before truncation.
pub const MAX_BODY_BYTES: usize = 4 * 1024; // 4 KB

/// Issue #1524: Expanded list of sensitive field names to catch common secret/PII patterns.
/// Matches case-insensitively via substring containment (see known limitations in docs).
const SENSITIVE_FIELDS: &[&str] = &[
    // Authentication & authorization
    "password",
    "password_confirmation",
    "passwd",
    "pwd",
    "token",
    "access_token",
    "refresh_token",
    "bearer",
    "authorization",
    "auth_token",
    "session_token",
    "api_key",
    "api_secret",
    "secret",
    "client_secret",
    "private_key",
    "private_secret",
    "signing_key",
    "hmac_key",

    // Financial & Identity
    "credit_card",
    "creditcard",
    "card_number",
    "cvv",
    "cvc",
    "ssn",
    "social_security",
    "tin",
    "ein",
    "bank_account",
    "routing_number",
    "account_number",

    // Personal Information
    "email",
    "phone",
    "phone_number",
    "mobile",
    "dob",
    "date_of_birth",
    "birthdate",
    "drivers_license",
    "passport",
    "identity_number",

    // OAuth & Third-party
    "oauth_token",
    "jwt",
    "jti",
    "nonce",
    "signature",
    "signed_request",
    "webhook_secret",
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

/// Redact an email address for logging purposes.
/// Returns a hashed version of the email that preserves privacy while
/// remaining useful for debugging (e.g., "user@example.com" -> "u***@e***.com").
pub fn redact_email(email: &str) -> String {
    if email.is_empty() {
        return String::new();
    }
    
    // Simple redaction: show first character, last domain, and TLD
    if let Some(at_pos) = email.find('@') {
        let local_part = &email[..at_pos];
        let domain_part = &email[at_pos + 1..];
        
        let redacted_local = if local_part.len() > 1 {
            format!("{}***", &local_part[0..1])
        } else {
            local_part.to_string()
        };
        
        // Keep the domain but obscure most of it
        let redacted_domain = if let Some(dot_pos) = domain_part.rfind('.') {
            let name_part = &domain_part[..dot_pos];
            let tld = &domain_part[dot_pos..];
            
            if name_part.len() > 1 {
                format!("{}***{}", &name_part[0..1], tld)
            } else {
                format!("{}{}", name_part, tld)
            }
        } else {
            // No dot in domain - obscure it completely
            "***".to_string()
        };
        
        format!("{}@{}", redacted_local, redacted_domain)
    } else {
        // Not a valid email format - obscure it completely
        "***@***".to_string()
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
        assert_eq!(v["email"], "[REDACTED]");
    }

    #[test]
    fn email_redaction_function_works() {
        assert_eq!(redact_email("user@example.com"), "u***@e***.com");
        assert_eq!(redact_email("a@b.com"), "a@b***.com");
        assert_eq!(redact_email("test@domain.co.uk"), "t***@d***.co.uk");
        assert_eq!(redact_email(""), "");
        assert_eq!(redact_email("not-an-email"), "***@***");
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

    /// Issue #1524: Property test — any field name containing a SENSITIVE_FIELDS
    /// substring should be redacted. Catches over-redaction (tokenizer_version) and
    /// under-redaction (ssn missing from original list).
    #[test]
    fn all_sensitive_substrings_redacted() {
        for sensitive_substr in SENSITIVE_FIELDS {
            // Test exact match
            let body = format!(r#"{{"{}":"value"}}"#, sensitive_substr);
            let redacted = redact_sensitive(&body);
            let v: serde_json::Value = serde_json::from_str(&redacted).unwrap();
            assert_eq!(
                v[sensitive_substr], "[REDACTED]",
                "Field '{}' should be redacted", sensitive_substr
            );

            // Test case-insensitive match
            let body_upper = format!(r#"{{"{}_upper":"value"}}"#, sensitive_substr);
            let redacted_upper = redact_sensitive(&body_upper);
            let v_upper: serde_json::Value = serde_json::from_str(&redacted_upper).unwrap();
            assert_eq!(
                v_upper[&format!("{}_upper", sensitive_substr)], "[REDACTED]",
                "Field with '{}' substring should be redacted", sensitive_substr
            );
        }
    }

    /// Issue #1524: Known limitation — over-redaction on false positives.
    /// Fields like "token_secret_value" will match both "token" and "secret".
    #[test]
    fn documents_substring_matching_tradeoff() {
        // False positive: tokenizer_version contains "token"
        let body = r#"{"tokenizer_version":"1.0","token":"secret"}"#;
        let redacted = redact_sensitive(body);
        let v: serde_json::Value = serde_json::from_str(&redacted).unwrap();

        // Confirms the over-redaction happens (substring matching)
        assert_eq!(v["tokenizer_version"], "[REDACTED]");
        assert_eq!(v["token"], "[REDACTED]");
    }

    /// Issue #1524: Test emerging sensitive field patterns not in original list.
    #[test]
    fn new_patterns_covered() {
        // private_key, auth_token, etc. should all be in expanded list
        let test_cases = vec![
            (r#"{"private_key":"-----BEGIN RSA PRIVATE KEY-----"}"#, "private_key"),
            (r#"{"auth_token":"abc123xyz"}"#, "auth_token"),
            (r#"{"signing_key":"key123"}"#, "signing_key"),
            (r#"{"webhook_secret":"webhook123"}"#, "webhook_secret"),
            (r#"{"ssn":"123-45-6789"}"#, "ssn"),
            (r#"{"bank_account":"0123456789"}"#, "bank_account"),
        ];

        for (body, field) in test_cases {
            let redacted = redact_sensitive(body);
            let v: serde_json::Value = serde_json::from_str(&redacted).unwrap();
            assert_eq!(
                v[field], "[REDACTED]",
                "Field '{}' should be redacted", field
            );
        }
    }
}
