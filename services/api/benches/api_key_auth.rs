use criterion::{black_box, criterion_group, criterion_main, Criterion};
use predictiq_api::security::ApiKeyAuth;

fn bench_api_key_verify_constant_time(c: &mut Criterion) {
    let keys = vec![
        "aaaaaaaaaaaaaaaa".to_string(),
        "baaaaaaaaaaaaaaaa".to_string(),
        "caaaaaaaaaaaaaaaa".to_string(),
        "daaaaaaaaaaaaaaaa".to_string(),
        "target-key-123456".to_string(),
    ];
    let auth = ApiKeyAuth::new(keys);

    let mut group = c.benchmark_group("api_key_verify_constant_time");

    // Test early mismatch (first character different)
    group.bench_function("early_mismatch", |b| {
        b.iter(|| auth.verify(black_box("xaaaaaaaaaaaaaaaa")))
    });

    // Test late mismatch (last character different)
    group.bench_function("late_mismatch", |b| {
        b.iter(|| auth.verify(black_box("aaaaaaaaaaaaaaab")))
    });

    // Test exact match
    group.bench_function("exact_match", |b| {
        b.iter(|| auth.verify(black_box("target-key-123456")))
    });

    // Test wrong length (shorter)
    group.bench_function("wrong_length_short", |b| {
        b.iter(|| auth.verify(black_box("short")))
    });

    // Test wrong length (longer)
    group.bench_function("wrong_length_long", |b| {
        b.iter(|| auth.verify(black_box("this-is-a-very-long-key-that-does-not-match")))
    });

    // Test no keys scenario
    let empty_auth = ApiKeyAuth::new(vec![]);
    group.bench_function("no_keys", |b| {
        b.iter(|| empty_auth.verify(black_box("any-key")))
    });

    group.finish();
}

fn bench_api_key_verify_scalability(c: &mut Criterion) {
    let mut group = c.benchmark_group("api_key_verify_scalability");

    // Test with different numbers of keys to ensure performance doesn't degrade
    for &key_count in &[1, 10, 100, 1000] {
        let keys: Vec<String> = (0..key_count)
            .map(|i| format!("key-{:016}", i))
            .collect();
        let auth = ApiKeyAuth::new(keys);

        group.bench_with_input(
            format!("keys_{}", key_count),
            &key_count,
            |b, _| {
                b.iter(|| auth.verify(black_box("non-existent-key")))
            },
        );
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_api_key_verify_constant_time,
    bench_api_key_verify_scalability
);
criterion_main!(benches);
