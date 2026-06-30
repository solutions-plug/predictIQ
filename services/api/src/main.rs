use predictiq_api::{
    audit::AuditLogger,
    blockchain::BlockchainClient,
    cache::RedisCache,
    config::{Config, CorsConfig},
    csrf::{CsrfConfig, csrf_protection_middleware},
    db::Database,
    email::{queue::EmailQueue, service::EmailService, webhook::WebhookHandler},
    handlers,
    idempotency, correlation, versioning, validation, rate_limit, audit_middleware,
    metrics::Metrics,
    newsletter::IpRateLimiter,
    security::{self, ApiKeyAuth, IpWhitelist, MetricsAuthConfig, RateLimiter, RequireHttps},
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
    config.validate().map_err(|e| anyhow::anyhow!("{e}"))?;

    // Warn if production environment does not enforce HTTPS.
    config.check_https_config();

    let metrics = Metrics::new()?;

    // Warn at startup if the OTLP endpoint is unreachable so operators know
    // that traces are being dropped before any export attempt is made.
    if let Some(ref endpoint) = config.otlp_endpoint {
        if !tracing_config::check_otlp_connectivity(endpoint).await {
            metrics.observe_otel_export_error("unreachable");
        }
    }

    let cache = RedisCache::new(&config.redis_url).await?;
    let db = Database::new(&config.database_url, cache.clone(), metrics.clone(), &config.db_pool).await?;
    let db_arc = Arc::new(db.clone());
    let blockchain = BlockchainClient::new(&config, cache.clone(), db.clone(), metrics.clone())?;
    blockchain.validate_network_passphrase().await?;

    let email_service = EmailService::new(config.clone())?;
    let email_queue = EmailQueue::new(cache.clone(), db.clone());
    let webhook_handler = WebhookHandler::new(db.clone());
    let audit_logger = AuditLogger::new(db.pool());

    let bind_addr = config.bind_addr;
    let require_https = config.require_https;

    let rate_limiter = Arc::new(RateLimiter::new());
    // Use DB-backed ApiKeyAuth for zero-downtime key rotation (issue #892).
    let api_key_auth = Arc::new(ApiKeyAuth::new_with_db(
        config.api_keys.clone(),
        db_arc.clone(),
    ));
    let ip_whitelist = Arc::new(IpWhitelist::new(config.admin_whitelist_ips.clone()));
    let config_trust_proxy = config.trust_proxy;

    // CSRF config: derive allowed origins from the CORS config so the two
    // lists stay in sync.
    let csrf_config = Arc::new(CsrfConfig {
        allowed_origins: config.cors.allowed_origins.clone(),
    });

    // ── Shutdown coordinators ─────────────────────────────────────────────────
    // Email queue gets its own coordinator so it can be drained with a dedicated
    // timeout (EMAIL_QUEUE_DRAIN_TIMEOUT_SECS) that is independent of the global
    // worker shutdown timeout.  Losing in-flight emails is more costly than
    // delaying exit, so the email drain timeout defaults to 60 s.
    //
    // Blockchain workers (sync + tx-monitor) use the global coordinator.
    // The newsletter cleanup task is fire-and-forget (low-risk) so it is not tracked.
    let email_coordinator = ShutdownCoordinator::new(1);
    let coordinator = ShutdownCoordinator::new(2);

    // ── Rate-limiter cleanup (fire-and-forget) ────────────────────────────────
    let rate_limiter_cleanup = rate_limiter.clone();
    let metrics_rate_limiter = metrics.clone();
    tokio::spawn(async move {
        const WORKER_NAME: &str = "rate_limiter_cleanup";

        metrics_rate_limiter.set_worker_status(WORKER_NAME, true);

        let mut interval = tokio::time::interval(Duration::from_secs(300));
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));
        heartbeat_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        loop {
            tokio::select! {
                _ = interval.tick() => {
                    rate_limiter_cleanup.cleanup().await;
                }
                _ = heartbeat_interval.tick() => {
                    metrics_rate_limiter.set_worker_status(WORKER_NAME, true);
                }
            }
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
    // Restore watched transactions from the database before workers start polling.
    if let Err(e) = state.blockchain.load_watched_transactions().await {
        tracing::warn!(error = %e, "failed to restore watched transactions from database; starting with empty watch list");
    }
    let _blockchain_handles = Arc::new(state.blockchain.clone())
        .start_background_tasks(&coordinator);

    // ── Newsletter cleanup (fire-and-forget) ──────────────────────────────────
    let db_cleanup = state.clone();
    let metrics_newsletter = state.metrics.clone();
    tokio::spawn(async move {
        const WORKER_NAME: &str = "newsletter_cleanup";
        
        // Set worker status to running
        metrics_newsletter.set_worker_status(WORKER_NAME, true);
        
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        let mut heartbeat_interval = tokio::time::interval(Duration::from_secs(30));
        heartbeat_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    let ttl = db_cleanup.config.newsletter_token_ttl_secs;
                    let batch = db_cleanup.config.newsletter_cleanup_batch_size;
                    match db_cleanup.db.newsletter_delete_expired_pending(ttl, batch).await {
                        Ok(n) if n > 0 => tracing::info!("[newsletter] cleaned up {n} expired pending subscriptions"),
                        Err(e) => tracing::warn!("[newsletter] cleanup error: {e}"),
                        _ => {}
                    }
                }
                _ = heartbeat_interval.tick() => {
                    metrics_newsletter.set_worker_status(WORKER_NAME, true);
                }
            }
        }
    });

    // ── API key cleanup (fire-and-forget) ─────────────────────────────────────
    // Hard-deletes keys whose overlap window has expired (expires_at <= NOW()).
    // Runs every hour; failed iterations are logged and retried on the next tick.
    let db_api_key_cleanup = db_arc.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            interval.tick().await;
            match db_api_key_cleanup.api_key_delete_expired().await {
                Ok(n) if n > 0 => tracing::info!("[api-keys] cleaned up {n} expired keys"),
                Err(e) => tracing::warn!("[api-keys] cleanup error: {e}"),
                _ => {}
            }
        }
    });

    // ── Email queue worker (monitored restart loop) ────────────────────────────
    // Wraps the worker in a panic-catching JoinHandle. If the task panics or
    // exits unexpectedly it is restarted with exponential backoff (1s, 2s, 4s,
    // 8s, 16s). After MAX_EMAIL_WORKER_RESTARTS consecutive crashes without a
    // clean recovery the loop logs FATAL and increments worker_crash_total so
    // the Prometheus alert fires.
    {
        const MAX_EMAIL_WORKER_RESTARTS: u32 = 5;

        let queue_worker = email_queue.clone();
        let service_worker = email_service.clone();
        let email_token = email_coordinator.token();
        let email_coord = email_coordinator.clone();
        let stale_threshold = state.config.email_stale_job_threshold_secs;
        let crash_counter = state.metrics.worker_crash_total.clone();
        let metrics_worker = state.metrics.clone();

        tokio::spawn(async move {
            let mut restarts: u32 = 0;
            loop {
                let q = queue_worker.clone();
                let s = service_worker.clone();
                let token = email_token.clone();
                let coord = email_coord.clone();
                let mw = metrics_worker.clone();

                let handle = tokio::spawn(async move {
                    q.start_worker(s, token, coord, stale_threshold, Some(mw)).await;
                });

                match handle.await {
                    Ok(_) => {
                        // Clean exit (shutdown token was cancelled) — do not restart.
                        tracing::info!("Email queue worker exited cleanly");
                        break;
                    }
                    Err(e) => {
                        restarts += 1;
                        crash_counter.with_label_values(&["email_queue_worker"]).inc();

                        if restarts >= MAX_EMAIL_WORKER_RESTARTS {
                            tracing::error!(
                                restarts,
                                error = %e,
                                "FATAL: email queue worker has crashed {} times — alerting and giving up",
                                MAX_EMAIL_WORKER_RESTARTS,
                            );
                            break;
                        }

                        let backoff = Duration::from_secs(2_u64.pow(restarts - 1));
                        tracing::error!(
                            restarts,
                            backoff_secs = backoff.as_secs(),
                            error = %e,
                            "Email queue worker crashed — restarting after backoff"
                        );
                        tokio::time::sleep(backoff).await;
                    }
                }
            }
        });
    }

    if let Err(err) = handlers::warm_critical_caches(state.clone()).await {
        tracing::warn!("cache warming skipped: {err}");
    }

    // ── CORS ──────────────────────────────────────────────────────────────────
    let cors_layer = build_cors_layer(&state.config.cors);

    // ── Routes ────────────────────────────────────────────────────────────────
    // Health probes bypass rate limiting so the load balancer is never gated.
    let health_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/health/live", get(handlers::health_live))
        .route("/health/ready", get(handlers::health_ready))
        .with_state(state.clone());

    let public_routes = Router::new()
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
            state.clone(),
            rate_limit::global_rate_limit_middleware,
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
        // CSRF defense-in-depth: validate Origin/Referer on state-changing requests.
        .layer(middleware::from_fn_with_state(csrf_config, csrf_protection_middleware))
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
        // ── API key rotation endpoints (issue #892) ────────────────────────────
        .route(
            "/api/v1/admin/api-keys",
            get(handlers::list_api_keys),
        )
        .route(
            "/api/v1/admin/api-keys/rotate",
            post(handlers::rotate_api_key),
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
            state.clone(),
            rate_limit::admin_rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(state.clone(), audit_middleware::audit_logging_middleware))
        .layer(middleware::from_fn(correlation::correlation_id_middleware))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let app = Router::new()
        .merge(health_routes)
        .merge(public_routes)
        .merge(metrics_routes)
        .merge(newsletter_routes)
        .merge(webhook_routes)
        .merge(admin_routes)
        .layer(middleware::from_fn(validation::request_validation_middleware))
        .layer(middleware::from_fn(validation::request_size_validation_middleware))
        .layer(middleware::from_fn(security::security_headers_middleware))
        .layer(compression::compression_layer())
        .layer(cors_layer)
        // HTTPS redirect is the outermost layer: it runs before any other
        // middleware so plain-HTTP requests are bounced before touching app logic.
        .layer(middleware::from_fn_with_state(
            RequireHttps(require_https),
            security::https_redirect_middleware,
        ));

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
