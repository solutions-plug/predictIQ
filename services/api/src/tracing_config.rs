use opentelemetry::{
    global,
    trace::{TraceError, TracerProvider as _},
    KeyValue,
};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    runtime,
    trace::{RandomIdGenerator, Sampler, TracerProvider},
    Resource,
};
use opentelemetry_semantic_conventions::resource::{SERVICE_NAME, SERVICE_VERSION};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

/// Validates that `raw` is a float in the closed interval [0.0, 1.0].
/// Returns `Err` with a human-readable reason when the value is malformed or out of range.
pub(crate) fn validate_sampler_arg(raw: &str) -> Result<f64, String> {
    match raw.trim().parse::<f64>() {
        Ok(rate) if (0.0..=1.0).contains(&rate) => Ok(rate),
        Ok(rate) => Err(format!("value {rate} is out of range [0.0, 1.0]")),
        Err(_) => Err(format!("cannot parse {raw:?} as a float")),
    }
}

/// Reads `OTEL_TRACES_SAMPLER_ARG` from the environment and validates it.
/// If the variable is absent, returns `fallback` silently.
/// If the variable is present but invalid, emits `tracing::warn!` and returns `fallback`.
pub(crate) fn resolve_sampler_rate(fallback: f64) -> f64 {
    let raw = match std::env::var("OTEL_TRACES_SAMPLER_ARG") {
        Ok(v) => v,
        Err(_) => return fallback,
    };

    match validate_sampler_arg(&raw) {
        Ok(rate) => rate,
        Err(reason) => {
            tracing::warn!(
                invalid_value = %raw,
                fallback_rate = fallback,
                reason = %reason,
                "OTEL_TRACES_SAMPLER_ARG is invalid; using fallback sample rate"
            );
            fallback
        }
    }
}

/// Initialize distributed tracing with OpenTelemetry
pub fn init_tracing(
    service_name: &str,
    service_version: &str,
    otlp_endpoint: Option<String>,
    sample_rate: f64,
) -> Result<(), TraceError> {
    // OTel standard env vars take precedence over the passed-in rate.
    // OTEL_TRACES_SAMPLER / OTEL_TRACES_SAMPLER_ARG default to 10 % for production.
    let sample_rate = sample_rate_from_env(sample_rate);

    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, service_name.to_string()),
        KeyValue::new(SERVICE_VERSION, service_version.to_string()),
        KeyValue::new("deployment.environment", std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string())),
    ]);

    // OTEL_TRACES_SAMPLER_ARG overrides the configured rate when present and valid.
    let sample_rate = resolve_sampler_rate(sample_rate);

    // Configure sampler based on sample rate
    let sampler = if sample_rate >= 1.0 {
        Sampler::AlwaysOn
    } else if sample_rate <= 0.0 {
        Sampler::AlwaysOff
    } else {
        Sampler::TraceIdRatioBased(sample_rate)
    };

    // Build tracer provider
    let tracer_provider = if let Some(ref endpoint) = otlp_endpoint {
        // Export to OTLP collector (Jaeger, Zipkin, etc.)
        let exporter = opentelemetry_otlp::new_exporter()
            .tonic()
            .with_endpoint(endpoint);

        opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(exporter)
            .with_trace_config(
                opentelemetry_sdk::trace::Config::default()
                    .with_sampler(sampler)
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(resource),
            )
            .install_batch(runtime::Tokio)
            .map(|_tracer| {
                // install_batch already sets the global provider; build a local one too
                TracerProvider::builder()
                    .with_config(
                        opentelemetry_sdk::trace::Config::default()
                            .with_sampler(Sampler::AlwaysOn),
                    )
                    .build()
            })
            .unwrap_or_else(|_| {
                TracerProvider::builder().build()
            })
    } else {
        // No exporter configured - use noop
        TracerProvider::builder()
            .with_config(
                opentelemetry_sdk::trace::Config::default()
                    .with_sampler(sampler)
                    .with_id_generator(RandomIdGenerator::default())
                    .with_resource(resource),
            )
            .build()
    };

    // Set global tracer provider
    global::set_tracer_provider(tracer_provider.clone());

    // Create tracing layer
    let telemetry_layer = tracing_opentelemetry::layer()
        .with_tracer(tracer_provider.tracer(service_name.to_string()));

    // Initialize tracing subscriber with OpenTelemetry layer
    tracing_subscriber::registry()
        .with(EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(telemetry_layer)
        .init();

    tracing::info!(
        service_name = service_name,
        service_version = service_version,
        sample_rate = sample_rate,
        otlp_endpoint = otlp_endpoint.as_deref().unwrap_or("none"),
        "Distributed tracing initialized"
    );

    Ok(())
}

/// Shutdown tracing and flush remaining spans
pub fn shutdown_tracing() {
    tracing::info!("Shutting down tracing");
    global::shutdown_tracer_provider();
}

/// Extract trace context from HTTP headers for propagation
pub fn extract_trace_context(headers: &axum::http::HeaderMap) -> opentelemetry::Context {
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    let propagator = TraceContextPropagator::new();
    let context = propagator.extract(&HeaderExtractor(headers));
    context
}

/// Inject trace context into HTTP headers for propagation
pub fn inject_trace_context(
    headers: &mut reqwest::header::HeaderMap,
    context: &opentelemetry::Context,
) {
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry_sdk::propagation::TraceContextPropagator;

    let propagator = TraceContextPropagator::new();
    propagator.inject_context(context, &mut HeaderInjector(headers));
}

/// Helper to extract headers for OpenTelemetry propagation
struct HeaderExtractor<'a>(&'a axum::http::HeaderMap);

impl<'a> opentelemetry::propagation::Extractor for HeaderExtractor<'a> {
    fn get(&self, key: &str) -> Option<&str> {
        self.0.get(key).and_then(|v| v.to_str().ok())
    }

    fn keys(&self) -> Vec<&str> {
        self.0.keys().map(|k| k.as_str()).collect()
    }
}

/// Helper to inject headers for OpenTelemetry propagation
struct HeaderInjector<'a>(&'a mut reqwest::header::HeaderMap);

impl<'a> opentelemetry::propagation::Injector for HeaderInjector<'a> {
    fn set(&mut self, key: &str, value: String) {
        if let Ok(header_name) = reqwest::header::HeaderName::from_bytes(key.as_bytes()) {
            if let Ok(header_value) = reqwest::header::HeaderValue::from_str(&value) {
                self.0.insert(header_name, header_value);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sampler_configuration() {
        // Test always on
        let result = init_tracing("test-service", "0.1.0", None, 1.0);
        assert!(result.is_ok());
        shutdown_tracing();

        // Test always off
        let result = init_tracing("test-service", "0.1.0", None, 0.0);
        assert!(result.is_ok());
        shutdown_tracing();

        // Test ratio-based
        let result = init_tracing("test-service", "0.1.0", None, 0.5);
        assert!(result.is_ok());
        shutdown_tracing();
    }

    // ── validate_sampler_arg ──────────────────────────────────────────────────

    #[test]
    fn sampler_arg_valid_mid_range() {
        assert_eq!(validate_sampler_arg("0.5").unwrap(), 0.5);
    }

    #[test]
    fn sampler_arg_valid_lower_boundary() {
        assert_eq!(validate_sampler_arg("0.0").unwrap(), 0.0);
    }

    #[test]
    fn sampler_arg_valid_upper_boundary() {
        assert_eq!(validate_sampler_arg("1.0").unwrap(), 1.0);
    }

    #[test]
    fn sampler_arg_rejects_non_float() {
        let err = validate_sampler_arg("abc").unwrap_err();
        assert!(err.contains("abc"), "error message should quote the invalid value: {err}");
    }

    #[test]
    fn sampler_arg_rejects_out_of_range_high() {
        // A value of 1.5 is a valid float but outside [0.0, 1.0].
        // The warning IS emitted for this case (Err path → warn! at callsite).
        let err = validate_sampler_arg("1.5").unwrap_err();
        assert!(err.contains("out of range"), "error should describe range: {err}");
    }

    #[test]
    fn sampler_arg_rejects_out_of_range_low() {
        let err = validate_sampler_arg("-0.1").unwrap_err();
        assert!(err.contains("out of range"), "error should describe range: {err}");
    }

    #[test]
    fn sampler_arg_rejects_empty_string() {
        assert!(validate_sampler_arg("").is_err());
    }

    #[test]
    fn sampler_arg_rejects_whitespace_only() {
        assert!(validate_sampler_arg("   ").is_err());
    }

    #[test]
    fn resolve_sampler_rate_returns_fallback_when_env_absent() {
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
        assert_eq!(resolve_sampler_rate(0.3), 0.3);
    }

    #[test]
    fn resolve_sampler_rate_uses_env_when_valid() {
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "0.7");
        let rate = resolve_sampler_rate(0.1);
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
        assert_eq!(rate, 0.7);
    }

    #[test]
    fn resolve_sampler_rate_falls_back_on_invalid_env() {
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "not-a-number");
        let rate = resolve_sampler_rate(0.2);
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
        // Invalid value → fallback; the warning IS emitted (logged via tracing::warn!)
        assert_eq!(rate, 0.2);
    }

    #[test]
    fn resolve_sampler_rate_falls_back_on_out_of_range_env() {
        std::env::set_var("OTEL_TRACES_SAMPLER_ARG", "2.0");
        let rate = resolve_sampler_rate(0.5);
        std::env::remove_var("OTEL_TRACES_SAMPLER_ARG");
        assert_eq!(rate, 0.5);
    }
}
