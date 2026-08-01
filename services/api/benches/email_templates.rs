/// Benchmark: cached vs per-request template compilation.
///
/// EmailTemplateEngine compiles all templates once at construction time
/// (EmailTemplateEngine::new()). This benchmark measures the render cost at
/// 1 000 iterations to confirm that per-request re-parsing is not occurring.
///
/// Template-update policy: templates are embedded via include_str! and compiled
/// at startup. To pick up a template change, restart the service. Hot-reload is
/// not supported in production because (a) it would require dynamic file I/O
/// and a filesystem watcher, and (b) template changes require review and
/// deployment anyway.
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use predictiq_api::email::templates::EmailTemplateEngine;

fn bench_cached_render(c: &mut Criterion) {
    // Build once — this is the production path (cached registry).
    let engine = EmailTemplateEngine::new().expect("engine init");
    let data = serde_json::json!({
        "confirm_url": "https://example.com/confirm?token=bench",
        "email": "bench@example.com"
    });

    let mut group = c.benchmark_group("email_template_render");

    group.bench_function("cached_1000_renders", |b| {
        b.iter(|| {
            for _ in 0..1_000 {
                let _ = engine.render(
                    black_box("newsletter_confirmation"),
                    black_box(&data),
                );
            }
        })
    });

    // Baseline: cost of constructing a fresh engine per render (per-request).
    // This is the anti-pattern the issue was filed against; the number should
    // be dramatically higher than the cached path above.
    group.bench_function("per_request_engine_1000_renders", |b| {
        b.iter(|| {
            for _ in 0..1_000 {
                let fresh = EmailTemplateEngine::new().expect("engine init");
                let _ = fresh.render(
                    black_box("newsletter_confirmation"),
                    black_box(&data),
                );
            }
        })
    });

    group.finish();
}

fn bench_all_templates_cached(c: &mut Criterion) {
    let engine = EmailTemplateEngine::new().expect("engine init");

    let fixtures: &[(&str, serde_json::Value)] = &[
        ("newsletter_confirmation", serde_json::json!({
            "confirm_url": "https://example.com/confirm?token=bench",
            "email": "bench@example.com"
        })),
        ("waitlist_confirmation", serde_json::json!({
            "email": "bench@example.com"
        })),
        ("contact_form_auto_response", serde_json::json!({
            "name": "Bench User",
            "subject": "Bench Subject",
            "message": "Bench message."
        })),
        ("welcome_email", serde_json::json!({
            "name": "Bench User",
            "dashboard_url": "https://example.com/dashboard",
            "help_url": "https://example.com/help",
            "unsubscribe_url": "https://example.com/unsubscribe"
        })),
    ];

    let mut group = c.benchmark_group("email_template_all_cached");
    for (name, data) in fixtures {
        group.bench_with_input(BenchmarkId::new("render", name), data, |b, d| {
            b.iter(|| engine.render(black_box(name), black_box(d)))
        });
    }
    group.finish();
}

criterion_group!(benches, bench_cached_render, bench_all_templates_cached);
criterion_main!(benches);
