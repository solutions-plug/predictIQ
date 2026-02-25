mod blockchain;
mod cache;
mod config;
mod db;
mod email;
mod handlers;
mod metrics;
mod newsletter;

use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use blockchain::BlockchainClient;
use cache::RedisCache;
use config::Config;
use db::Database;
use email::{queue::EmailQueue, service::EmailService, webhook::WebhookHandler};
use metrics::Metrics;
use newsletter::IpRateLimiter;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Clone)]
pub struct AppState {
    pub(crate) config: Config,
    pub(crate) cache: RedisCache,
    pub(crate) db: Database,
    pub(crate) blockchain: BlockchainClient,
    pub(crate) metrics: Metrics,
    pub(crate) newsletter_rate_limiter: IpRateLimiter,
    pub(crate) email_service: EmailService,
    pub(crate) email_queue: EmailQueue,
    pub(crate) webhook_handler: WebhookHandler,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let config = Config::from_env();
    let metrics = Metrics::new()?;
    let cache = RedisCache::new(&config.redis_url).await?;
    let db = Database::new(&config.database_url, cache.clone(), metrics.clone()).await?;
    let blockchain = BlockchainClient::new(&config, cache.clone(), metrics.clone())?;
    
    // Initialize email service components
    let email_service = EmailService::new(config.clone())?;
    let email_queue = EmailQueue::new(cache.clone(), db.clone());
    let webhook_handler = WebhookHandler::new(db.clone());

    let bind_addr = config.bind_addr;

    let state = Arc::new(AppState {
        config,
        cache,
        db,
        blockchain,
        metrics,
        newsletter_rate_limiter: IpRateLimiter::default(),
        email_service: email_service.clone(),
        email_queue: email_queue.clone(),
        webhook_handler: webhook_handler.clone(),
    });

    Arc::new(state.blockchain.clone()).start_background_tasks();
    
    // Start email queue worker in background
    let queue_worker = email_queue.clone();
    let service_worker = email_service.clone();
    tokio::spawn(async move {
        queue_worker.start_worker(service_worker).await;
    });

    if let Err(err) = handlers::warm_critical_caches(state.clone()).await {
        tracing::warn!("cache warming skipped: {err}");
    }

    let app = Router::new()
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
        .route(
            "/api/markets/:market_id/resolve",
            post(handlers::resolve_market),
        )
        // Email service endpoints
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
            "/webhooks/sendgrid",
            post(handlers::sendgrid_webhook),
        )
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!("API listening on {bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
