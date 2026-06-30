use std::time::Duration;

use anyhow::Context;
use prometheus::{Encoder, HistogramVec, IntCounterVec, IntGauge, IntGaugeVec, Registry, TextEncoder};

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
    email_dlq_size: IntGauge,
    db_pool_connections_active: IntGaugeVec,
    db_pool_connections_idle: IntGaugeVec,
    db_pool_acquire_duration: HistogramVec,
    rate_limit_rejections: IntCounterVec,
    deprecated_api_calls: IntCounterVec,
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

        let email_dlq_size = IntGauge::new(
            "email_dlq_size",
            "Number of email jobs currently in the dead-letter queue",
        )
        .context("email_dlq_size metric")?;

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

        registry.register(Box::new(cache_hits.clone()))?;
        registry.register(Box::new(cache_misses.clone()))?;
        registry.register(Box::new(invalidations.clone()))?;
        registry.register(Box::new(request_latency.clone()))?;
        registry.register(Box::new(rpc_errors.clone()))?;
        registry.register(Box::new(rpc_fallbacks.clone()))?;
        registry.register(Box::new(db_timeouts.clone()))?;
        registry.register(Box::new(email_dlq_size.clone()))?;
        registry.register(Box::new(db_pool_connections_active.clone()))?;
        registry.register(Box::new(db_pool_connections_idle.clone()))?;
        registry.register(Box::new(db_pool_acquire_duration.clone()))?;
        registry.register(Box::new(rate_limit_rejections.clone()))?;
        registry.register(Box::new(deprecated_api_calls.clone()))?;

        Ok(Self {
            registry,
            cache_hits,
            cache_misses,
            invalidations,
            request_latency,
            rpc_errors,
            rpc_fallbacks,
            db_timeouts,
            email_dlq_size,
            db_pool_connections_active,
            db_pool_connections_idle,
            db_pool_acquire_duration,
            rate_limit_rejections,
            deprecated_api_calls,
        })
    }

    pub fn observe_hit(&self, layer: &str, endpoint: &str) {
        self.cache_hits.with_label_values(&[layer, endpoint]).inc();
    }

    pub fn observe_miss(&self, layer: &str, endpoint: &str) {
        self.cache_misses
            .with_label_values(&[layer, endpoint])
            .inc();
    }

    pub fn observe_invalidation(&self, scope: &str, count: usize) {
        if count > 0 {
            self.invalidations
                .with_label_values(&[scope])
                .inc_by(count as u64);
        }
    }


    pub fn observe_request(&self, route: &str, status_code: &str, duration: Duration) {
        self.request_latency
            .with_label_values(&[route, status_code])
            .observe(duration.as_secs_f64());
    }

    pub fn observe_rpc_error(&self, method: &str) {
        self.rpc_errors.with_label_values(&[method]).inc();
    }

    pub fn observe_rpc_fallback(&self, endpoint: &str) {
        self.rpc_fallbacks.with_label_values(&[endpoint]).inc();
    }

    pub fn observe_db_timeout(&self, operation: &str) {
        self.db_timeouts.with_label_values(&[operation]).inc();
    }

    pub fn set_dlq_size(&self, n: i64) {
        self.email_dlq_size.set(n);
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
    pub fn observe_pool_connections(&self, pool: &str, active: i64, idle: i64) {
        self.db_pool_connections_active
            .with_label_values(&[pool])
            .set(active);
        self.db_pool_connections_idle
            .with_label_values(&[pool])
            .set(idle);
    }

    /// Record how long the caller waited to acquire a connection from the pool.
    pub fn observe_pool_acquire(&self, pool: &str, duration: Duration) {
        self.db_pool_acquire_duration
            .with_label_values(&[pool])
            .observe(duration.as_secs_f64());
    }

    /// Increment the rate-limit rejection counter for a route.
    /// Call this whenever a request is rejected with 429 Too Many Requests.
    pub fn observe_rate_limit_rejection(&self, route: &str) {
        self.rate_limit_rejections
            .with_label_values(&[route])
            .inc();
    }

    pub fn observe_deprecated_api_call(&self, version: &str) {
        self.deprecated_api_calls
            .with_label_values(&[version])
            .inc();
    }

    pub fn render(&self) -> anyhow::Result<String> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
