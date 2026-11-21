# TTS Service Health Check Endpoints

The TTS service provides comprehensive health check endpoints for monitoring service availability and dependency status. These endpoints are designed to integrate with load balancers, Kubernetes probes, and monitoring systems.

## Endpoints

### 1. Comprehensive Health Check

**Endpoint:** `GET /health`

**Purpose:** Full health check with detailed dependency verification. Returns the overall service status and individual checks for each dependency.

**Response (200 OK - Healthy):**

```json
{
  "status": "healthy",
  "timestamp": "2024-01-15T10:30:00.000Z",
  "uptime": 3600000,
  "checks": {
    "service": {
      "status": "ok",
      "message": "TTS service is running"
    },
    "outputDirectory": {
      "status": "ok",
      "message": "Output directory is writable: /tmp/tts-output",
      "latency": 5
    },
    "elevenlabs": {
      "status": "ok",
      "message": "ElevenLabs is configured and accessible",
      "latency": 10
    },
    "jobStore": {
      "status": "ok",
      "message": "Job store is functional (42 jobs in memory)"
    }
  },
  "message": "✅ All systems operational"
}
```

**Response (503 Service Unavailable - Unhealthy):**

```json
{
  "status": "unhealthy",
  "timestamp": "2024-01-15T10:30:00.000Z",
  "uptime": 3600000,
  "checks": {
    "service": {
      "status": "ok",
      "message": "TTS service is running"
    },
    "outputDirectory": {
      "status": "error",
      "message": "Output directory check failed: EACCES: permission denied",
      "latency": 2
    },
    "elevenlabs": {
      "status": "error",
      "message": "ElevenLabs API key appears invalid"
    },
    "jobStore": {
      "status": "ok",
      "message": "Job store is functional (0 jobs in memory)"
    }
  },
  "message": "❌ Service is unhealthy\n  - outputDirectory: Output directory check failed: EACCES: permission denied\n  - elevenlabs: ElevenLabs API key appears invalid"
}
```

**Status Codes:**

- `200 OK` — Service is healthy and ready to accept requests
- `503 Service Unavailable` — Service is degraded or unhealthy

**Use Case:** Detailed monitoring dashboards, alerting systems, and manual health verification.

---

### 2. Readiness Probe

**Endpoint:** `GET /health/ready`

**Purpose:** Lightweight readiness check for Kubernetes readiness probes. Indicates whether the service is ready to accept traffic.

**Response (200 OK - Ready):**

```json
{
  "status": "ready",
  "message": "Service is ready to accept requests",
  "timestamp": "2024-01-15T10:30:00.000Z"
}
```

**Response (503 Service Unavailable - Not Ready):**

```json
{
  "status": "not_ready",
  "message": "Service not ready: Output directory not writable",
  "timestamp": "2024-01-15T10:30:00.000Z"
}
```

**Status Codes:**

- `200 OK` — Service is ready
- `503 Service Unavailable` — Service is not ready

**Use Case:** Kubernetes readiness probes. When this endpoint returns 503, the pod is removed from the service load balancer.

**Kubernetes Configuration:**

```yaml
readinessProbe:
  httpGet:
    path: /health/ready
    port: 3000
  initialDelaySeconds: 10
  periodSeconds: 5
  timeoutSeconds: 2
  failureThreshold: 3
```

---

### 3. Liveness Probe

**Endpoint:** `GET /health/live`

**Purpose:** Lightweight liveness check for Kubernetes liveness probes. Indicates whether the service process is alive.

**Response (200 OK - Alive):**

```json
{
  "status": "alive",
  "message": "Service process is alive",
  "timestamp": "2024-01-15T10:30:00.000Z"
}
```

**Response (503 Service Unavailable - Dead):**

```json
{
  "status": "dead",
  "message": "Liveness check failed: Service process crashed",
  "timestamp": "2024-01-15T10:30:00.000Z"
}
```

**Status Codes:**

- `200 OK` — Service is alive
- `503 Service Unavailable` — Service is dead

**Use Case:** Kubernetes liveness probes. When this endpoint returns 503, the pod is restarted.

**Kubernetes Configuration:**

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 3000
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 2
  failureThreshold: 3
```

---

## Health Check Details

### Service Status

Verifies that the TTS service instance is initialized and functional.

### Output Directory

Checks that the configured output directory:

- Exists or can be created
- Is writable (performs a test write/delete)
- Has sufficient permissions

### ElevenLabs (if configured)

Checks that:

- API key is configured
- API key format is valid
- Service is accessible

### Google TTS (if configured)

Checks that:

- Credentials are configured
- Credential file exists (if using keyFilename)
- Service is accessible

### Job Store

Verifies that the in-memory job store is functional and reports the number of jobs currently in memory.

---

## Integration with Load Balancers

### AWS Application Load Balancer (ALB)

```hcl
resource "aws_lb_target_group" "tts_service" {
  name             = "tts-service"
  port             = 3000
  protocol         = "HTTP"
  vpc_id           = aws_vpc.main.id
  target_type      = "ip"

  health_check {
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 3
    interval            = 30
    path                = "/health"
    matcher             = "200"
  }
}
```

### Nginx Upstream

```nginx
upstream tts_service {
  server tts-1:3000;
  server tts-2:3000;
  server tts-3:3000;

  check interval=3000 rise=2 fall=5 timeout=1000 type=http;
  check_http_send "GET /health HTTP/1.0\r\n\r\n";
  check_http_expect_alive http_2xx;
}
```

### HAProxy

```haproxy
backend tts_service
  balance roundrobin
  option httpchk GET /health
  server tts-1 tts-1:3000 check
  server tts-2 tts-2:3000 check
  server tts-3 tts-3:3000 check
```

---

## Monitoring and Alerting

### Prometheus Metrics

Export health check results as Prometheus metrics:

```typescript
import { register, Counter, Gauge } from "prom-client";

const healthCheckCounter = new Counter({
  name: "tts_health_checks_total",
  help: "Total number of health checks",
  labelNames: ["status"],
});

const healthCheckLatency = new Gauge({
  name: "tts_health_check_latency_ms",
  help: "Health check latency in milliseconds",
  labelNames: ["check"],
});

app.get("/metrics", (req, res) => {
  res.set("Content-Type", register.contentType);
  res.end(register.metrics());
});
```

### Grafana Dashboard

Create a dashboard to visualize:

- Health check status over time
- Dependency availability
- Response latencies
- Error rates

### Alert Rules

```yaml
groups:
  - name: tts_service
    rules:
      - alert: TTSServiceUnhealthy
        expr: tts_health_status != 1
        for: 2m
        annotations:
          summary: "TTS service is unhealthy"

      - alert: TTSOutputDirectoryError
        expr: tts_health_check_status{check="outputDirectory"} != 1
        for: 1m
        annotations:
          summary: "TTS output directory is not writable"

      - alert: TTSProviderUnavailable
        expr: tts_health_check_status{check=~"elevenlabs|google"} != 1
        for: 5m
        annotations:
          summary: "TTS provider is unavailable"
```

---

## Deployment Integration

### Docker Health Check

The runtime image does not include `curl`, so the container health check must use a probe that ships with the runtime stage. The check targets the lightweight liveness route (`/health/live`) rather than the comprehensive `/health` route, and uses conservative intervals to avoid restart churn.

```dockerfile
FROM node:20-alpine

WORKDIR /app
COPY . .
RUN npm install && npm run build

EXPOSE 3000

# Probe the lightweight liveness route using a client present in the runtime image.
# Do not use curl (not installed) or /health (heavy dependency checks).
HEALTHCHECK --interval=30s --timeout=5s --start-period=30s --retries=3 \
  CMD node -e "require('http').get('http://localhost:3000/health/live', (r) => process.exit(r.statusCode === 200 ? 0 : 1)).on('error', () => process.exit(1))"

CMD ["npm", "start"]
```

**Probe conventions:**

- **Liveness (`/health/live`)** — used for the container `HEALTHCHECK`; very fast, indicates the process is alive.
- **Readiness (`/health/ready`)** — used by orchestrators to gate traffic; fast, indicates the service can accept requests.
- **Comprehensive (`/health`)** — for dashboards and alerting only; performs dependency checks and must not be used for liveness probes.
- **Intervals/timeouts** — keep `--interval` at 30s or higher, `--timeout` at 5s, and `--start-period` at 30s so slow starts do not trigger restarts.
- **Client availability** — only use HTTP clients guaranteed to exist in the runtime stage (e.g. `node`, `wget`); avoid `curl` unless explicitly installed.

### Kubernetes Deployment

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: tts-service
spec:
  replicas: 3
  selector:
    matchLabels:
      app: tts-service
  template:
    metadata:
      labels:
        app: tts-service
    spec:
      containers:
        - name: tts-service
          image: predictiq/tts-service:latest
          ports:
            - containerPort: 3000
          env:
            - name: TTS_PROVIDER
              value: "elevenlabs"
            - name: ELEVENLABS_API_KEY
              valueFrom:
                secretKeyRef:
                  name: tts-secrets
                  key: elevenlabs-api-key
            - name: TTS_OUTPUT_DIR
              value: "/data/tts-output"
          volumeMounts:
            - name: tts-output
              mountPath: /data/tts-output
          readinessProbe:
            httpGet:
              path: /health/ready
              port: 3000
            initialDelaySeconds: 10
            periodSeconds: 5
            timeoutSeconds: 2
            failureThreshold: 3
          livenessProbe:
            httpGet:
              path: /health/live
              port: 3000
            initialDelaySeconds: 30
            periodSeconds: 10
            timeoutSeconds: 2
            failureThreshold: 3
      volumes:
        - name: tts-output
          persistentVolumeClaim:
            claimName: tts-output-pvc
```

---

## Testing Health Checks

### Manual Testing

```bash
# Comprehensive health check
curl -v http://localhost:3000/health

# Readiness probe
curl -v http://localhost:3000/health/ready

# Liveness probe
curl -v http://localhost:3000/health/live
```

### Verifying the Container Health Check

The runtime image does not include `curl`, so verify the built-in `HEALTHCHECK` (which probes `/health/live`) instead:

```bash
# Inspect the configured health check command
docker inspect --format '{{json .Config.Healthcheck}}' tts-service

# Confirm the container reports healthy within the start period
docker inspect --format '{{.State.Health.Status}}' tts-service
```

### Automated Testing

```typescript
import { describe, it, expect } from "@jest/globals";
import request from "supertest";
import app from "./server";

describe("Health Check Endpoints", () => {
  it("GET /health returns 200 when healthy", async () => {
    const res = await request(app).get("/health");
    expect(res.status).toBe(200);
    expect(res.body.status).toBe("healthy");
  });

  it("GET /health/ready returns 200 when ready", async () => {
    const res = await request(app).get("/health/ready");
    expect(res.status).toBe(200);
    expect(res.body.status).toBe("ready");
  });

  it("GET /health/live returns 200 when alive", async () => {
    const res = await request(app).get("/health/live");
    expect(res.status).toBe(200);
    expect(res.body.status).toBe("alive");
  });
});
```

---

## Troubleshooting

### Health Check Returns 503

1. **Check output directory permissions:**

   ```bash
   ls -la /tmp/tts-output
   chmod 755 /tmp/tts-output
   ```

2. **Verify API keys:**

   ```bash
   echo $ELEVENLABS_API_KEY
   echo $GOOGLE_APPLICATION_CREDENTIALS
   ```

3. **Check service logs:**
   ```bash
   docker logs tts-service
   ```

### High Latency on Health Checks

- Reduce the number of dependency checks in the comprehensive health check
- Use readiness/liveness probes instead of the full health check
- Check network connectivity to external services

### Health Check Timeout

- Increase the timeout in load balancer configuration
- Reduce the number of checks performed
- Check for resource constraints (CPU, memory)

---

## Best Practices

1. **Use appropriate probes:**
   - Readiness probe: `/health/ready` (fast, indicates traffic readiness)
   - Liveness probe: `/health/live` (very fast, indicates process alive)
   - Monitoring: `/health` (comprehensive, for dashboards)

2. **Configure appropriate intervals:**
   - Readiness: 5-10 second intervals
   - Liveness: 10-30 second intervals
   - Monitoring: 30-60 second intervals

3. **Set reasonable thresholds:**
   - Failure threshold: 2-3 consecutive failures
   - Success threshold: 1-2 consecutive successes

4. **Monitor health check metrics:**
   - Track health check latency
   - Alert on repeated failures
   - Monitor dependency availability

5. **Keep health checks lightweight:**
   - Avoid expensive operations
   - Cache results when appropriate
   - Use timeouts to prevent hanging
