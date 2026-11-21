use redis::aio::ConnectionManager;
use redis::Client;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;
use std::time::Duration;

use crate::config::Config;

/// Build the Postgres connection pool.
pub async fn build_pg_pool(config: &Config) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(config.database_max_connections)
        .acquire_timeout(Duration::from_secs(5))
        .connect(&config.database_url)
        .await
}

/// Build a Redis connection manager.
///
/// The URL may use either the `redis://` or `rediss://` scheme. When the
/// `rediss://` scheme is used the client negotiates TLS, which is required by
/// ElastiCache clusters that enforce in-transit encryption.
pub async fn build_redis_manager(config: &Config) -> Result<ConnectionManager, redis::RedisError> {
    let client = Client::open(config.redis_url.as_str())?;
    ConnectionManager::new(client).await
}

/// Returns true when the configured Redis URL requests TLS (`rediss://`).
pub fn redis_url_uses_tls(redis_url: &str) -> bool {
    redis_url.starts_with("rediss://")
}

#[cfg(test)]
mod tests {
    use super::redis_url_uses_tls;

    #[test]
    fn detects_rediss_scheme() {
        assert!(redis_url_uses_tls("rediss://cache.example:6379"));
        assert!(!redis_url_uses_tls("redis://cache.example:6379"));
    }
}
