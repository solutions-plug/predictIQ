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
    db_timeouts: IntCounterVec,
    ledger_gaps: IntCounterVec,
    email_dlq_size: IntGauge,
    email_queue_depth: IntGauge,
    db_pool_connections_active: IntGaugeVec,
    db_pool_connections_idle: IntGaugeVec,
    db_pool_acquire_duration: HistogramVec,
    rate_limit_rejections: IntCounterVec,
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

        let db_timeouts = IntCounterVec::new(
            prometheus::Opts::new("db_timeouts_total", "DB queries that exceeded the timeout, by operation"),
            &["operation"],
        )
        .context("db_timeouts metric")?;

        let ledger_gaps = IntCounterVec::new(
            prometheus::Opts::new(
                "blockchain_ledger_gaps_total",
                "Ledger gap events detected during blockchain sync, labelled by network",
            ),
            &["network"],
        )
        .context("ledger_gaps metric")?;

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
        registry.register(Box::new(db_timeouts.clone()))?;
        registry.register(Box::new(ledger_gaps.clone()))?;
        registry.register(Box::new(email_dlq_size.clone()))?;
        registry.register(Box::new(email_queue_depth.clone()))?;
        registry.register(Box::new(db_pool_connections_active.clone()))?;
        registry.register(Box::new(db_pool_connections_idle.clone()))?;
        registry.register(Box::new(db_pool_acquire_duration.clone()))?;
        registry.register(Box::new(rate_limit_rejections.clone()))?;
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
            db_timeouts,
            ledger_gaps,
            email_dlq_size,
            email_queue_depth,
            db_pool_connections_active,
            db_pool_connections_idle,
            db_pool_acquire_duration,
            rate_limit_rejections,
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

    pub fn observe_db_timeout(&self, operation: &str) {
        let labels = normalize_label_values(&[operation]);
        self.db_timeouts.with_label_values(&[&labels[0]]).inc();
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
        // Reset all states to 0 first
        self.cache_circuit_breaker_state
            .with_label_values(&["closed"])
            .set(0);
        self.cache_circuit_breaker_state
            .with_label_values(&["open"])
            .set(0);
        self.cache_circuit_breaker_state
            .with_label_values(&["half_open"])
            .set(0);

        // Set the current state to 1
        match state {
            0 => {
                self.cache_circuit_breaker_state
                    .with_label_values(&["closed"])
                    .set(1);
            }
            1 => {
                self.cache_circuit_breaker_state
                    .with_label_values(&["open"])
                    .set(1);
            }
            2 => {
                self.cache_circuit_breaker_state
                    .with_label_values(&["half_open"])
                    .set(1);
            }
            _ => {}
        }
    }

    /// Convenience alias that maps a numeric state to the labelled gauge vec.
    /// Delegates to set_cache_circuit_breaker_state; kept for backward compatibility.
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
