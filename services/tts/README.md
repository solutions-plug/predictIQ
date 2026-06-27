# TTS Service

AI text-to-speech service for social video narrations. Supports ElevenLabs (primary) and Google Cloud TTS (fallback).

## Features

- **Request Authentication** (Issue #723): Requests authenticated via API key header
- **Audio Caching** (Issue #724): Generated audio cached by content hash with configurable TTL
- **Input Validation** (Issue #725): Input text length capped at 5000 characters (configurable)
- **Distributed Tracing** (Issue #726): W3C Trace Context propagation for end-to-end observability

## Configuration

### Environment Variables

#### Authentication (Issue #723)

- `TTS_API_KEY`: Comma-separated list of valid API keys. If set, all TTS endpoints require authentication.

Example:
```bash
export TTS_API_KEY="key1,key2,key3"
```

#### Rate Limiting

- `TTS_RATE_LIMIT_MAX`: Max requests per window (default: 100)
- `TTS_RATE_LIMIT_WINDOW_MS`: Window duration in milliseconds (default: 60000)

#### Caching (Issue #724)

- `TTS_CACHE_TTL_MS`: Cache TTL in milliseconds (default: 86400000 = 24 hours)
- `TTS_CACHE_MAX_ENTRIES`: Max cache entries before eviction (default: 1000)

#### Input Validation (Issue #725)

- `TTS_MAX_INPUT_LENGTH`: Max input text length in characters (default: 5000)

#### Tracing (Issue #726)

- `OTEL_EXPORTER_OTLP_ENDPOINT`: OpenTelemetry collector endpoint (default: http://localhost:4317)
- `OTEL_SERVICE_NAME`: Service name for tracing (default: predictiq-tts)
- `OTEL_TRACE_SAMPLING_RATIO`: Trace sampling ratio 0-1 (default: 1.0)

#### Providers

- `TTS_PROVIDER`: Primary provider - "elevenlabs" or "google" (default: elevenlabs)
- `ELEVENLABS_API_KEY`: ElevenLabs API key
- `ELEVENLABS_MODEL_ID`: ElevenLabs model ID (default: eleven_multilingual_v2)
- `GOOGLE_APPLICATION_CREDENTIALS`: Path to Google Cloud credentials JSON
- `TTS_OUTPUT_DIR`: Output directory for generated audio (default: /tmp/tts-output)

## API Endpoints

### Health Checks

- `GET /health` — Comprehensive health check (200 if healthy, 503 if degraded)
- `GET /health/ready` — Kubernetes readiness probe
- `GET /health/live` — Kubernetes liveness probe

### TTS Endpoints

All TTS endpoints require authentication if `TTS_API_KEY` is configured.

#### POST /tts/enqueue

Enqueue a TTS job and return immediately with job ID.

**Request:**
```bash
curl -X POST http://localhost:3000/tts/enqueue \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Hello world",
    "voiceId": "el-rachel-en",
    "provider": "elevenlabs"
  }'
```

**Response:**
```json
{
  "jobId": "tts_1234567890_abc123",
  "status": "pending"
}
```

**Headers:**
- `Authorization: Bearer <api-key>` — Required if auth configured
- `Cache-Control: no-cache` — Optional, bypass cache for this request

#### GET /tts/job/:id

Get the status and details of a TTS job.

**Request:**
```bash
curl http://localhost:3000/tts/job/tts_1234567890_abc123 \
  -H "Authorization: Bearer YOUR_API_KEY"
```

**Response:**
```json
{
  "id": "tts_1234567890_abc123",
  "text": "Hello world",
  "status": "done",
  "outputPath": "/tmp/tts-output/tts_1234567890_abc123.mp3",
  "createdAt": "2024-01-15T10:30:00Z",
  "updatedAt": "2024-01-15T10:30:05Z"
}
```

#### GET /tts/jobs

List all jobs, optionally filtered by status.

**Request:**
```bash
curl "http://localhost:3000/tts/jobs?status=done" \
  -H "Authorization: Bearer YOUR_API_KEY"
```

**Query Parameters:**
- `status`: Filter by status - "pending" | "processing" | "done" | "error"

#### POST /tts/generate

Synchronous generation — waits for completion and returns the output path.

**Request:**
```bash
curl -X POST http://localhost:3000/tts/generate \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Content-Type: application/json" \
  -d '{
    "text": "Hello world",
    "voiceId": "el-rachel-en"
  }'
```

**Response:**
```json
{
  "outputPath": "/tmp/tts-output/tts_1234567890_abc123.mp3"
}
```

**Headers:**
- `Authorization: Bearer <api-key>` — Required if auth configured
- `Cache-Control: no-cache` — Optional, bypass cache for this request

#### GET /tts/voices

List available voices.

**Request:**
```bash
curl http://localhost:3000/tts/voices
```

**Response:**
```json
{
  "el-rachel-en": {
    "voiceId": "21m00Tcm4TlvDq8ikWAM",
    "language": "en-US",
    "label": "Rachel (EN)"
  },
  ...
}
```

## Error Responses

### 400 Bad Request

- Missing required fields
- Input text exceeds maximum length (Issue #725)
- Invalid voice ID

```json
{
  "error": "Input text exceeds maximum length of 5000 characters"
}
```

### 401 Unauthorized

- Missing or invalid API key (Issue #723)

```json
{
  "error": "Invalid API key"
}
```

### 429 Too Many Requests

- Rate limit exceeded

```json
{
  "error": "Rate limit exceeded for key \"ip:1.2.3.4\": 101/100 in 60000ms"
}
```

### 502 Bad Gateway

- Provider error (ElevenLabs or Google Cloud TTS)

```json
{
  "error": "[elevenlabs] ElevenLabs HTTP 429: Rate limit exceeded"
}
```

## Caching (Issue #724)

Audio is cached by a SHA256 hash of `provider:voiceId:text`. Cache hits skip the provider API call entirely.

**Cache Key Formula:**
```
sha256(provider + ":" + voiceId + ":" + text)
```

**Bypass Cache:**
```bash
curl -X POST http://localhost:3000/tts/generate \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "Cache-Control: no-cache" \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello", "voiceId": "el-rachel-en"}'
```

**Cache Metrics:**
```bash
curl http://localhost:3000/metrics/cache \
  -H "Authorization: Bearer YOUR_API_KEY"
```

## Distributed Tracing (Issue #726)

The TTS service extracts and propagates W3C Trace Context (`traceparent` header) from incoming requests. This enables end-to-end trace correlation between the API service and TTS service.

**Trace Context Propagation:**
```bash
curl -X POST http://localhost:3000/tts/generate \
  -H "Authorization: Bearer YOUR_API_KEY" \
  -H "traceparent: 00-0af7651916cd43dd8448eb211c80319c-b7ad6b7169203331-01" \
  -H "Content-Type: application/json" \
  -d '{"text": "Hello", "voiceId": "el-rachel-en"}'
```

The TTS service will:
1. Extract the `traceparent` header
2. Create a child span linked to the parent trace
3. Export the span to the configured OpenTelemetry collector

**Viewing Traces:**
- Jaeger UI: http://localhost:16686
- Tempo UI: http://localhost:3000 (if configured)

## Development

### Build

```bash
npm run build
```

### Run

```bash
npm run dev
```

### Test

```bash
npm test
```

## Security Considerations

- **Authentication**: Always set `TTS_API_KEY` in production to prevent unauthorized access
- **Input Validation**: Input text is limited to 5000 characters to prevent DoS attacks
- **Rate Limiting**: Requests are rate-limited per IP address to prevent abuse
- **SSML Injection**: Input text is sanitized to strip XML/SSML tags

## Performance Considerations

- **Caching**: Audio is cached for 24 hours by default, reducing API calls and latency
- **Rate Limiting**: Default limit is 100 requests per 60 seconds per IP
- **Async Processing**: Jobs are processed asynchronously to avoid blocking requests
