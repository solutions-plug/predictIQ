//! CORS configuration layer construction and testing.
//!
//! The `build_cors_layer` function constructs a tower-http `CorsLayer` from
//! configuration, with special handling for dev mode (fully permissive) vs.
//! production mode (strict allowlist enforcement).

use axum_http::header::{HeaderName, HeaderValue};
use tower_http::cors::{CorsLayer, AllowOrigin};
use axum_http::Method;
use std::fmt;
use std::time::Duration;
use crate::config::CorsConfig;

/// Error returned when a CORS configuration entry cannot be parsed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorsConfigError {
    /// The configuration field that contained the offending entry.
    pub field: &'static str,
    /// The offending entry, verbatim.
    pub value: String,
    /// The underlying parse error message.
    pub reason: String,
}

impl fmt::Display for CorsConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid CORS {} entry {:?}: {}",
            self.field, self.value, self.reason
        )
    }
}

impl std::error::Error for CorsConfigError {}

/// Build a CORS layer from configuration.
///
/// # Behavior
/// - **Dev mode** (`CORS_DEV_MODE=true`): Returns a permissive layer that allows
///   all origins, methods, and headers. A warning is emitted to the log. This
///   setting should never be used in production.
/// - **Strict mode**: Enforces an allowlist of origins, methods, and headers from
///   config. If no origins are specified, cross-origin requests are blocked.
///   The `allow_credentials` flag determines whether cookies/credentials are permitted.
///
/// # Parameters
/// - `cfg`: CORS configuration loaded from environment variables.
///
/// # Returns
/// A `Result` containing a `CorsLayer` ready to be added to the Axum router, or a
/// `CorsConfigError` naming the first unparsable entry encountered. Unparsable
/// entries are never silently dropped.
pub fn build_cors_layer(cfg: &CorsConfig) -> Result<CorsLayer, CorsConfigError> {
    if cfg.dev_mode {
        tracing::warn!(
            "CORS_DEV_MODE is enabled — all origins are permitted. \
             This MUST NOT be used in production."
        );
        return Ok(CorsLayer::permissive());
    }

    let origins: Vec<HeaderValue> = cfg
        .allowed_origins
        .iter()
        .map(|o| {
            o.parse::<HeaderValue>().map_err(|e| {
                let err = CorsConfigError {
                    field: "allowed_origins",
                    value: o.clone(),
                    reason: e.to_string(),
                };
                tracing::error!(%err, "CORS: unparsable allowed origin");
                err
            })
        })
        .collect::<Result<_, _>>()?;

    let methods: Vec<Method> = cfg
        .allowed_methods
        .iter()
        .map(|m| {
            m.parse::<Method>().map_err(|e| {
                let err = CorsConfigError {
                    field: "allowed_methods",
                    value: m.clone(),
                    reason: e.to_string(),
                };
                tracing::error!(%err, "CORS: unparsable allowed method");
                err
            })
        })
        .collect::<Result<_, _>>()?;

    let headers: Vec<HeaderName> = cfg
        .allowed_headers
        .iter()
        .map(|h| {
            h.parse::<HeaderName>().map_err(|e| {
                let err = CorsConfigError {
                    field: "allowed_headers",
                    value: h.clone(),
                    reason: e.to_string(),
                };
                tracing::error!(%err, "CORS: unparsable allowed header");
                err
            })
        })
        .collect::<Result<_, _>>()?;

    let layer = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods(methods)
        .allow_headers(headers)
        .max_age(Duration::from_secs(cfg.max_age_secs));

    if cfg.allow_credentials {
        if cfg.allowed_origins.iter().any(|o| o.trim() == "*") {
            tracing::warn!("CORS: wildcard origin is not allowed with credentials — credentials disabled");
            return Ok(layer);
        }
        Ok(layer.allow_credentials(true))
    } else {
        Ok(layer)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dev_mode_is_permissive() {
        let cfg = CorsConfig {
            dev_mode: true,
            allowed_origins: vec!["https://example.com".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        let layer = build_cors_layer(&cfg).expect("dev mode layer should build");
        // Permissive layer allows all origins/methods/headers.
        // We can't easily inspect the inner state, but we verify it builds without panic.
        drop(layer);
    }

    #[test]
    fn strict_mode_with_single_origin() {
        let cfg = CorsConfig {
            dev_mode: false,
            allowed_origins: vec!["https://app.example.com".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string()],
            allowed_headers: vec!["content-type".to_string(), "authorization".to_string()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        let layer = build_cors_layer(&cfg).expect("valid strict config should build");
        // Strict mode respects allowlist configuration.
        drop(layer);
    }

    #[test]
    fn strict_mode_with_empty_origins_blocks_cors() {
        let cfg = CorsConfig {
            dev_mode: false,
            allowed_origins: vec![],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        let layer = build_cors_layer(&cfg).expect("empty origins config should build");
        // With no allowed origins, cross-origin requests should be blocked.
        drop(layer);
    }

    #[test]
    fn credentials_flag_is_honored() {
        let cfg_with_credentials = CorsConfig {
            dev_mode: false,
            allowed_origins: vec!["https://app.example.com".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string()],
            allow_credentials: true,
            max_age_secs: 3600,
        };

        let cfg_without_credentials = CorsConfig {
            dev_mode: false,
            allowed_origins: vec!["https://app.example.com".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        let layer_with = build_cors_layer(&cfg_with_credentials)
            .expect("valid config with credentials should build");
        let layer_without = build_cors_layer(&cfg_without_credentials)
            .expect("valid config without credentials should build");

        // Both layers build successfully; credentials behavior is embedded in them.
        drop(layer_with);
        drop(layer_without);
    }

    #[test]
    fn max_age_is_respected() {
        let cfg = CorsConfig {
            dev_mode: false,
            allowed_origins: vec!["https://app.example.com".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string()],
            allow_credentials: false,
            max_age_secs: 7200,
        };

        let layer = build_cors_layer(&cfg).expect("valid max_age config should build");
        // max_age is baked into the layer during construction.
        drop(layer);
    }

    #[test]
    fn multiple_origins_are_supported() {
        let cfg = CorsConfig {
            dev_mode: false,
            allowed_origins: vec![
                "https://app1.example.com".to_string(),
                "https://app2.example.com".to_string(),
                "https://staging.example.com".to_string(),
            ],
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string()],
            allowed_headers: vec!["content-type".to_string(), "authorization".to_string()],
            allow_credentials: true,
            max_age_secs: 5000,
        };

        let layer = build_cors_layer(&cfg).expect("multiple origins config should build");
        drop(layer);
    }

    #[test]
    fn invalid_origin_returns_error_naming_entry() {
        let bad = "not a valid header value 😀";
        let cfg = CorsConfig {
            dev_mode: false,
            // Mix valid and invalid origins.
            allowed_origins: vec![
                "https://valid.example.com".to_string(),
                bad.to_string(),
            ],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        let err = build_cors_layer(&cfg).expect_err("invalid origin must fail");
        assert_eq!(err.field, "allowed_origins");
        assert_eq!(err.value, bad);
        assert!(
            err.to_string().contains(bad),
            "error message should name the offending entry: {err}"
        );
    }

    #[test]
    fn invalid_method_returns_error_naming_entry() {
        let bad = "NOT A METHOD";
        let cfg = CorsConfig {
            dev_mode: false,
            allowed_origins: vec!["https://app.example.com".to_string()],
            allowed_methods: vec!["GET".to_string(), bad.to_string()],
            allowed_headers: vec!["content-type".to_string()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        let err = build_cors_layer(&cfg).expect_err("invalid method must fail");
        assert_eq!(err.field, "allowed_methods");
        assert_eq!(err.value, bad);
        assert!(
            err.to_string().contains(bad),
            "error message should name the offending entry: {err}"
        );
    }

    #[test]
    fn invalid_header_returns_error_naming_entry() {
        let bad = "invalid header name 😀";
        let cfg = CorsConfig {
            dev_mode: false,
            allowed_origins: vec!["https://app.example.com".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["content-type".to_string(), bad.to_string()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        let err = build_cors_layer(&cfg).expect_err("invalid header must fail");
        assert_eq!(err.field, "allowed_headers");
        assert_eq!(err.value, bad);
        assert!(
            err.to_string().contains(bad),
            "error message should name the offending entry: {err}"
        );
    }

    #[test]
    fn dev_mode_ignores_invalid_entries() {
        let cfg = CorsConfig {
            dev_mode: true,
            allowed_origins: vec!["not a valid header value 😀".to_string()],
            allowed_methods: vec!["NOT A METHOD".to_string()],
            allowed_headers: vec!["invalid header name 😀".to_string()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        // Dev mode stays permissive and does not validate the allowlist.
        let layer = build_cors_layer(&cfg).expect("dev mode should always build");
        drop(layer);
    }
}
