# PredictIQ Observability Configuration

This directory contains configuration files for monitoring and observability of the PredictIQ system.

## Files

### grafana-dashboard.json
Grafana dashboard configuration that provides visual monitoring of key system metrics:

- **API Performance**: Response times (p95, p99), error rates, throughput
- **Cache Performance**: Hit rates and efficiency
- **Database Performance**: Query times, connection pool utilization
- **Contract Performance**: Gas costs for different operations
- **System Health**: Overall service status

### grafana-slo-dashboard.json
Grafana dashboard for SLO compliance and error budget tracking:

- **Error Budget Status**: Remaining budget per SLO
- **Burn Rate**: How fast error budget is being consumed
- **SLO Compliance**: Percentage of time meeting targets
- **Trend Analysis**: Historical performance trends

### grafana-provisioning.yaml
Grafana provisioning configuration for automatic dashboard loading from the repository.

### alerts.yaml
Prometheus/Alertmanager alert rules for critical system thresholds:

- **API Alerts**: High response times, error rates, low throughput
- **Cache Alerts**: Low hit rates
- **Database Alerts**: Slow queries, high connection pool utilization
- **Contract Alerts**: High gas costs
- **System Alerts**: Service downtime, high resource usage
- **Regression Alerts**: Performance degradation detection

### thresholds.json
Performance threshold definitions used by testing and monitoring:

- Backend response time targets (p95, p99, avg)
- Error rate limits
- Throughput minimums
- Cache hit rate requirements
- Contract gas cost limits
- Database query time targets
- Regression detection thresholds

## Setup

### Grafana Dashboard Provisioning

Dashboards are version-controlled and automatically loaded by Grafana on startup.

#### Docker Compose Setup

```yaml
services:
  grafana:
    image: grafana/grafana:latest
    volumes:
      - ./performance/config/grafana-provisioning.yaml:/etc/grafana/provisioning/dashboards/dashboards.yaml
      - ./performance/config/:/var/lib/grafana/dashboards/
    environment:
      - GF_SECURITY_ADMIN_PASSWORD=admin
    ports:
      - "3000:3000"
```

#### Kubernetes Setup

```yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: grafana-provisioning
data:
  dashboards.yaml: |
    apiVersion: 1
    providers:
      - name: 'PredictIQ Dashboards'
        orgId: 1
        folder: 'Performance'
        type: file
        disableDeletion: false
        updateIntervalSeconds: 10
        allowUiUpdates: true
        options:
          path: /var/lib/grafana/dashboards

---
apiVersion: apps/v1
kind: Deployment
metadata:
  name: grafana
spec:
  template:
    spec:
      containers:
      - name: grafana
        image: grafana/grafana:latest
        volumeMounts:
        - name: provisioning
          mountPath: /etc/grafana/provisioning/dashboards
        - name: dashboards
          mountPath: /var/lib/grafana/dashboards
      volumes:
      - name: provisioning
        configMap:
          name: grafana-provisioning
      - name: dashboards
        configMap:
          name: grafana-dashboards
```

### Exporting Dashboard Changes

When you make changes to dashboards in the Grafana UI, export them back to the repository:

```bash
# Set your Grafana API key
export GRAFANA_API_KEY=your-api-key-here

# Export dashboards
node ../scripts/export-grafana-dashboards.js

# Review changes
git diff grafana-*.json

# Commit and push
git add grafana-*.json
git commit -m "chore: update Grafana dashboards"
git push
```

#### Getting a Grafana API Key

1. Log in to Grafana as an admin
2. Navigate to Configuration → API Keys
3. Click "New API Key"
4. Set Role to "Admin"
5. Copy the generated key

#### Workflow for Dashboard Updates

1. **Make changes in Grafana UI**
   - Add panels, modify queries, adjust colors, etc.
   - Test the dashboard thoroughly

2. **Export to repository**
   ```bash
   GRAFANA_API_KEY=your-key node ../scripts/export-grafana-dashboards.js
   ```

3. **Review and commit**
   ```bash
   git diff grafana-*.json  # Review changes
   git add grafana-*.json
   git commit -m "feat: add new performance panel to dashboard"
   ```

4. **Deploy**
   - Push to main branch
   - Grafana will automatically reload dashboards on next startup
   - Or manually reload via Grafana UI

### Manual Dashboard Import

If provisioning is not available:

1. Import the dashboard into Grafana:
   ```bash
   curl -X POST http://grafana:3000/api/dashboards/db \
     -H "Content-Type: application/json" \
     -H "Authorization: Bearer YOUR_API_KEY" \
     -d @grafana-dashboard.json
   ```

2. Or manually import via Grafana UI:
   - Navigate to Dashboards → Import
   - Upload `grafana-dashboard.json`

### Prometheus Alerts

1. Add alerts to Prometheus configuration:
   ```yaml
   # prometheus.yml
   rule_files:
     - "alerts.yaml"
   ```

2. Configure Alertmanager for notifications:
   ```yaml
   # alertmanager.yml
   route:
     receiver: 'team-notifications'
     group_by: ['alertname', 'component']
     group_wait: 30s
     group_interval: 5m
     repeat_interval: 4h
   
   receivers:
     - name: 'team-notifications'
       slack_configs:
         - api_url: 'YOUR_SLACK_WEBHOOK_URL'
           channel: '#predictiq-alerts'
   ```

## Metrics Collection

Ensure your application exports the following metrics:

### HTTP Metrics
- `http_request_duration_seconds_bucket` - Request duration histogram
- `http_requests_total` - Total request counter with status labels

### Cache Metrics
- `cache_hits_total` - Cache hit counter
- `cache_misses_total` - Cache miss counter

### Database Metrics
- `db_query_duration_seconds_bucket` - Query duration histogram
- `db_connections_active` - Active connections gauge
- `db_connections_max` - Maximum connections gauge

### Contract Metrics
- `contract_gas_used` - Gas usage gauge with operation labels

### System Metrics
- `up` - Service availability (1 = up, 0 = down)
- `node_memory_*` - Memory metrics
- `node_cpu_seconds_total` - CPU metrics

## Alert Severity Levels

- **Critical**: Immediate action required, system functionality impaired
- **Warning**: Attention needed, potential issues developing
- **Info**: Informational, no immediate action required

## Customization

### Adjusting Thresholds

Edit `thresholds.json` to modify performance targets:

```json
{
  "backend": {
    "response_time": {
      "p95": 200,  // Adjust as needed
      "p99": 500,
      "avg": 150
    }
  }
}
```

### Adding New Panels

To add new panels to the Grafana dashboard:

1. Edit `grafana-dashboard.json`
2. Add a new panel object to the `panels` array
3. Configure queries, visualization, and alerts
4. Re-import the dashboard

### Adding New Alerts

To add new alert rules:

1. Edit `alerts.yaml`
2. Add a new rule under the appropriate group
3. Define the PromQL expression, duration, and annotations
4. Reload Prometheus configuration

## Monitoring Best Practices

1. **Set realistic thresholds** based on actual system performance
2. **Use percentiles** (p95, p99) instead of averages for latency
3. **Configure alert fatigue prevention** with appropriate `for` durations
4. **Group related alerts** to avoid notification spam
5. **Document alert runbooks** for incident response
6. **Review and adjust** thresholds regularly based on system evolution

## Integration with CI/CD

The thresholds defined here are used by:

- Performance tests in `performance/backend/k6/`
- Regression detection in `performance/scripts/compare-results.js`
- Automated performance gates in CI pipelines

## Support

For questions or issues with observability configuration:
- Check Grafana documentation: https://grafana.com/docs/
- Check Prometheus documentation: https://prometheus.io/docs/
- Review performance test results in `performance/` directory
