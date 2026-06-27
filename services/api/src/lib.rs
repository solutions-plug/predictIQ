pub mod audit;
pub mod audit_middleware;
#[cfg(test)]
mod resolve_market_tests;
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
