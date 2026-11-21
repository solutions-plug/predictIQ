pub mod audit;
pub mod audit_middleware;
pub mod batch_53_implementations;
pub mod body_redact;
pub mod client_ip;
pub mod content_type;
pub mod cors;
pub mod csrf;
#[cfg(test)]
mod resolve_market_tests;
#[cfg(test)]
mod cancel_market_tests;
#[cfg(test)]
mod api_key_auth_tests;
#[cfg(test)]
mod shutdown_tests;
pub mod blockchain;
pub mod cache;
pub mod compression;
pub mod config;
pub mod correlation;
pub mod db;
pub mod email;
pub mod handlers;
pub mod idempotency;
pub mod metrics;
pub mod migrations;
pub mod newsletter;
pub mod pagination;
pub mod rate_limit;
pub mod security;
pub mod shutdown;
pub mod tracing_config;
pub mod validation;
pub mod versioning;
pub mod openapi_spec;

// Re-export AppState so integration tests can construct it.
pub use crate::app_state::AppState;

mod app_state {
    use crate::{
        audit::AuditLogger,
        blockchain::BlockchainClient,
        cache::RedisCache,
        config::Config,
        db::Database,
        email::{queue::EmailQueue, service::EmailService, webhook::WebhookHandler},
        metrics::Metrics,
        newsletter::IpRateLimiter,
    };

    #[derive(Clone)]
    pub struct AppState {
        pub config: Config,
        pub cache: RedisCache,
        pub db: Database,
        pub blockchain: BlockchainClient,
        pub metrics: Metrics,
        pub newsletter_rate_limiter: IpRateLimiter,
        pub email_service: EmailService,
        pub email_queue: EmailQueue,
        pub webhook_handler: WebhookHandler,
        pub audit_logger: AuditLogger,
    }
}

use axum::{routing::get, Router};

/// Builds the application router.
///
/// The body-size limit is read once from `config` (which is loaded at startup)
/// and the body-size middleware is layered exactly once on the router.
pub fn build_router(state: AppState) -> Router {
    // Read the body-size limit once at startup from the centralised config.
    let body_size_limit = state.config.body_size_limit;

    let router = Router::new()
        .route("/health", get(handlers::health::health_check))
        .with_state(state);

    // Apply the body-size middleware exactly once, using the startup-loaded limit.
    compression::apply_body_size_limit(router, body_size_limit)
}