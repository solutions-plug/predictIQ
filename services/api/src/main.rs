mod blockchain;
mod cache;
mod config;
mod db;
mod handlers;
mod metrics;
mod newsletter;
mod rate_limit;
mod security;
mod validation;

use std::sync::Arc;

use axum::{
    middleware,
    routing::{get, post},
    Router,
};
use blockchain::BlockchainClient;
use cache::RedisCache;
use config::Config;
use db::Database;
use metrics::Metrics;
use newsletter::IpRateLimiter;
use security::{ApiKeyAuth, IpWhitelist, RateLimiter};
use tokio::net::TcpListener;
use tower_http::{
    compression::CompressionLayer,
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
    pub(crate) newsletter_rate_limiter: IpRateLimiter,
    pub(crate) rate_limiter: Arc<RateLimiter>,
    pub(crate) api_key_auth: Arc<ApiKeyAuth>,
    pub(crate) ip_whitelist: Arc<IpWhitelist>,
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

    let bind_addr = config.bind_addr;

    // Initialize security components
    let rate_limiter = Arc::new(RateLimiter::new());
    let api_key_auth = Arc::new(ApiKeyAuth::new(config.api_keys.clone()));
    let ip_whitelist = Arc::new(IpWhitelist::new(config.admin_whitelist_ips.clone()));

    // Start rate limiter cleanup task
    let rate_limiter_cleanup = rate_limiter.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(300));
        loop {
            interval.tick().await;
            rate_limiter_cleanup.cleanup().await;
        }
    });

    let state = Arc::new(AppState {
        config,
        cache,
        db,
        blockchain,
        metrics,
        newsletter_rate_limiter: IpRateLimiter::default(),
        rate_limiter: rate_limiter.clone(),
        api_key_auth: api_key_auth.clone(),
        ip_whitelist: ip_whitelist.clone(),
    });

    Arc::new(state.blockchain.clone()).start_background_tasks();

    if let Err(err) = handlers::warm_critical_caches(state.clone()).await {
        tracing::warn!("cache warming skipped: {err}");
    }

    // CORS configuration
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes (with global rate limiting)
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
        ));

    // Newsletter routes (with specific rate limiting)
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
        ));

    // Admin routes (with API key auth, IP whitelist, and rate limiting)
    let admin_routes = Router::new()
        .route(
            "/api/markets/:market_id/resolve",
            post(handlers::resolve_market),
        )
        .layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            rate_limit::admin_rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            api_key_auth,
            security::api_key_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            ip_whitelist,
            security::ip_whitelist_middleware,
        ));

    let app = Router::new()
        .merge(public_routes)
        .merge(newsletter_routes)
        .merge(admin_routes)
        .layer(middleware::from_fn(security::security_headers_middleware))
        .layer(middleware::from_fn(validation::request_validation_middleware))
        .layer(middleware::from_fn(validation::content_type_validation_middleware))
        .layer(middleware::from_fn(validation::request_size_validation_middleware))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = TcpListener::bind(bind_addr).await?;
    tracing::info!("API listening on {bind_addr}");
    axum::serve(listener, app).await?;

    Ok(())
}
