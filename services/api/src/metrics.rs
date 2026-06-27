use std::time::Duration;

use anyhow::Context;
use prometheus::{Encoder, HistogramVec, IntCounterVec, Registry, TextEncoder};

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

        registry.register(Box::new(cache_hits.clone()))?;
        registry.register(Box::new(cache_misses.clone()))?;
        registry.register(Box::new(invalidations.clone()))?;
        registry.register(Box::new(request_latency.clone()))?;
        registry.register(Box::new(rpc_errors.clone()))?;
        registry.register(Box::new(rpc_fallbacks.clone()))?;
        registry.register(Box::new(db_timeouts.clone()))?;
        registry.register(Box::new(ledger_gaps.clone()))?;

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

    /// Record a ledger-gap event on `network`, incrementing the counter by `gap_size` ledgers.
    pub fn observe_ledger_gap(&self, network: &str, gap_size: u32) {
        if gap_size > 0 {
            self.ledger_gaps
                .with_label_values(&[network])
                .inc_by(u64::from(gap_size));
        }
    }

    pub fn observe_tx_eviction(&self, count: u64) {
        if count > 0 {
            self.invalidations
                .with_label_values(&["tx_watch_eviction"])
                .inc_by(count);
        }
    }

    pub fn render(&self) -> anyhow::Result<String> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
