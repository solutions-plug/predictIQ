use std::{future::Future, time::Duration};

use anyhow::Context;
use redis::{aio::ConnectionManager, AsyncCommands, Client};
use serde::{de::DeserializeOwned, Serialize};

#[derive(Clone)]
pub struct RedisCache {
    pub(crate) manager: ConnectionManager,
}

impl RedisCache {
    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let client = Client::open(redis_url).context("invalid REDIS_URL")?;
        let manager = client
            .get_connection_manager()
            .await
            .context("failed to connect to redis")?;
        Ok(Self { manager })
    }

    pub async fn get_json<T>(&self, key: &str) -> anyhow::Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let mut conn = self.manager.clone();
        let val: Option<String> = conn.get(key).await?;
        match val {
            Some(raw) => Ok(Some(serde_json::from_str(&raw)?)),
            None => Ok(None),
        }
    }

    pub async fn set_json<T>(&self, key: &str, value: &T, ttl: Duration) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let mut conn = self.manager.clone();
        let raw = serde_json::to_string(value)?;
        let _: () = conn.set_ex(key, raw, ttl.as_secs()).await?;
        Ok(())
    }

    pub async fn del(&self, key: &str) -> anyhow::Result<()> {
        let mut conn = self.manager.clone();
        let _: usize = conn.del(key).await?;
        Ok(())
    }

    pub async fn del_by_pattern(&self, pattern: &str) -> anyhow::Result<usize> {
        let mut conn = self.manager.clone();
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(pattern)
            .query_async(&mut conn)
            .await?;
        if keys.is_empty() {
            return Ok(0);
        }
        let deleted: usize = conn.del(keys).await?;
        Ok(deleted)
    }

    pub async fn get_or_set_json<T, F, Fut>(
        &self,
        key: &str,
        ttl: Duration,
        fetcher: F,
    ) -> anyhow::Result<(T, bool)>
    where
        T: Serialize + DeserializeOwned + Clone,
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<T>>,
    {
        if let Some(cached) = self.get_json(key).await? {
            return Ok((cached, true));
        }

        let value = fetcher().await?;
        self.set_json(key, &value, ttl).await?;
        Ok((value, false))
    }
}

pub mod keys {
    pub const API_PREFIX: &str = "api:v1";
    pub const DBQ_PREFIX: &str = "dbq:v1";
    pub const CHAIN_PREFIX: &str = "chain:v1";

    pub fn api_statistics() -> String {
        format!("{API_PREFIX}:statistics")
    }

    pub fn api_featured_markets() -> String {
        format!("{API_PREFIX}:featured_markets")
    }

    pub fn api_featured_markets_with_params(category: Option<&str>, page: i64, limit: i64) -> String {
        match category {
            Some(cat) => format!("{API_PREFIX}:featured_markets:cat:{}:page:{}:limit:{}", cat, page, limit),
            None => format!("{API_PREFIX}:featured_markets:page:{}:limit:{}", page, limit),
        }
    }

    pub fn api_content(page: i64, page_size: i64) -> String {
        format!("{API_PREFIX}:content:page:{page}:size:{page_size}")
    }

    pub fn dbq_statistics() -> String {
        format!("{DBQ_PREFIX}:statistics")
    }

    pub fn dbq_featured_markets(limit: i64) -> String {
        format!("{DBQ_PREFIX}:featured_markets:limit:{limit}")
    }

    pub fn dbq_content(page: i64, page_size: i64) -> String {
        format!("{DBQ_PREFIX}:content:page:{page}:size:{page_size}")
    }

    pub fn chain_market(market_id: i64) -> String {
        format!("{CHAIN_PREFIX}:market:{market_id}")
    }

    pub fn chain_platform_stats(network: &str) -> String {
        format!("{CHAIN_PREFIX}:platform_stats:{network}")
    }

    pub fn chain_user_bets(network: &str, user: &str, page: i64, page_size: i64) -> String {
        format!(
            "{CHAIN_PREFIX}:user_bets:{network}:{}:page:{page}:size:{page_size}",
            user.to_lowercase()
        )
    }

    pub fn chain_oracle_result(network: &str, market_id: i64) -> String {
        format!("{CHAIN_PREFIX}:oracle:{network}:market:{market_id}")
    }

    pub fn chain_tx_status(network: &str, tx_hash: &str) -> String {
        format!(
            "{CHAIN_PREFIX}:tx_status:{network}:{}",
            tx_hash.to_lowercase()
        )
    }

    pub fn chain_health(network: &str) -> String {
        format!("{CHAIN_PREFIX}:health:{network}")
    }

    pub fn chain_last_seen_ledger(network: &str) -> String {
        format!("{CHAIN_PREFIX}:last_seen_ledger:{network}")
    }

    pub fn chain_sync_cursor(network: &str) -> String {
        format!("{CHAIN_PREFIX}:sync_cursor:{network}")
    }
}
