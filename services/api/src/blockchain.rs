use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::{
    cache::{keys, RedisCache},
    metrics::Metrics,
};

#[derive(Clone)]
pub struct BlockchainClient {
    http: reqwest::Client,
    rpc_url: String,
    cache: RedisCache,
    metrics: Metrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMarketData {
    pub market_id: i64,
    pub onchain_volume: String,
    pub resolved_outcome: Option<u32>,
}

impl BlockchainClient {
    pub fn new(rpc_url: String, cache: RedisCache, metrics: Metrics) -> Self {
        Self {
            http: reqwest::Client::new(),
            rpc_url,
            cache,
            metrics,
        }
    }

    pub async fn market_data_cached(&self, market_id: i64) -> anyhow::Result<ChainMarketData> {
        let key = keys::chain_market(market_id);
        let ttl = Duration::from_secs(60);
        let endpoint = "market_data";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                // Replace this call with the exact Soroban RPC method used by your backend.
                let url = format!(
                    "{}/market/{}",
                    self.rpc_url.trim_end_matches('/'),
                    market_id
                );
                let response = self.http.get(url).send().await?.error_for_status()?;
                let body = response.json::<ChainMarketData>().await?;
                Ok(body)
            })
            .await?;

        if hit {
            self.metrics.observe_hit("chain", endpoint);
        } else {
            self.metrics.observe_miss("chain", endpoint);
        }

        Ok(value)
    }
}
