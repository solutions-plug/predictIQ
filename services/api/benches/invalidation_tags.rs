/// Benchmark: tag-based invalidation key derivation overhead.
///
/// Measures the cost of `InvalidationTag::cache_keys()` (pure CPU, no I/O)
/// to confirm that the tag abstraction adds negligible overhead compared to
/// building the key list manually inline.
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use predictiq_api::cache::InvalidationTag;

fn bench_tag_cache_keys(c: &mut Criterion) {
    let mut group = c.benchmark_group("invalidation_tag");

    // Tag-based: derive keys from a MarketResolved tag.
    group.bench_function("market_resolved_tag_keys", |b| {
        let tag = InvalidationTag::MarketResolved {
            market_id: black_box(42),
            network: black_box("testnet".to_string()),
            featured_limit: black_box(10),
        };
        b.iter(|| black_box(tag.cache_keys()))
    });

    // Baseline: build the same key list manually (what the handler did before).
    group.bench_function("market_resolved_manual_keys", |b| {
        b.iter(|| {
            let market_id: i64 = black_box(42);
            let network: &str = black_box("testnet");
            let featured_limit: i64 = black_box(10);
            black_box(vec![
                format!("chain:v1:market:{market_id}"),
                format!("chain:v1:oracle:{network}:market:{market_id}"),
                "api:v1:statistics".to_string(),
                "api:v1:featured_markets".to_string(),
                "dbq:v1:statistics".to_string(),
                format!("dbq:v1:featured_markets:limit:{featured_limit}"),
            ])
        })
    });

    group.finish();
}

criterion_group!(benches, bench_tag_cache_keys);
criterion_main!(benches);
