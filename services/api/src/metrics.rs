use std::time::Duration;

use anyhow::Context;
use prometheus::{Encoder, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec, Registry, TextEncoder};

const MAX_LABEL_VALUE_LEN: usize = 48;

fn normalize_label(value: &str) -> String {
    let sanitized: String = value
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect();
    let sanitized = sanitized.trim_matches('_').to_string();
    if sanitized.len() > MAX_LABEL_VALUE_LEN {
        let head = &sanitized[..(MAX_LABEL_VALUE_LEN - 8)];
        format!("{}_hotlbl", head)
    } else {
        sanitized
    }
}

fn normalize_label_values(vals: &[&str]) -> Vec<String> {
    vals.iter().map(|v| normalize_label(v)).collect()
}

#[derive(Clone)]
pub struct Metrics {
    registry: Registry,
    cache_hits: IntCounterVec,
    cache_misses: IntCounterVec,
    invalidations: IntCounterVec,
    request_latency: HistogramVec,
    rpc_errors: IntCounterVec,
    rpc_fallbacks: IntCounterVec,
    db_query_duration: HistogramVec,
    db_timeouts: IntCounterVec,
    db_pool_exhaustion: IntCounterVec,
    ledger_gaps: IntCounterVec,
    email_dlq_size: IntGauge,
    email_queue_depth: IntGauge,
    db_pool_connections_active: IntGaugeVec,
    db_pool_connections_idle: IntGaugeVec,
    db_pool_acquire_duration: HistogramVec,
    rate_limit_rejections: IntCounterVec,
    deprecated_api_calls: IntCounterVec,
    /// Counts authentication failures by failure reason.
    /// Labels: `reason` — one of: "invalid_api_key", "expired_token", "missing_credentials".
    auth_failures: IntCounterVec,
    /// #936: counts how many times the sync worker has been restarted after a panic.
    sync_worker_restarts: prometheus::IntCounter,
    /// #936: timestamp of last heartbeat from the sync worker (unix seconds).
    sync_worker_heartbeat_ts: IntGauge,
    sendgrid_retries: IntCounterVec,
    pub worker_crash_total: IntCounterVec,
    otel_export_errors: IntCounterVec,
    worker_status: IntGaugeVec,
    cache_circuit_breaker_state: IntGaugeVec,
}

impl Metrics {
    pub fn new() -> anyhow::Result<Self> {
        let registry = Registry::new();

        let cache_hits = IntCounterVec::new(
            prometheus::Opts::new("cache_hits_total", "Cache hits by layer and endpoint"),
            &["layer", "endpoint"],
        )
        .context("cache_hits metric")?;

        let cache_misses = IntCounterVec::new(
            prometheus::Opts::new("cache_misses_total", "Cache misses by layer and endpoint"),
            &["layer", "endpoint"],
        )
        .context("cache_misses metric")?;

        let invalidations = IntCounterVec::new(
            prometheus::Opts::new("cache_invalidations_total", "Cache invalidations by scope"),
            &["scope"],
        )
        .context("cache_invalidations metric")?;

        let request_latency = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request latency in seconds",
            )
            .buckets(vec![
                0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
            ]),
            &["route", "status_code"],
        )
        .context("request_latency metric")?;

        let rpc_errors = IntCounterVec::new(
            prometheus::Opts::new("rpc_errors_total", "RPC errors by method"),
            &["method"],
        )
        .context("rpc_errors metric")?;

        let rpc_fallbacks = IntCounterVec::new(
            prometheus::Opts::new(
                "rpc_fallbacks_total",
                "RPC calls that fell back to zero/default payload, by endpoint",
            ),
            &["endpoint"],
        )
        .context("rpc_fallbacks metric")?;

        let db_query_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "db_query_duration_seconds",
                "Database query duration in seconds by query name",
            )
            .buckets(vec![
                0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0,
            ]),
            &["query_name"],
        )
        .context("db_query_duration metric")?;

        let db_timeouts = IntCounterVec::new(
            prometheus::Opts::new("db_timeouts_total", "DB queries that exceeded the timeout, by operation"),
            &["operation"],
        )
        .context("db_timeouts metric")?;

        let db_pool_exhaustion = IntCounterVec::new(
            prometheus::Opts::new(
                "db_pool_exhaustion_total",
                "Number of times the connection pool was exhausted, by pool name",
            ),
            &["pool"],
        )
        .context("db_pool_exhaustion metric")?;

        let email_dlq_size = IntGauge::new(
            "email_dlq_size",
            "Number of email jobs currently in the dead-letter queue",
        )
        .context("email_dlq_size metric")?;

        let email_queue_depth = IntGauge::new(
            "email_queue_depth",
            "Number of email jobs currently in the main queue",
        )
        .context("email_queue_depth metric")?;

        let db_pool_connections_active = IntGaugeVec::new(
            prometheus::Opts::new(
                "db_pool_connections_active",
                "Number of connections currently checked out from the pool",
            ),
            &["pool"],
        )
        .context("db_pool_connections_active metric")?;

        let db_pool_connections_idle = IntGaugeVec::new(
            prometheus::Opts::new(
                "db_pool_connections_idle",
                "Number of idle connections sitting in the pool",
            ),
            &["pool"],
        )
        .context("db_pool_connections_idle metric")?;

        let db_pool_acquire_duration = HistogramVec::new(
            prometheus::HistogramOpts::new(
                "db_pool_acquire_duration_seconds",
                "Time spent waiting to acquire a connection from the pool",
            )
            .buckets(vec![
                0.0001, 0.0005, 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0,
            ]),
            &["pool"],
        )
        .context("db_pool_acquire_duration metric")?;

        let rate_limit_rejections = IntCounterVec::new(
            prometheus::Opts::new(
                "rate_limit_rejections_total",
                "Requests rejected by the rate limiter, by route",
            ),
            &["route"],
        )
        .context("rate_limit_rejections metric")?;

        let deprecated_api_calls = IntCounterVec::new(
            prometheus::Opts::new(
                "deprecated_api_calls_total",
                "API calls using deprecated versions, by version",
            ),
            &["version"],
        )
        .context("deprecated_api_calls metric")?;

        let auth_failures = IntCounterVec::new(
            prometheus::Opts::new(
                "auth_failures_total",
                "Authentication failures by reason (invalid_api_key, expired_token, missing_credentials)",
            ),
            &["reason"],
        )
        .context("auth_failures metric")?;

        let sync_worker_restarts = prometheus::IntCounter::new(
            "blockchain_sync_worker_restarts_total",
            "Number of times the blockchain sync worker has been restarted after a panic",
        )
        .context("sync_worker_restarts metric")?;

        let sync_worker_heartbeat_ts = IntGauge::new(
            "blockchain_sync_worker_last_heartbeat_ts",
            "Unix timestamp of the last heartbeat emitted by the sync worker",
        )
        .context("sync_worker_heartbeat_ts metric")?;

        let ledger_gaps = IntCounterVec::new(
            prometheus::Opts::new(
                "blockchain_ledger_gaps_total",
                "Total ledger sequence gaps detected, labelled by gap_type (restart|sync)",
            ),
            &["gap_type"],
        )
        .context("ledger_gaps metric")?;

        let sendgrid_retries = IntCounterVec::new(
            prometheus::Opts::new("sendgrid_retries_total", "SendGrid send retries by reason"),
            &["reason"],
        )
        .context("sendgrid_retries metric")?;

        let worker_crash_total = IntCounterVec::new(
            prometheus::Opts::new(
                "worker_crash_total",
                "Number of times a background worker has crashed and been restarted, by worker",
            ),
            &["worker"],
        )
        .context("worker_crash_total metric")?;

        let otel_export_errors = IntCounterVec::new(
            prometheus::Opts::new(
                "otel_export_errors_total",
                "OpenTelemetry trace export failures, by reason",
            ),
            &["reason"],
        )
        .context("otel_export_errors metric")?;

        let worker_status = IntGaugeVec::new(
            prometheus::Opts::new(
                "worker_status",
                "Background worker health status (1=running, 0=stopped)",
            ),
            &["name"],
        )
        .context("worker_status metric")?;

        let cache_circuit_breaker_state = IntGaugeVec::new(
            prometheus::Opts::new(
                "cache_circuit_breaker_state",
                "Redis cache circuit breaker state (0=closed, 1=open, 2=half-open)",
            ),
            &["state"],
        )
        .context("cache_circuit_breaker_state metric")?;

        registry.register(Box::new(cache_hits.clone()))?;
        registry.register(Box::new(cache_misses.clone()))?;
        registry.register(Box::new(invalidations.clone()))?;
        registry.register(Box::new(request_latency.clone()))?;
        registry.register(Box::new(rpc_errors.clone()))?;
        registry.register(Box::new(rpc_fallbacks.clone()))?;
        registry.register(Box::new(db_query_duration.clone()))?;
        registry.register(Box::new(db_timeouts.clone()))?;
        registry.register(Box::new(db_pool_exhaustion.clone()))?;
        registry.register(Box::new(ledger_gaps.clone()))?;
        registry.register(Box::new(email_dlq_size.clone()))?;
        registry.register(Box::new(email_queue_depth.clone()))?;
        registry.register(Box::new(db_pool_connections_active.clone()))?;
        registry.register(Box::new(db_pool_connections_idle.clone()))?;
        registry.register(Box::new(db_pool_acquire_duration.clone()))?;
        registry.register(Box::new(rate_limit_rejections.clone()))?;
        registry.register(Box::new(deprecated_api_calls.clone()))?;
        registry.register(Box::new(auth_failures.clone()))?;
        registry.register(Box::new(sync_worker_restarts.clone()))?;
        registry.register(Box::new(sync_worker_heartbeat_ts.clone()))?;
        registry.register(Box::new(sendgrid_retries.clone()))?;
        registry.register(Box::new(worker_crash_total.clone()))?;
        registry.register(Box::new(otel_export_errors.clone()))?;
        registry.register(Box::new(worker_status.clone()))?;
        registry.register(Box::new(cache_circuit_breaker_state.clone()))?;

        Ok(Self {
            registry,
            cache_hits,
            cache_misses,
            invalidations,
            request_latency,
            rpc_errors,
            rpc_fallbacks,
            db_query_duration,
            db_timeouts,
            db_pool_exhaustion,
            ledger_gaps,
            email_dlq_size,
            email_queue_depth,
            db_pool_connections_active,
            db_pool_connections_idle,
            db_pool_acquire_duration,
            rate_limit_rejections,
            deprecated_api_calls,
            auth_failures,
            sync_worker_restarts,
            sync_worker_heartbeat_ts,
            sendgrid_retries,
            worker_crash_total,
            otel_export_errors,
            worker_status,
            cache_circuit_breaker_state,
        })
    }

    pub fn observe_hit(&self, layer: &str, endpoint: &str) {
        let labels = normalize_label_values(&[layer, endpoint]);
        self.cache_hits.with_label_values(&[&labels[0], &labels[1]]).inc();
    }

    pub fn observe_miss(&self, layer: &str, endpoint: &str) {
        let labels = normalize_label_values(&[layer, endpoint]);
        self.cache_misses
            .with_label_values(&[&labels[0], &labels[1]])
            .inc();
    }

    pub fn observe_invalidation(&self, scope: &str, count: usize) {
        if count > 0 {
            let labels = normalize_label_values(&[scope]);
            self.invalidations
                .with_label_values(&[&labels[0]])
                .inc_by(count as u64);
        }
    }

    pub fn observe_request(&self, route: &str, status_code: u16, duration: f64) {
        let labels = normalize_label_values(&[route, &status_code.to_string()]);
        self.request_latency
            .with_label_values(&[&labels[0], &labels[1]])
            .observe(duration);
    }

    pub fn observe_rpc_error(&self, method: &str) {
        let labels = normalize_label_values(&[method]);
        self.rpc_errors.with_label_values(&[&labels[0]]).inc();
    }

    pub fn observe_rpc_fallback(&self, endpoint: &str) {
        let labels = normalize_label_values(&[endpoint]);
        self.rpc_fallbacks.with_label_values(&[&labels[0]]).inc();
    }

    pub fn observe_db_query_duration(&self, query_name: &str, duration: Duration) {
        self.db_query_duration
            .with_label_values(&[query_name])
            .observe(duration.as_secs_f64());
    }

    pub fn observe_db_timeout(&self, operation: &str) {
        let labels = normalize_label_values(&[operation]);
        self.db_timeouts.with_label_values(&[&labels[0]]).inc();
    }

    pub fn observe_db_pool_exhaustion(&self, pool: &str) {
        self.db_pool_exhaustion
            .with_label_values(&[pool])
            .inc();
    }

    /// Record a ledger-gap event on `network`, incrementing the counter by `gap_size` ledgers.
    pub fn observe_ledger_gap(&self, network: &str, gap_size: u32) {
        if gap_size > 0 {
            self.ledger_gaps
                .with_label_values(&[network])
                .inc_by(u64::from(gap_size));
        }
    }

    pub fn set_dlq_size(&self, n: i64) {
        self.email_dlq_size.set(n);
    }

    pub fn set_email_queue_depth(&self, n: i64) {
        self.email_queue_depth.set(n);
    }

    pub fn observe_tx_eviction(&self, count: u64) {
        if count > 0 {
            self.invalidations
                .with_label_values(&["tx_watch_eviction"])
                .inc_by(count);
        }
    }

    /// Update connection pool utilisation gauges.
    /// Call this on each pool event (connection acquired, released, opened, closed).
    pub fn observe_pool_connections(&self, pool_label: &str, active: i64, idle: i64) {
        let labels = normalize_label_values(&[pool_label]);
        self.db_pool_connections_active
            .with_label_values(&[&labels[0]])
            .set(active);
        self.db_pool_connections_idle
            .with_label_values(&[&labels[0]])
            .set(idle);
    }

    /// Snapshot the current pool size and idle count into Prometheus gauges.
    /// Call this before rendering `/metrics` so the values are current.
    pub fn record_pool_metrics(&self, max: u32, idle: i64) {
        let pool_label = format!("pool_{}", max);
        self.db_pool_connections_active
            .with_label_values(&[&pool_label])
            .set((max as i64).saturating_sub(idle));
        self.db_pool_connections_idle
            .with_label_values(&[&pool_label])
            .set(idle);
    }

    /// Record how long the caller waited to acquire a connection from the pool.
    pub fn observe_pool_acquire(&self, pool: &str, duration: Duration) {
        let labels = normalize_label_values(&[pool]);
        self.db_pool_acquire_duration
            .with_label_values(&[&labels[0]])
            .observe(duration.as_secs_f64());
    }

    /// Increment the rate-limit rejection counter for a route.
    /// Call this whenever a request is rejected with 429 Too Many Requests.
    pub fn observe_rate_limit_rejection(&self, route: &str) {
        let labels = normalize_label_values(&[route]);
        self.rate_limit_rejections
            .with_label_values(&[&labels[0]])
            .inc();
    }

    pub fn observe_deprecated_api_call(&self, version: &str) {
        self.deprecated_api_calls
            .with_label_values(&[version])
            .inc();
    }

    /// Increment the auth_failures_total counter.
    ///
    /// `reason` should be one of:
    /// - `"invalid_api_key"` — key present but not recognized
    /// - `"expired_token"`   — token present but expired/invalid
    /// - `"missing_credentials"` — no key or token supplied at all
    pub fn observe_auth_failure(&self, reason: &str) {
        self.auth_failures.with_label_values(&[reason]).inc();
    }

    /// #936: Increment the sync worker restart counter.
    pub fn observe_sync_worker_restart(&self) {
        self.sync_worker_restarts.inc();
    }

    /// #936: Record a heartbeat from the sync worker (stores current unix timestamp).
    pub fn observe_sync_worker_heartbeat(&self) {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;
        self.sync_worker_heartbeat_ts.set(ts);
    }

    /// #936: Return the last heartbeat unix timestamp (for /health/ready).
    pub fn sync_worker_last_heartbeat_ts(&self) -> i64 {
        self.sync_worker_heartbeat_ts.get()
    }

    /// #938: Record a ledger sequence gap.
    pub fn observe_ledger_gap(&self, gap: u32) {
        self.ledger_gaps.with_label_values(&["sync"]).inc_by(gap as u64);
    }

    /// Increment the SendGrid retry counter.
    /// `reason` should be "rate_limited" (429) or "server_error" (5xx).
    pub fn observe_sendgrid_retry(&self, reason: &str) {
        self.sendgrid_retries.with_label_values(&[reason]).inc();
    }

    /// Increment the OTEL export error counter.
    /// Pass `reason = "unreachable"` for startup connectivity failures,
    /// `reason = "export_failed"` for runtime export errors.
    pub fn observe_otel_export_error(&self, reason: &str) {
        self.otel_export_errors.with_label_values(&[reason]).inc();
    }

    /// Set worker status to running (1) or stopped (0).
    /// Call this on worker startup (1), during heartbeats (1), and on shutdown (0).
    pub fn set_worker_status(&self, name: &str, running: bool) {
        self.worker_status
            .with_label_values(&[name])
            .set(if running { 1 } else { 0 });
    }

    /// Update the cache circuit breaker state gauge.
    /// Call this whenever the circuit breaker transitions state.
    /// state: 0=closed, 1=open, 2=half-open
    pub fn set_cache_circuit_breaker_state(&self, state: i64) {
        self.cache_circuit_breaker_state.with_label_values(&["closed"]).set(0);
        self.cache_circuit_breaker_state.with_label_values(&["open"]).set(0);
        self.cache_circuit_breaker_state.with_label_values(&["half_open"]).set(0);

        match state {
            0 => { self.cache_circuit_breaker_state.with_label_values(&["closed"]).set(1); }
            1 => { self.cache_circuit_breaker_state.with_label_values(&["open"]).set(1); }
            2 => { self.cache_circuit_breaker_state.with_label_values(&["half_open"]).set(1); }
            _ => {}
        }
    }

    pub fn set_circuit_breaker_state(&self, state: i64) {
        self.set_cache_circuit_breaker_state(state);
    }

    pub fn render(&self) -> anyhow::Result<String> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observe_db_query_duration_records_histogram() {
        let metrics = Metrics::new().unwrap();
        metrics.observe_db_query_duration("test_query", Duration::from_millis(100));
        let output = metrics.render().unwrap();
        assert!(output.contains("db_query_duration_seconds"));
        assert!(output.contains("query_name=\"test_query\""));
    }

    #[test]
    fn observe_db_pool_exhaustion_increments_counter() {
        let metrics = Metrics::new().unwrap();
        metrics.observe_db_pool_exhaustion("api");
        let output = metrics.render().unwrap();
        assert!(output.contains("db_pool_exhaustion_total"));
        assert!(output.contains("pool=\"api\""));
        assert!(output.contains("1"));
    }

    // ── normalize_label ────────────────────────────────────────────────────────

    #[test]
    fn normalize_label_lowercases_and_sanitises() {
        assert_eq!(normalize_label("ApiV1_Statistics"), "api_v1_statistics");
        assert_eq!(normalize_label("featured-markets"), "featured_markets");
    }

    #[test]
    fn normalize_label_strips_leading_and_trailing_underscores() {
        assert_eq!(normalize_label("_hello_world_"), "hello_world");
        assert_eq!(normalize_label("___"), "");
    }

    #[test]
    fn normalize_label_truncates_long_values_and_appends_hotlbl() {
        let long = "a".repeat(60);
        let result = normalize_label(&long);
        assert!(result.ends_with("_hotlbl"), "{}", result);
        assert!(result.len() <= MAX_LABEL_VALUE_LEN, "{}", result);
    }

    #[test]
    fn normalize_label_values_applies_to_all_elements() {
        let vals = normalize_label_values(&["Layer-API", "Endpoint/V1"]);
        assert_eq!(vals[0], "layer_api");
        assert_eq!(vals[1], "endpoint_v1");
    }

    // ── Metrics construction ───────────────────────────────────────────────────

    #[test]
    fn metrics_new_registers_all_collectors() {
        let m = Metrics::new().expect("metrics construction must not fail");
        // Calling observation methods must not panic.
        m.observe_hit("db", "statistics");
        m.observe_miss("api", "featured_markets");
        m.observe_invalidation("market_resolve", 5);
        m.observe_request("statistics", 200, 0.05);
        m.observe_rpc_error("getContractData");
        m.observe_rpc_fallback("market_data");
        m.observe_db_timeout("statistics");
        m.record_pool_metrics(10, 4);
        m.observe_pool_acquire("pool_10", Duration::from_millis(2));
        m.observe_rate_limit_rejection("ratelimit");
        m.observe_tx_eviction(3);
        m.set_dlq_size(7);
        m.set_email_queue_depth(12);
        m.observe_auth_failure("invalid_api_key");
        m.set_circuit_breaker_state(0);
        m.set_worker_status("test_worker", true);
        let rendered = m.render().expect("render must not fail");
        assert!(rendered.contains("cache_hits_total"));
        assert!(rendered.contains("http_request_duration_seconds"));
    }

    // ── record_pool_metrics ────────────────────────────────────────────────────

    #[test]
    fn record_pool_metrics_sets_active_and_idle_gauges() {
        let m = Metrics::new().unwrap();
        m.record_pool_metrics(10, 3);
        let rendered = m.render().unwrap();
        assert!(rendered.contains("db_pool_connections_active{pool=\"pool_10\"} 7"));
        assert!(rendered.contains("db_pool_connections_idle{pool=\"pool_10\"} 3"));
    }

    #[test]
    fn record_pool_metrics_with_zero_active() {
        let m = Metrics::new().unwrap();
        m.record_pool_metrics(20, 0);
        let rendered = m.render().unwrap();
        assert!(rendered.contains("db_pool_connections_active{pool=\"pool_20\"} 20"));
        assert!(rendered.contains("db_pool_connections_idle{pool=\"pool_20\"} 0"));
    }

    // ── Cardinality guard: observe_request normalises labels ───────────────────

    #[test]
    fn observe_request_normalises_route_label() {
        let m = Metrics::new().unwrap();
        m.observe_request("StatistIcs", 200, 0.1);
        let rendered = m.render().unwrap();
        assert!(rendered.contains("route=\"statistics\""));
    }

    // ── Long label values trigger _hotlbl suffix ───────────────────────────────

    #[test]
    fn long_label_is_truncated_with_hotlbl() {
        let long_route = "x".repeat(60);
        let m = Metrics::new().unwrap();
        m.observe_request(&long_route, 500, 0.01);
        let rendered = m.render().unwrap();
        assert!(rendered.contains("_hotlbl"));
    }

    // ── observe_hit / observe_miss normalise both labels ──────────────────────

    #[test]
    fn cache_metrics_normalise_layer_and_endpoint() {
        let m = Metrics::new().unwrap();
        m.observe_hit("API", "featured-markets");
        m.observe_miss("CHAIN", "oracle_result");
        let rendered = m.render().unwrap();
        assert!(rendered.contains("layer=\"api\""));
        assert!(rendered.contains("endpoint=\"featured_markets\""));
        assert!(rendered.contains("layer=\"chain\""));
        assert!(rendered.contains("endpoint=\"oracle_result\""));
    }
}
