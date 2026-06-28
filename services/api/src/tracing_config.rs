use opentelemetry::{
    global,
    trace::TracerProvider as _,
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
use url::Url;

/// Resolve the trace sampling rate from OTel standard env vars, falling back to `default_rate`.
///
/// Reads `OTEL_TRACES_SAMPLER` and `OTEL_TRACES_SAMPLER_ARG` per the OpenTelemetry
/// environment-variable specification.  The default production rate is **10 %** (0.1).
///
/// | `OTEL_TRACES_SAMPLER`               | Effect                              |
/// |-------------------------------------|-------------------------------------|
/// | `always_on`                         | Sample 100 %                        |
/// | `always_off`                        | Sample 0 %                          |
/// | `traceidratio` *(default)*          | Use `OTEL_TRACES_SAMPLER_ARG` ratio |
/// | `parentbased_always_on`             | Sample 100 %                        |
/// | `parentbased_always_off`            | Sample 0 %                          |
/// | `parentbased_traceidratio`          | Use `OTEL_TRACES_SAMPLER_ARG` ratio |
pub fn sample_rate_from_env(default_rate: f64) -> f64 {
    let sampler = std::env::var("OTEL_TRACES_SAMPLER")
        .unwrap_or_else(|_| "traceidratio".to_string());

    match sampler.trim() {
        "always_on" | "parentbased_always_on" => 1.0,
        "always_off" | "parentbased_always_off" => 0.0,
        "traceidratio" | "parentbased_traceidratio" => {
            std::env::var("OTEL_TRACES_SAMPLER_ARG")
                .ok()
                .and_then(|v| v.trim().parse::<f64>().ok())
                .filter(|r| (0.0..=1.0).contains(r))
                .unwrap_or(default_rate)
        }
        _ => default_rate,
    }
}

/// Validate that `endpoint` is a parseable URL and that the host:port is
/// reachable via a TCP dial.  Logs a WARNING if the check fails so the service
/// starts normally but operators know traces are being lost.
///
/// Returns `true` when the endpoint is reachable, `false` otherwise.
pub async fn check_otlp_connectivity(endpoint: &str) -> bool {
    let parsed = match Url::parse(endpoint) {
        Ok(u) => u,
        Err(e) => {
            tracing::warn!(
                endpoint,
                error = %e,
                "OTLP endpoint URL is invalid — traces will not be exported"
            );
            return false;
        }
    };

    let host = match parsed.host_str() {
        Some(h) => h.to_string(),
        None => {
            tracing::warn!(
                endpoint,
                "OTLP endpoint URL has no host — traces will not be exported"
            );
            return false;
        }
    };

    let port = parsed.port().unwrap_or(4317);
    let addr = format!("{host}:{port}");

    match tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::net::TcpStream::connect(&addr),
    )
    .await
    {
        Ok(Ok(_)) => {
            tracing::info!(endpoint, "OTLP endpoint connectivity check passed");
            true
        }
        Ok(Err(e)) => {
            tracing::warn!(
                endpoint,
                error = %e,
                "OTLP endpoint is not reachable — traces may be lost"
            );
            false
        }
        Err(_) => {
            tracing::warn!(
                endpoint,
                "OTLP endpoint connectivity check timed out after 2 s — traces may be lost"
            );
            false
        }
    }
}

/// Initialize distributed tracing with OpenTelemetry.
///
/// Validates `otlp_endpoint` as a parseable URL; returns an error immediately
/// if the value is set but cannot be parsed (prevents silent misconfiguration).
/// A separate TCP reachability check is available via [`check_otlp_connectivity`].
pub fn init_tracing(
    service_name: &str,
    service_version: &str,
    otlp_endpoint: Option<String>,
    sample_rate: f64,
) -> anyhow::Result<()> {
    // Fail fast on an unparseable endpoint URL so the error surfaces at startup
    // rather than silently at the first export attempt.
    if let Some(ref ep) = otlp_endpoint {
        Url::parse(ep).map_err(|e| {
            anyhow::anyhow!(
                "OTEL_EXPORTER_OTLP_ENDPOINT '{}' is not a valid URL: {}",
                ep,
                e
            )
        })?;
    }

    // OTel standard env vars take precedence over the passed-in rate.
    // OTEL_TRACES_SAMPLER / OTEL_TRACES_SAMPLER_ARG default to 10 % for production.
    let sample_rate = sample_rate_from_env(sample_rate);

    // Create resource with service information
    let resource = Resource::new(vec![
        KeyValue::new(SERVICE_NAME, service_name.to_string()),
        KeyValue::new(SERVICE_VERSION, service_version.to_string()),
        KeyValue::new("deployment.environment", std::env::var("ENVIRONMENT").unwrap_or_else(|_| "development".to_string())),
    ]);

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

    #[test]
    fn invalid_otlp_url_is_rejected_at_startup() {
        let result = init_tracing(
            "test-service",
            "0.1.0",
            Some("not a valid url :::".to_string()),
            0.1,
        );
        assert!(result.is_err());
        let msg = format!("{}", result.unwrap_err());
        assert!(
            msg.contains("OTEL_EXPORTER_OTLP_ENDPOINT"),
            "error message should mention the env var, got: {msg}"
        );
    }

    #[test]
    fn valid_otlp_url_passes_parse_check() {
        // URL parsing only — no TCP connection is attempted in init_tracing
        let result = init_tracing(
            "test-service",
            "0.1.0",
            Some("http://localhost:4317".to_string()),
            0.0,
        );
        // init may fail later (e.g. already-initialized subscriber) but must not
        // fail at URL validation, so we only check it's NOT a URL-parse error.
        if let Err(e) = result {
            let msg = format!("{e}");
            assert!(
                !msg.contains("not a valid URL"),
                "init_tracing rejected a valid URL: {msg}"
            );
        }
        shutdown_tracing();
    }
}
