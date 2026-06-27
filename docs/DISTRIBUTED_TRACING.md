# Distributed Tracing Configuration

This document describes the distributed tracing implementation across PredictIQ services.

## Overview

Distributed tracing is implemented using OpenTelemetry with support for Jaeger and Zipkin exporters. Trace context is propagated across service boundaries to enable end-to-end request tracking.

## Architecture

- **API Service (Rust)**: Uses `tracing-opentelemetry` with OTLP exporter
- **TTS Service (TypeScript)**: Uses `@opentelemetry/sdk-node` with OTLP exporter
- **Trace Propagation**: W3C Trace Context standard via HTTP headers

## Configuration

### Environment Variables

All services support the following environment variables:

- `OTEL_EXPORTER_OTLP_ENDPOINT`: OTLP collector endpoint (default: `http://localhost:4317`)
- `OTEL_SERVICE_NAME`: Service name for traces (default: service-specific)
- `OTEL_TRACE_SAMPLING_RATIO`: Sampling rate 0.0-1.0 (default: `1.0`)
- `RUST_LOG`: Log level for Rust services (default: `info`)

### OTLP Collector Configuration

The OpenTelemetry collector (`otel-collector-config.yml`) supports environment variable substitution for exporter endpoints:

- `JAEGER_ENDPOINT`: Jaeger exporter endpoint (default: `jaeger:14250`)
- `ZIPKIN_ENDPOINT`: Zipkin exporter endpoint (default: `http://zipkin:9411/api/v2/spans`)

**Example: Production Configuration**

```bash
# Set custom endpoints for production
export JAEGER_ENDPOINT=jaeger.prod.internal:14250
export ZIPKIN_ENDPOINT=http://zipkin.prod.internal:9411/api/v2/spans

# Start the tracing stack
docker-compose -f docker-compose.tracing.yml up
```

**Example: Development Configuration**

```bash
# Use default local endpoints
docker-compose -f docker-compose.tracing.yml up
```

### API Service Configuration

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
export OTEL_SERVICE_NAME=predictiq-api
export OTEL_TRACE_SAMPLING_RATIO=0.1
export RUST_LOG=info
```

### TTS Service Configuration

```bash
export OTEL_EXPORTER_OTLP_ENDPOINT=http://jaeger:4317
export OTEL_SERVICE_NAME=predictiq-tts
export OTEL_TRACE_SAMPLING_RATIO=0.1
```

## Deployment

### Using Jaeger (Recommended)

```yaml
# docker-compose.yml
services:
  jaeger:
    image: jaegertracing/all-in-one:latest
    ports:
      - "16686:16686"  # Jaeger UI
      - "4317:4317"    # OTLP gRPC receiver
      - "4318:4318"    # OTLP HTTP receiver
    environment:
      - COLLECTOR_OTLP_ENABLED=true
```

Access Jaeger UI at: http://localhost:16686

### Using Zipkin

```yaml
# docker-compose.yml
services:
  zipkin:
    image: openzipkin/zipkin:latest
    ports:
      - "9411:9411"
  
  otel-collector:
    image: otel/opentelemetry-collector:latest
    command: ["--config=/etc/otel-collector-config.yaml"]
    volumes:
      - ./otel-collector-config.yaml:/etc/otel-collector-config.yaml
    ports:
      - "4317:4317"
```

Access Zipkin UI at: http://localhost:9411

## Trace Context Propagation

Trace context is automatically propagated via HTTP headers using W3C Trace Context:

- `traceparent`: Contains trace ID, span ID, and sampling decision
- `tracestate`: Vendor-specific trace information

### Example: Frontend to API

```typescript
// Frontend makes request with trace context
const response = await fetch('/api/markets/featured', {
  headers: {
    'traceparent': '00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01'
  }
});
```

### Example: API to Blockchain

Trace context is automatically injected into outgoing HTTP requests via the OpenTelemetry instrumentation.

## Sampling Strategies

### Production (Low Volume)
```bash
OTEL_TRACE_SAMPLING_RATIO=0.1  # Sample 10% of traces
```

### Staging/Testing
```bash
OTEL_TRACE_SAMPLING_RATIO=1.0  # Sample 100% of traces
```

### High-Traffic Production
```bash
OTEL_TRACE_SAMPLING_RATIO=0.01  # Sample 1% of traces
```

## Custom Spans

### Rust (API Service)

```rust
use tracing::{info_span, instrument};

#[instrument(skip(state))]
async fn my_handler(state: State<AppState>) -> Result<Json<Response>> {
    let span = info_span!("database_query");
    let _guard = span.enter();
    
    // Your code here
    
    Ok(Json(response))
}
```

### TypeScript (TTS Service)

```typescript
import { trace } from "@opentelemetry/api";

const tracer = trace.getTracer("tts-service");

async function processJob(job: TTSJob) {
  return tracer.startActiveSpan("process_job", async (span) => {
    try {
      span.setAttribute("job.id", job.id);
      span.setAttribute("job.provider", job.provider);
      
      // Your code here
      
      span.setStatus({ code: SpanStatusCode.OK });
    } catch (error) {
      span.setStatus({ code: SpanStatusCode.ERROR, message: String(error) });
      throw error;
    } finally {
      span.end();
    }
  });
}
```

## Correlating Traces with Logs

OpenTelemetry injects `trace_id` and `span_id` into the active context. Both services are configured to emit these fields in structured log output so every log line can be linked back to its trace.

### How trace IDs appear in logs

**API Service (Rust)** — `tracing-opentelemetry` automatically attaches the active span's trace/span IDs to each `tracing` event. With a JSON formatter the output looks like:

```json
{
  "timestamp": "2024-08-15T12:34:56.789Z",
  "level": "INFO",
  "message": "market resolved",
  "trace_id": "4bf92f3577b34da6a3ce929d0e0e4736",
  "span_id": "00f067aa0ba902b7",
  "service.name": "predictiq-api"
}
```

**TTS Service (TypeScript)** — spans created via `@opentelemetry/api` expose IDs through the active context:

```typescript
import { trace, context } from "@opentelemetry/api";

function logWithTrace(message: string, extra: Record<string, unknown> = {}) {
  const span = trace.getActiveSpan();
  const spanContext = span?.spanContext();
  console.log(JSON.stringify({
    timestamp: new Date().toISOString(),
    message,
    trace_id: spanContext?.traceId ?? "none",
    span_id: spanContext?.spanId ?? "none",
    ...extra,
  }));
}
```

### Finding related logs from a trace ID

Copy the `traceId` from the Jaeger UI (trace detail view → top-level span header).

#### CloudWatch Logs Insights

```sql
-- All log lines for a specific trace
fields @timestamp, message, span_id, service_name
| filter trace_id = "4bf92f3577b34da6a3ce929d0e0e4736"
| sort @timestamp asc
| limit 200
```

```sql
-- Error logs for traces in the last hour
fields @timestamp, message, trace_id, span_id
| filter level = "ERROR"
| filter ispresent(trace_id)
| sort @timestamp desc
| limit 100
```

```sql
-- Latency distribution grouped by trace
stats count(*) as log_count, min(@timestamp) as start, max(@timestamp) as end
| by trace_id
| sort log_count desc
```

Run queries from the CloudWatch console → **Log Insights** → select the `/predictiq/api` and `/predictiq/tts` log groups simultaneously to see correlated output from both services in one result set.

#### Jaeger → CloudWatch workflow

1. Open the Jaeger UI at `http://localhost:16686` and find the slow or erroring trace.
2. Copy the **Trace ID** from the URL or the trace header (e.g., `4bf92f3577b34da6a3ce929d0e0e4736`).
3. Paste it into the CloudWatch Logs Insights `trace_id` filter above.
4. The result shows every structured log line emitted during that request's lifetime, across all services.

## Troubleshooting

### No traces appearing in Jaeger

1. Check OTLP endpoint is reachable:
   ```bash
   curl http://localhost:4317
   ```

2. Verify environment variables are set correctly

3. Check service logs for OpenTelemetry errors

4. Ensure sampling ratio is not 0.0

### Traces not connected across services

1. Verify trace context headers are being propagated
2. Check that all services use the same OTLP endpoint
3. Ensure W3C Trace Context propagation is enabled

### High memory usage

1. Reduce sampling ratio
2. Configure batch span processor limits
3. Enable span compression in OTLP exporter

## Metrics

Key metrics to monitor:

- `otel.traces.exported`: Number of traces exported
- `otel.traces.dropped`: Number of traces dropped
- `otel.exporter.queue.size`: Export queue size
- `otel.exporter.latency`: Export latency

## References

- [OpenTelemetry Documentation](https://opentelemetry.io/docs/)
- [Jaeger Documentation](https://www.jaegertracing.io/docs/)
- [W3C Trace Context](https://www.w3.org/TR/trace-context/)
