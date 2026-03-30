mod blockchain;
mod cache;
mod config;
mod db;
mod email;
mod handlers;
mod metrics;
mod newsletter;
mod rate_limit;
mod security;
mod validation;

#[cfg(test)]
mod api_key_auth_tests;

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
use email::{queue::EmailQueue, service::EmailService, webhook::WebhookHandler};
use metrics::Metrics;
use newsletter::IpRateLimiter;
use security::{ApiKeyAuth, IpWhitelist, RateLimiter, TrustProxy};
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
    pub(crate) email_service: EmailService,
    pub(crate) email_queue: EmailQueue,
    pub(crate) webhook_handler: WebhookHandler,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    predictiq_api::run().await
}
