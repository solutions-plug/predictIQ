use predictiq_api::{
    audit::AuditLogger,
    blockchain::BlockchainClient,
    cache::RedisCache,
    config::{Config, CorsConfig},
    db::Database,
    email::{queue::EmailQueue, service::EmailService, webhook::WebhookHandler},
    handlers,
    idempotency, correlation, versioning, validation, rate_limit, audit_middleware,
    metrics::Metrics,
    newsletter::IpRateLimiter,
    security::{self, ApiKeyAuth, IpWhitelist, MetricsAuthConfig, RateLimiter},
    shutdown::{self as shutdown, wait_for_signal, ShutdownCoordinator},
    tracing_config, compression,
    AppState,
};

use std::{sync::Arc, time::Duration};

use axum::{
    http::{HeaderName, HeaderValue, Method},
    middleware,
    routing::{get, post},
    Router,
};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

/// Read `SHUTDOWN_TIMEOUT_SECS` from the environment; default 30 s.
fn shutdown_timeout() -> Duration {
    let secs = std::env::var("SHUTDOWN_TIMEOUT_SECS")
        .ok()
        .and_then(|v| v.parse::<u64>().ok())
        .unwrap_or(30);
    Duration::from_secs(secs)
}

/// Build a [`CorsLayer`] from the application's [`CorsConfig`].
///
/// When `dev_mode` is `true` the layer is fully permissive and a warning is
/// emitted so the setting is never silent.  In all other cases only the
/// explicitly configured origins, methods, and headers are allowed.
fn build_cors_layer(cfg: &CorsConfig) -> CorsLayer {
    if cfg.dev_mode {
        tracing::warn!(
            "CORS_DEV_MODE is enabled — all origins are permitted. \
             This MUST NOT be used in production."
        );
        return CorsLayer::permissive();
    }

    let origins: Vec<HeaderValue> = cfg
        .allowed_origins
        .iter()
        .filter_map(|o| o.parse::<HeaderValue>().ok())
        .collect();

    let methods: Vec<Method> = cfg
        .allowed_methods
        .iter()
        .filter_map(|m| m.parse::<Method>().ok())
        .collect();

    let headers: Vec<HeaderName> = cfg
        .allowed_headers
        .iter()
        .filter_map(|h| h.parse::<HeaderName>().ok())
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

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = Config::from_env();

    tracing_config::init_tracing(
        "predictiq-api",
        env!("CARGO_PKG_VERSION"),
        config.otlp_endpoint.clone(),
        config.trace_sample_rate,
    )?;

    // Validate required configuration before proceeding
    config.validate()?;

    let metrics = Metrics::new()?;
    let cache = RedisCache::new(&config.redis_url).await?;
    let db = Database::new(&config.database_url, cache.clone(), metrics.clone(), &config.db_pool).await?;
    let blockchain = BlockchainClient::new(&config, cache.clone(), metrics.clone())?;
    blockchain.validate_network_passphrase().await?;

    let email_service = EmailService::new(config.clone())?;
    let email_queue = EmailQueue::new(cache.clone(), db.clone());
    let webhook_handler = WebhookHandler::new(db.clone());
    let audit_logger = AuditLogger::new(db.pool());

    let bind_addr = config.bind_addr;

    let rate_limiter = Arc::new(RateLimiter::new());
    let api_key_auth = Arc::new(ApiKeyAuth::new(config.api_keys.clone()));
    let ip_whitelist = Arc::new(IpWhitelist::new(config.admin_whitelist_ips.clone()));
    let config_trust_proxy = config.trust_proxy;

    // ── Shutdown coordinators ─────────────────────────────────────────────────
    // Email queue gets its own coordinator so it can be drained with a dedicated
    // timeout (EMAIL_QUEUE_DRAIN_TIMEOUT_SECS) that is independent of the global
    // worker shutdown timeout.  Losing in-flight emails is more costly than
    // delaying exit, so the email drain timeout defaults to 60 s.
    //
    // Blockchain workers (sync + tx-monitor) use the global coordinator.
    // The rate-limiter cleanup and newsletter cleanup tasks are fire-and-forget
    // (low-risk, no persistent state) so they are not tracked.
    let email_coordinator = ShutdownCoordinator::new(1);
    let coordinator = ShutdownCoordinator::new(2);

    // ── Rate-limiter cleanup (fire-and-forget) ────────────────────────────────
    let rate_limiter_cleanup = rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            rate_limiter_cleanup.cleanup().await;
        }
    });

    let state = Arc::new(AppState {
        config,
        cache: cache.clone(),
        db,
        blockchain,
        metrics,
        newsletter_rate_limiter: IpRateLimiter::new(cache.clone()),
        email_service: email_service.clone(),
        email_queue: email_queue.clone(),
        webhook_handler: webhook_handler.clone(),
        audit_logger,
    });

    // ── Blockchain background workers ─────────────────────────────────────────
    let _blockchain_handles = Arc::new(state.blockchain.clone())
        .start_background_tasks(&coordinator);

    // ── Newsletter cleanup (fire-and-forget) ──────────────────────────────────
    let db_cleanup = state.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            let ttl = db_cleanup.config.newsletter_token_ttl_secs;
            let batch = db_cleanup.config.newsletter_cleanup_batch_size;
            match db_cleanup.db.newsletter_delete_expired_pending(ttl, batch).await {
                Ok(n) if n > 0 => tracing::info!("[newsletter] cleaned up {n} expired pending subscriptions"),
                Err(e) => tracing::warn!("[newsletter] cleanup error: {e}"),
                _ => {}
            }
        }
    });

    // ── Email queue worker ────────────────────────────────────────────────────
    let queue_worker = email_queue.clone();
    let service_worker = email_service.clone();
    let email_token = email_coordinator.token();
    let email_coord = email_coordinator.clone();
    let stale_threshold = state.config.email_stale_job_threshold_secs;
    tokio::spawn(async move {
        queue_worker
            .start_worker(service_worker, email_token, email_coord, stale_threshold)
            .await;
    });

    if let Err(err) = handlers::warm_critical_caches(state.clone()).await {
        tracing::warn!("cache warming skipped: {err}");
    }

    // ── CORS ──────────────────────────────────────────────────────────────────
    let cors_layer = build_cors_layer(&state.config.cors);

    // ── Routes ────────────────────────────────────────────────────────────────
    let public_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/api/v1/blockchain/health", get(handlers::blockchain_health))
        .route("/api/v1/blockchain/markets/:market_id", get(handlers::blockchain_market_data))
        .route("/api/v1/blockchain/stats", get(handlers::blockchain_platform_stats))
        .route("/api/v1/blockchain/users/:user/bets", get(handlers::blockchain_user_bets))
        .route("/api/v1/blockchain/oracle/:market_id", get(handlers::blockchain_oracle_result))
        .route("/api/v1/blockchain/tx/:tx_hash", get(handlers::blockchain_tx_status))
        .route("/api/v1/statistics", get(handlers::statistics))
        .route("/api/v1/markets/featured", get(handlers::featured_markets))
        .route("/api/v1/content", get(handlers::content))
        .layer(middleware::from_fn(correlation::correlation_id_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(versioning::versioning_middleware))
        .layer(middleware::from_fn_with_state(
            (rate_limiter.clone(), security::TrustProxy(config_trust_proxy)),
            security::global_rate_limit_middleware,
        ))
        .with_state(state.clone());

    let metrics_auth_config = Arc::new(MetricsAuthConfig::new(
        state.config.metrics_public,
        state.config.metrics_allowlist_ips.clone(),
        api_key_auth.clone(),
    ));
    let metrics_routes = Router::new()
        .route("/metrics", get(handlers::metrics))
        .layer(middleware::from_fn_with_state(
            metrics_auth_config,
            security::metrics_auth_middleware,
        ))
        .layer(middleware::from_fn(correlation::correlation_id_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let newsletter_routes = Router::new()
        .route("/api/v1/newsletter/subscribe", post(handlers::newsletter_subscribe))
        .route("/api/v1/newsletter/confirm", get(handlers::newsletter_confirm))
        .route("/api/v1/newsletter/unsubscribe", get(handlers::newsletter_unsubscribe))
        .route("/api/v1/newsletter/gdpr/export", get(handlers::newsletter_gdpr_export))
        .route("/api/v1/newsletter/gdpr/delete", axum::routing::delete(handlers::newsletter_gdpr_delete))
        .layer(middleware::from_fn(correlation::correlation_id_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(state.clone(), idempotency::idempotency_middleware))
        .layer(middleware::from_fn(validation::content_type_validation_middleware))
        .layer(middleware::from_fn(validation::request_size_validation_middleware))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit::newsletter_rate_limit_middleware,
        ))
        .with_state(state.clone());

    // ── Webhook routes (provider-signed, no admin auth required) ──────────────┐
    // Provider webhooks like SendGrid are authenticated via cryptographic       │
    // signatures in request headers, NOT via API keys. This is the correct      │
    // security model: the webhook endpoint trusts the provider to sign requests,│
    // and verifies the signature matches known credentials.                     │
    //                                                                           │
    // Middleware stack (order matters — applied inside-out):                    │
    // 1. sendgrid_webhook_middleware: verify provider signature                │
    // 2. request_size_validation_middleware: prevent payload bombs              │
    // 3. security_headers_middleware: add security headers                      │
    // 4. correlation_id_middleware: request tracing                             │
    // 5. TraceLayer: OpenTelemetry tracing                                      │
    //                                                                           │
    // Notable omissions (admin auth NOT required):                              │
    // - api_key_middleware: webhooks are provider-signed                        │
    // - ip_whitelist_middleware: webhooks come from SendGrid IPs               │
    // - idempotency_middleware: webhook events are idempotent by nature         │
    // - audit_logging_middleware: webhook events are tracked via email_events  │
    let webhook_routes = Router::new()
        .route("/webhooks/sendgrid", post(handlers::sendgrid_webhook))
        .layer(middleware::from_fn(validation::request_size_validation_middleware))
        .layer(middleware::from_fn(security::security_headers_middleware))
        .layer(middleware::from_fn_with_state(
            security::WebhookConfig {
                secret: state.config.sendgrid_webhook_secret.clone(),
                replay_window_secs: state.config.webhook_replay_window_secs,
            },
            security::sendgrid_webhook_middleware,
        ))
        .layer(middleware::from_fn(correlation::correlation_id_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let admin_routes = Router::new()
        .route(
            "/api/v1/markets/:market_id/resolve",
            post(handlers::resolve_market),
        )
        .route(
            "/api/blockchain/replay",
            post(handlers::blockchain_replay),
        )
        .route(
            "/api/v1/email/preview/:template_name",
            get(handlers::email_preview),
        )
        .route(
            "/api/v1/email/test",
            post(handlers::email_send_test),
        )
        .route(
            "/api/v1/email/analytics",
            get(handlers::email_analytics),
        )
        .route(
            "/api/v1/email/queue/stats",
            get(handlers::email_queue_stats),
        )
        .route(
            "/api/v1/email/queue/dead-letter",
            get(handlers::email_dead_letter_list),
        )
        .route(
            "/api/v1/email/queue/dead-letter/:job_id/requeue",
            post(handlers::email_dead_letter_requeue),
        )
        .route(
            "/api/v1/audit/logs",
            get(handlers::audit_logs),
        )
        .route(
            "/api/v1/audit/statistics",
            get(handlers::audit_statistics),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            idempotency::idempotency_middleware,
        ))
        .layer(middleware::from_fn(validation::content_type_validation_middleware))
        .layer(middleware::from_fn(validation::request_size_validation_middleware))
        .layer(middleware::from_fn_with_state(
            (ip_whitelist.clone(), security::TrustProxy(config_trust_proxy)),
            security::ip_whitelist_middleware,
        ))
        .layer(middleware::from_fn_with_state(api_key_auth.clone(), security::api_key_middleware))
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit::admin_rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), audit_middleware::audit_logging_middleware))
        .layer(middleware::from_fn(correlation::correlation_id_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let app = Router::new()
        .merge(public_routes)
        .merge(metrics_routes)
        .merge(newsletter_routes)
        .merge(webhook_routes)
        .merge(admin_routes)
        .layer(middleware::from_fn(validation::request_validation_middleware))
        .layer(middleware::from_fn(validation::request_size_validation_middleware))
        .layer(middleware::from_fn(security::security_headers_middleware))
        .layer(compression::compression_layer())
        .layer(cors_layer);

    // ── Server + graceful shutdown ────────────────────────────────────────────
    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!("API listening on {bind_addr}");

    // Axum's built-in graceful shutdown drains in-flight HTTP requests.
    // After the server stops accepting connections we then drain background workers.
    let axum_shutdown = {
        let coord = coordinator.clone();
        let email_coord = email_coordinator.clone();
        async move {
            wait_for_signal().await;
            tracing::info!("Shutdown signal received — stopping HTTP server");

            // Drain email queue first with its own timeout to avoid losing in-flight emails.
            let email_drain = shutdown::email_queue_drain_timeout();
            tracing::info!(
                timeout_secs = email_drain.as_secs(),
                "Draining email queue worker"
            );
            match email_coord.shutdown(email_drain).await {
                Ok(_) => tracing::info!("Email queue worker drained cleanly"),
                Err(e) => tracing::warn!("Email queue drain timeout — in-flight email may be lost: {e}"),
            }

            // Then drain remaining background workers (blockchain sync, tx-monitor).
            let timeout_dur = shutdown_timeout();
            tracing::info!(
                timeout_secs = timeout_dur.as_secs(),
                "Waiting for remaining background workers to drain"
            );
            match coord.shutdown(timeout_dur).await {
                Ok(_) => tracing::info!("All background workers stopped cleanly"),
                Err(e) => tracing::warn!("Shutdown timeout — forcing exit: {e}"),
            }
            tracing_config::shutdown_tracing();
        }
    };

    axum::serve(listener, app)
        .with_graceful_shutdown(axum_shutdown)
        .await?;

    Ok(())
}
