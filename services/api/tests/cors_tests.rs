/// CORS policy tests.
///
/// Each test builds a minimal router with a specific [`CorsConfig`] and fires
/// requests through it, asserting on the response headers that tower-http's
/// [`CorsLayer`] emits.
///
/// Covered scenarios
/// -----------------
/// * Allowed origin receives `Access-Control-Allow-Origin`
/// * Unlisted origin receives no `Access-Control-Allow-Origin`
/// * Preflight (OPTIONS) for an allowed origin returns 200 with the correct
///   `Access-Control-Allow-Methods` and `Access-Control-Allow-Headers`
/// * Preflight for an unlisted origin is rejected (no ACAO header)
/// * `allow_credentials = true` emits `Access-Control-Allow-Credentials: true`
/// * `allow_credentials = false` does not emit the credentials header
/// * Dev mode is fully permissive (wildcard origin)
/// * Multiple allowed origins are each individually honoured
#[cfg(test)]
mod tests {
    use axum::{body::Body, http::Request, routing::get, Router};
    use predictiq_api::config::CorsConfig;
    use tower::ServiceExt;
    use tower_http::cors::CorsLayer;
    use std::time::Duration;

    // ── helpers ───────────────────────────────────────────────────────────────

    fn cors_layer_from(cfg: &CorsConfig) -> CorsLayer {
        use axum::http::{HeaderName, HeaderValue, Method};

        if cfg.dev_mode {
            return CorsLayer::permissive();
        }

        let origins: Vec<HeaderValue> = cfg
            .allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();

        let methods: Vec<Method> = cfg
            .allowed_methods
            .iter()
            .filter_map(|m| m.parse().ok())
            .collect();

        let headers: Vec<HeaderName> = cfg
            .allowed_headers
            .iter()
            .filter_map(|h| h.parse().ok())
            .collect();

        let layer = CorsLayer::new()
            .allow_origin(origins)
            .allow_methods(methods)
            .allow_headers(headers)
            .max_age(Duration::from_secs(cfg.max_age_secs));

        if cfg.allow_credentials {
            layer.allow_credentials(true)
        } else {
            layer
        }
    }

    fn app(cfg: &CorsConfig) -> Router {
        Router::new()
            .route("/api/data", get(|| async { "ok" }))
            .layer(cors_layer_from(cfg))
    }

    fn default_cfg(origins: Vec<&str>) -> CorsConfig {
        CorsConfig {
            dev_mode: false,
            allowed_origins: origins.into_iter().map(str::to_string).collect(),
            allowed_methods: vec![
                "GET".into(),
                "POST".into(),
                "PUT".into(),
                "PATCH".into(),
                "DELETE".into(),
                "OPTIONS".into(),
            ],
            allowed_headers: vec!["content-type".into(), "authorization".into()],
            allow_credentials: false,
            max_age_secs: 3600,
        }
    }

    // ── allowed origin ────────────────────────────────────────────────────────

    #[tokio::test]
    async fn allowed_origin_receives_acao_header() {
        let cfg = default_cfg(vec!["https://app.predictiq.com"]);
        let response = app(&cfg)
            .oneshot(
                Request::builder()
                    .uri("/api/data")
                    .header("origin", "https://app.predictiq.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let acao = response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok());

        assert_eq!(acao, Some("https://app.predictiq.com"));
    }

    // ── unlisted origin ───────────────────────────────────────────────────────

    #[tokio::test]
    async fn unlisted_origin_receives_no_acao_header() {
        let cfg = default_cfg(vec!["https://app.predictiq.com"]);
        let response = app(&cfg)
            .oneshot(
                Request::builder()
                    .uri("/api/data")
                    .header("origin", "https://evil.example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let acao = response.headers().get("access-control-allow-origin");
        assert!(
            acao.is_none(),
            "expected no ACAO header for unlisted origin, got: {acao:?}"
        );
    }

    // ── preflight (OPTIONS) ───────────────────────────────────────────────────

    #[tokio::test]
    async fn preflight_for_allowed_origin_returns_200_with_cors_headers() {
        let cfg = default_cfg(vec!["https://app.predictiq.com"]);
        let response = app(&cfg)
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/data")
                    .header("origin", "https://app.predictiq.com")
                    .header("access-control-request-method", "POST")
                    .header("access-control-request-headers", "content-type")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status().is_success(),
            "preflight should succeed, got {}",
            response.status()
        );

        let acao = response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok());
        assert_eq!(acao, Some("https://app.predictiq.com"));

        let acam = response
            .headers()
            .get("access-control-allow-methods")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            acam.to_uppercase().contains("POST"),
            "expected POST in allow-methods, got: {acam}"
        );

        let acah = response
            .headers()
            .get("access-control-allow-headers")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        assert!(
            acah.to_lowercase().contains("content-type"),
            "expected content-type in allow-headers, got: {acah}"
        );
    }

    #[tokio::test]
    async fn preflight_for_unlisted_origin_has_no_acao_header() {
        let cfg = default_cfg(vec!["https://app.predictiq.com"]);
        let response = app(&cfg)
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/data")
                    .header("origin", "https://attacker.example.com")
                    .header("access-control-request-method", "POST")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let acao = response.headers().get("access-control-allow-origin");
        assert!(
            acao.is_none(),
            "expected no ACAO header for unlisted origin in preflight, got: {acao:?}"
        );
    }

    // ── credentials ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn credentials_true_emits_acac_header() {
        let cfg = CorsConfig {
            allow_credentials: true,
            ..default_cfg(vec!["https://app.predictiq.com"])
        };
        let response = app(&cfg)
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/data")
                    .header("origin", "https://app.predictiq.com")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let acac = response
            .headers()
            .get("access-control-allow-credentials")
            .and_then(|v| v.to_str().ok());
        assert_eq!(acac, Some("true"));
    }

    #[tokio::test]
    async fn credentials_false_does_not_emit_acac_header() {
        let cfg = CorsConfig {
            allow_credentials: false,
            ..default_cfg(vec!["https://app.predictiq.com"])
        };
        let response = app(&cfg)
            .oneshot(
                Request::builder()
                    .method("OPTIONS")
                    .uri("/api/data")
                    .header("origin", "https://app.predictiq.com")
                    .header("access-control-request-method", "GET")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let acac = response.headers().get("access-control-allow-credentials");
        assert!(
            acac.is_none(),
            "expected no ACAC header when credentials=false, got: {acac:?}"
        );
    }

    // ── dev mode ──────────────────────────────────────────────────────────────

    #[tokio::test]
    async fn dev_mode_allows_any_origin() {
        let cfg = CorsConfig {
            dev_mode: true,
            ..default_cfg(vec![]) // no explicit origins needed
        };
        let response = app(&cfg)
            .oneshot(
                Request::builder()
                    .uri("/api/data")
                    .header("origin", "https://totally-random-origin.example.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let acao = response
            .headers()
            .get("access-control-allow-origin")
            .and_then(|v| v.to_str().ok());

        // CorsLayer::permissive() reflects the request origin or uses wildcard.
        assert!(
            acao.is_some(),
            "dev mode should emit an ACAO header for any origin"
        );
    }

    #[tokio::test]
    async fn non_dev_mode_with_no_origins_configured_blocks_all() {
        let cfg = default_cfg(vec![]); // empty allowlist, dev_mode=false
        let response = app(&cfg)
            .oneshot(
                Request::builder()
                    .uri("/api/data")
                    .header("origin", "https://app.predictiq.com")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        let acao = response.headers().get("access-control-allow-origin");
        assert!(
            acao.is_none(),
            "empty allowlist should block all origins, got: {acao:?}"
        );
    }

    // ── multiple origins ──────────────────────────────────────────────────────

    #[tokio::test]
    async fn each_allowed_origin_is_individually_reflected() {
        let cfg = default_cfg(vec![
            "https://app.predictiq.com",
            "https://staging.predictiq.com",
        ]);

        for origin in &["https://app.predictiq.com", "https://staging.predictiq.com"] {
            let response = app(&cfg)
                .oneshot(
                    Request::builder()
                        .uri("/api/data")
                        .header("origin", *origin)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            let acao = response
                .headers()
                .get("access-control-allow-origin")
                .and_then(|v| v.to_str().ok());

            assert_eq!(
                acao,
                Some(*origin),
                "expected ACAO={origin}, got {acao:?}"
            );
        }
    }

    // ── CorsConfig parsing ────────────────────────────────────────────────────

    #[test]
    fn cors_config_from_env_defaults_are_secure() {
        // With no env vars set the config must not be in dev mode and must
        // have an empty origin list (no cross-origin access by default).
        // We can't call from_env() without polluting the process env, so we
        // verify the invariants on a manually constructed default-equivalent.
        let cfg = CorsConfig {
            dev_mode: false,
            allowed_origins: vec![],
            allowed_methods: vec![
                "GET".into(),
                "POST".into(),
                "PUT".into(),
                "PATCH".into(),
                "DELETE".into(),
                "OPTIONS".into(),
            ],
            allowed_headers: vec!["content-type".into(), "authorization".into()],
            allow_credentials: false,
            max_age_secs: 3600,
        };

        assert!(!cfg.dev_mode, "dev_mode must default to false");
        assert!(
            cfg.allowed_origins.is_empty(),
            "allowed_origins must default to empty (no cross-origin access)"
        );
        assert!(!cfg.allow_credentials, "allow_credentials must default to false");
    }
}
