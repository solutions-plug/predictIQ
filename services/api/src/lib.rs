mod blockchain;
mod cache;
mod config;
mod db;
mod email;
mod handlers;
mod metrics;
mod newsletter;
mod rate_limit;
pub mod security;
mod shutdown;
mod validation;

#[cfg(test)]
mod shutdown_tests;

use std::sync::Arc;
use std::time::Duration;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use blockchain::BlockchainClient;
use cache::RedisCache;
use config::Config;
use db::Database;
use email::{queue::EmailQueue, service::EmailService, webhook::WebhookHandler};
use metrics::Metrics;
use security::{ApiKeyAuth, IpWhitelist, RateLimiter};
use shutdown::ShutdownCoordinator;
use tokio::net::TcpListener;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub(crate) config: Config,
    pub(crate) cache: RedisCache,
    pub(crate) db: Database,
    pub(crate) blockchain: BlockchainClient,
    pub(crate) metrics: Metrics,
    pub(crate) email_service: EmailService,
    pub(crate) email_queue: EmailQueue,
    pub(crate) webhook_handler: WebhookHandler,
}

pub async fn run() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let metrics = Metrics::new()?;
    let cache = RedisCache::new(&config.redis_url).await?;
    let db = Database::new(
        &config.database_url,
        &config.db_pool,
        cache.clone(),
        metrics.clone(),
    )
    .await?;
    let blockchain = BlockchainClient::new(&config, cache.clone(), metrics.clone())?;

    let email_service = EmailService::new(config.clone())?;
    let email_queue = EmailQueue::new(cache.clone(), db.clone());
    let webhook_handler = WebhookHandler::new(db.clone());

    let bind_addr = config.bind_addr;

    let rate_limiter = Arc::new(RateLimiter::new());
    let _api_key_auth = Arc::new(ApiKeyAuth::new(config.api_keys.clone()));
    let _ip_whitelist = Arc::new(IpWhitelist::new(config.admin_whitelist_ips.clone()));

    // Setup shutdown coordination for 3 workers: rate limiter cleanup, blockchain (2), email queue
    let shutdown_coordinator = ShutdownCoordinator::new(4);
    
    // Start rate limiter cleanup task
    let rate_limiter_cleanup = rate_limiter.clone();
    let mut cleanup_shutdown_rx = shutdown_coordinator.subscribe();
    let cleanup_handle = tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        
        loop {
            tokio::select! {
                _ = cleanup_shutdown_rx.recv() => {
                    tracing::info!("Rate limiter cleanup worker received shutdown signal");
                    // Perform final cleanup
                    rate_limiter_cleanup.cleanup().await;
                    tracing::info!("Rate limiter cleanup worker shutdown complete");
                    break;
                }
                _ = interval.tick() => {
                    rate_limiter_cleanup.cleanup().await;
                }
            }
        }
    });

    let state = Arc::new(AppState {
        config,
        cache,
        db,
        blockchain,
        metrics,
        email_service: email_service.clone(),
        email_queue: email_queue.clone(),
        webhook_handler: webhook_handler.clone(),
    });

    // Start blockchain background tasks
    let blockchain_handles = Arc::new(state.blockchain.clone())
        .start_background_tasks(&shutdown_coordinator);

    // Start email queue worker
    let queue_worker = email_queue.clone();
    let service_worker = email_service.clone();
    let email_shutdown_rx = shutdown_coordinator.subscribe();
    let email_handle = tokio::spawn(async move {
        queue_worker.start_worker(service_worker, email_shutdown_rx).await;
    });

    // Collect all worker handles
    let mut worker_handles = vec![
        shutdown::WorkerHandle::new(
            "rate-limiter-cleanup".to_string(),
            cleanup_handle,
            shutdown_coordinator.clone(),
        ),
        shutdown::WorkerHandle::new(
            "email-queue".to_string(),
            email_handle,
            shutdown_coordinator.clone(),
        ),
    ];
    worker_handles.extend(blockchain_handles);

    if let Err(err) = handlers::warm_critical_caches(state.clone()).await {
        tracing::warn!("cache warming skipped: {err}");
    }

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let public_routes = Router::new()
        .route("/health", get(handlers::health))
        .route("/metrics", get(handlers::metrics))
        .route("/api/blockchain/health", get(handlers::blockchain_health))
        .route(
            "/api/blockchain/markets/:market_id",
            get(handlers::blockchain_market_data),
        )
        .route(
            "/api/blockchain/stats",
            get(handlers::blockchain_platform_stats),
        )
        .route(
            "/api/blockchain/users/:user/bets",
            get(handlers::blockchain_user_bets),
        )
        .route(
            "/api/blockchain/oracle/:market_id",
            get(handlers::blockchain_oracle_result),
        )
        .route(
            "/api/blockchain/tx/:tx_hash",
            get(handlers::blockchain_tx_status),
        )
        .route("/api/statistics", get(handlers::statistics))
        .route("/api/markets/featured", get(handlers::featured_markets))
        .route("/api/content", get(handlers::content))
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            security::global_rate_limit_middleware,
        ))
        .with_state(state.clone());

    let newsletter_routes = Router::new()
        .route(
            "/api/v1/newsletter/subscribe",
            post(handlers::newsletter_subscribe),
        )
        .route(
            "/api/v1/newsletter/confirm",
            get(handlers::newsletter_confirm),
        )
        .route(
            "/api/v1/newsletter/unsubscribe",
            axum::routing::delete(handlers::newsletter_unsubscribe),
        )
        .route(
            "/api/v1/newsletter/gdpr/export",
            get(handlers::newsletter_gdpr_export),
        )
        .route(
            "/api/v1/newsletter/gdpr/delete",
            axum::routing::delete(handlers::newsletter_gdpr_delete),
        )
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit::newsletter_rate_limit_middleware,
        ))
        .with_state(state.clone());

    let admin_routes = Router::new()
        .route(
            "/api/markets/:market_id/resolve",
            post(handlers::resolve_market),
        )
        .route(
            "/api/v1/email/preview/:template_name",
            get(handlers::email_preview),
        )
        .route("/api/v1/email/test", post(handlers::email_send_test))
        .route("/api/v1/email/analytics", get(handlers::email_analytics))
        .route(
            "/api/v1/email/queue/stats",
            get(handlers::email_queue_stats),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    // Webhook routes use provider-signed auth (SendGrid HMAC), not admin API keys.
    let webhook_secret = state.config.sendgrid_webhook_secret.clone();
    let webhook_routes = Router::new()
        .route("/webhooks/sendgrid", post(handlers::sendgrid_webhook))
        .layer(middleware::from_fn_with_state(
            webhook_secret,
            security::sendgrid_webhook_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let app = public_routes
        .merge(newsletter_routes)
        .merge(admin_routes)
        .merge(webhook_routes)
        .layer(cors);

    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!("API listening on {bind_addr}");

    // Setup graceful shutdown
    let shutdown_coordinator_clone = shutdown_coordinator.clone();
    tokio::spawn(async move {
        if let Err(e) = shutdown::setup_signal_handlers().await {
            tracing::error!("Failed to setup signal handlers: {}", e);
            return;
        }
        
        // Initiate graceful shutdown with 30 second timeout
        if let Err(e) = shutdown_coordinator_clone.shutdown(Duration::from_secs(30)).await {
            tracing::error!("Graceful shutdown failed: {}", e);
        }
    });

    // Start the HTTP server with graceful shutdown
    let server = axum::serve(listener, app);
    let mut server_shutdown_rx = shutdown_coordinator.subscribe();
    
    tokio::select! {
        result = server => {
            if let Err(e) = result {
                tracing::error!("Server error: {}", e);
            }
        }
        _ = server_shutdown_rx.recv() => {
            tracing::info!("HTTP server received shutdown signal");
        }
    }

    // Wait for all workers to complete
    for handle in worker_handles {
        if let Err(e) = handle.join().await {
            tracing::error!("Worker join error: {:?}", e);
        }
    }

    tracing::info!("Application shutdown complete");
    Ok(())
}
