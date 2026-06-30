# Distributed Tracing

This service implements distributed tracing using OpenTelemetry to track requests across service boundaries.

## Features

- Trace context propagation across HTTP calls
- Configurable sampling rates
- Support for multiple backends (Jaeger, Zipkin)
- Automatic span creation at service boundaries
- Request correlation with trace IDs

## Endpoint Validation

`OTEL_EXPORTER_OTLP_ENDPOINT` is validated as a parseable URL at startup.
An invalid value (e.g. `not-a-url`) causes the service to **exit immediately**
with a clear error rather than failing silently during the first export.

After the tracing subscriber is initialised, a **TCP connectivity check**
(2-second timeout) is attempted against the configured endpoint.
If the endpoint is unreachable the service **continues to start** but:

- Logs a `WARN` message referencing the endpoint and error.
- Increments `otel_export_errors_total{reason="unreachable"}`.

Monitor that counter to detect collector outages before they cause silent
data loss in production.

## Prometheus Metrics

| Metric | Labels | Description |
|---|---|---|
| `otel_export_errors_total` | `reason` | Export failures — `unreachable` (startup TCP check), `export_failed` (runtime) |

## Configuration

Configure tracing via environment variables:

```bash
# OTLP endpoint (leave unset to disable trace export)
# Must be a valid URL — invalid values fail the service at startup.
OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4317

# ── Sampling (OTel standard env vars — preferred) ────────────────────────────
# OTEL_TRACES_SAMPLER selects the sampler strategy.
# OTEL_TRACES_SAMPLER_ARG sets the ratio for ratio-based samplers (0.0–1.0).
# These take precedence over TRACE_SAMPLE_RATE when set.
#
# Default for production: traceidratio at 10% (0.1)
OTEL_TRACES_SAMPLER=traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1

# Supported OTEL_TRACES_SAMPLER values:
#   always_on                — 100% sampling
#   always_off               — 0% sampling
#   traceidratio             — ratio set by OTEL_TRACES_SAMPLER_ARG
#   parentbased_always_on    — 100% sampling
#   parentbased_always_off   — 0% sampling
#   parentbased_traceidratio — ratio set by OTEL_TRACES_SAMPLER_ARG

# ── Legacy fallback ──────────────────────────────────────────────────────────
# TRACE_SAMPLE_RATE is used when OTEL_TRACES_SAMPLER is not set.
# Default: 0.1 (10%)
TRACE_SAMPLE_RATE=0.1

# Environment name for trace metadata
ENVIRONMENT=development
```

## Quick Start with Jaeger

1. Start Jaeger using Docker Compose:

```bash
docker-compose -f docker-compose.tracing.yml up -d jaeger
```

2. Configure the API to export traces:

```bash
export OTLP_ENDPOINT=http://localhost:4317
export TRACE_SAMPLE_RATE=1.0
```

3. Start the API:

```bash
cargo run
```

4. Open Jaeger UI:

```
http://localhost:16686
```

5. Make some API requests and view traces in Jaeger

## Quick Start with OpenTelemetry Collector

The OpenTelemetry Collector can export to multiple backends simultaneously:

1. Start the full tracing stack:

```bash
docker-compose -f docker-compose.tracing.yml up -d
```

This starts:
- Jaeger (UI at http://localhost:16686)
- Zipkin (UI at http://localhost:9411)
- OpenTelemetry Collector (receives traces and exports to both)

2. Configure the API:

```bash
export OTLP_ENDPOINT=http://localhost:4317
export TRACE_SAMPLE_RATE=1.0
```

## Trace Context Propagation

Traces automatically propagate across service boundaries using W3C Trace Context headers:

- `traceparent`: Contains trace ID, span ID, and sampling decision
- `tracestate`: Vendor-specific trace information

When making HTTP requests to other services, the trace context is automatically injected into request headers.

## Sampling Strategies

### Always On (Development)
```bash
OTEL_TRACES_SAMPLER=always_on
```
Traces 100% of requests. Use for development and debugging.

### Ratio-Based (Production) — default
```bash
OTEL_TRACES_SAMPLER=traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1
```
Traces 10% of requests. This is the **default** when no sampler is configured.
Reduces overhead while maintaining visibility.

### Always Off (Disabled)
```bash
OTEL_TRACES_SAMPLER=always_off
# or unset OTLP_ENDPOINT
```
Disables trace export. Tracing instrumentation remains but spans are not exported.

## Viewing Traces

### Jaeger UI

1. Open http://localhost:16686
2. Select "predictiq-api" from the Service dropdown
3. Click "Find Traces"
4. Click on a trace to view the full request flow

### Zipkin UI

1. Open http://localhost:9411
2. Click "Run Query" to see recent traces
3. Click on a trace to view details

## Trace Attributes

Each span includes:

- `service.name`: "predictiq-api"
- `service.version`: API version from Cargo.toml
- `deployment.environment`: From ENVIRONMENT env var
- HTTP method, path, status code
- Request duration
- Error information (if applicable)

## Integration with Other Services

To propagate traces to downstream services:

```rust
use crate::tracing_config::{extract_trace_context, inject_trace_context};

// Extract context from incoming request
let context = extract_trace_context(&headers);

// Inject context into outgoing request
let mut headers = reqwest::header::HeaderMap::new();
inject_trace_context(&mut headers, &context);

let response = client
    .get("http://downstream-service/api")
    .headers(headers)
    .send()
    .await?;
```

## Production Considerations

### Sampling

In production, use a low sampling rate (0.01–0.1) to reduce overhead.
The default is **10 %** when no sampler env vars are set.

```bash
# 10% (default)
OTEL_TRACES_SAMPLER=traceidratio
OTEL_TRACES_SAMPLER_ARG=0.1

# 5%
OTEL_TRACES_SAMPLER=traceidratio
OTEL_TRACES_SAMPLER_ARG=0.05
```

### Backend

Use a managed tracing backend:
- Jaeger (self-hosted or managed)
- Zipkin
- AWS X-Ray
- Google Cloud Trace
- Datadog APM
- New Relic

### Performance

Tracing adds minimal overhead:
- ~1-2ms per traced request
- Batched export reduces network calls
- Memory-limited to prevent OOM

### Security

- Traces may contain sensitive data (URLs, headers)
- Configure backend access controls
- Consider PII scrubbing in production
- Use TLS for OTLP export in production

## Troubleshooting

### Traces not appearing

1. Check `otel_export_errors_total` in `/metrics` — any non-zero value means
   the startup connectivity check failed.

2. Check OTLP endpoint is reachable:
```bash
curl http://localhost:4317
```

3. Check API logs for tracing initialization:
```
Distributed tracing initialized service_name="predictiq-api"
```

4. Verify sampling rate is > 0:
```bash
echo $OTEL_TRACES_SAMPLER_ARG   # OTel standard
echo $TRACE_SAMPLE_RATE          # legacy fallback
```

### High memory usage

Reduce batch size or increase export frequency in `otel-collector-config.yml`:

```yaml
processors:
  batch:
    timeout: 5s  # Export more frequently
    send_batch_size: 512  # Smaller batches
```

### Missing spans

Ensure trace context is propagated:
- Check `traceparent` header is present in requests
- Verify downstream services support W3C Trace Context

## References

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [Zipkin Documentation](https://zipkin.io/)
