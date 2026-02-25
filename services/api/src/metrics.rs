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
                "HTTP latency in seconds",
            ),
            &["endpoint"],
        )
        .context("request_latency metric")?;

        registry.register(Box::new(cache_hits.clone()))?;
        registry.register(Box::new(cache_misses.clone()))?;
        registry.register(Box::new(invalidations.clone()))?;
        registry.register(Box::new(request_latency.clone()))?;

        Ok(Self {
            registry,
            cache_hits,
            cache_misses,
            invalidations,
            request_latency,
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

    pub fn observe_request(&self, endpoint: &str, duration: Duration) {
        self.request_latency
            .with_label_values(&[endpoint])
            .observe(duration.as_secs_f64());
    }

    pub fn render(&self) -> anyhow::Result<String> {
        let mut buffer = vec![];
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        encoder.encode(&metric_families, &mut buffer)?;
        Ok(String::from_utf8(buffer)?)
    }
}
