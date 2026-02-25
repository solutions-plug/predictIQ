use std::{
    collections::HashSet,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{sync::RwLock, time::sleep};

use crate::{
    cache::{keys, RedisCache},
    config::Config,
    metrics::Metrics,
};

#[derive(Clone)]
pub struct BlockchainClient {
    http: Client,
    rpc_url: String,
    network: String,
    contract_id: String,
    retry_attempts: u32,
    retry_base_delay_ms: u64,
    event_poll_interval: Duration,
    tx_poll_interval: Duration,
    confirmation_ledger_lag: u32,
    sync_market_ids: Vec<i64>,
    cache: RedisCache,
    metrics: Metrics,
    monitor: Arc<MonitoringState>,
}

#[derive(Default)]
struct MonitoringState {
    watched_txs: RwLock<HashSet<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMarketData {
    pub market_id: i64,
    pub title: Option<String>,
    pub status: Option<String>,
    pub onchain_volume: String,
    pub resolved_outcome: Option<u32>,
    pub ledger: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformStatistics {
    pub total_markets: u64,
    pub active_markets: u64,
    pub resolved_markets: u64,
    pub total_volume: String,
    pub ledger: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBet {
    pub market_id: i64,
    pub outcome: u32,
    pub amount: String,
    pub token: Option<String>,
    pub ledger: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserBetsPage {
    pub user: String,
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub items: Vec<UserBet>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleResult {
    pub market_id: i64,
    pub source: Option<String>,
    pub outcome: Option<u32>,
    pub confidence_bps: Option<u64>,
    pub ledger: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatus {
    pub hash: String,
    pub status: String,
    pub ledger: Option<u32>,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainHealth {
    pub network: String,
    pub rpc_url: String,
    pub latest_ledger: u32,
    pub is_healthy: bool,
    pub contract_reachable: bool,
    pub checked_at_unix: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    pub id: String,
    pub ledger: u32,
    pub topic: String,
    pub tx_hash: Option<String>,
    pub value: Value,
}

#[derive(Debug, Deserialize)]
struct RpcEnvelope<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

impl BlockchainClient {
    pub fn new(config: &Config, cache: RedisCache, metrics: Metrics) -> anyhow::Result<Self> {
        let http = Client::builder()
            .pool_max_idle_per_host(16)
            .pool_idle_timeout(Duration::from_secs(60))
            .connect_timeout(Duration::from_secs(3))
            .timeout(Duration::from_secs(10))
            .build()
            .context("failed to construct RPC http client")?;

        Ok(Self {
            http,
            rpc_url: config.blockchain_rpc_url.clone(),
            network: config.network_name().to_string(),
            contract_id: config.contract_id.clone(),
            retry_attempts: config.retry_attempts.max(1),
            retry_base_delay_ms: config.retry_base_delay_ms.max(50),
            event_poll_interval: config.event_poll_interval,
            tx_poll_interval: config.tx_poll_interval,
            confirmation_ledger_lag: config.confirmation_ledger_lag.max(1),
            sync_market_ids: config.sync_market_ids.clone(),
            cache,
            metrics,
            monitor: Arc::new(MonitoringState::default()),
        })
    }

    async fn rpc_call<T: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: Value,
    ) -> anyhow::Result<T> {
        let mut attempt: u32 = 0;

        loop {
            attempt += 1;

            let payload = json!({
                "jsonrpc": "2.0",
                "id": format!("{}-{}", method, attempt),
                "method": method,
                "params": params,
            });

            let response = self.http.post(&self.rpc_url).json(&payload).send().await;

            match response {
                Ok(resp) => {
                    let parsed = resp
                        .error_for_status()
                        .context("rpc status error")?
                        .json::<RpcEnvelope<T>>()
                        .await
                        .context("rpc parse error")?;

                    if let Some(err) = parsed.error {
                        if attempt >= self.retry_attempts {
                            return Err(anyhow!(
                                "rpc {} failed: {} ({})",
                                method,
                                err.message,
                                err.code
                            ));
                        }
                    } else if let Some(result) = parsed.result {
                        return Ok(result);
                    } else if attempt >= self.retry_attempts {
                        return Err(anyhow!("rpc {} returned empty result", method));
                    }
                }
                Err(err) => {
                    if attempt >= self.retry_attempts {
                        return Err(anyhow!("rpc {} transport failed: {err}", method));
                    }
                }
            }

            let backoff = self.retry_base_delay_ms * u64::from(attempt);
            sleep(Duration::from_millis(backoff)).await;
        }
    }

    async fn latest_ledger(&self) -> anyhow::Result<u32> {
        #[derive(Debug, Deserialize)]
        struct LatestLedgerResult {
            sequence: u32,
        }

        #[derive(Debug, Deserialize)]
        struct GetLatestLedgerResult {
            #[serde(rename = "latestLedger")]
            latest_ledger: LatestLedgerResult,
        }

        let result: GetLatestLedgerResult = self.rpc_call("getLatestLedger", json!({})).await?;
        Ok(result.latest_ledger.sequence)
    }

    pub async fn market_data_cached(&self, market_id: i64) -> anyhow::Result<ChainMarketData> {
        let key = keys::chain_market(market_id);
        let ttl = Duration::from_secs(60);
        let endpoint = "market_data";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let ledger = self.latest_ledger().await.unwrap_or(0);
                let data: Value = self
                    .rpc_call(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": format!("market:{}", market_id),
                        }),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        json!({
                            "title": null,
                            "status": null,
                            "onchain_volume": "0",
                            "resolved_outcome": null
                        })
                    });

                Ok(ChainMarketData {
                    market_id,
                    title: data
                        .get("title")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    status: data
                        .get("status")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    onchain_volume: data
                        .get("onchain_volume")
                        .and_then(Value::as_str)
                        .unwrap_or("0")
                        .to_string(),
                    resolved_outcome: data
                        .get("resolved_outcome")
                        .and_then(Value::as_u64)
                        .map(|v| v as u32),
                    ledger,
                })
            })
            .await?;

        if hit {
            self.metrics.observe_hit("chain", endpoint);
        } else {
            self.metrics.observe_miss("chain", endpoint);
        }

        Ok(value)
    }

    pub async fn platform_statistics_cached(&self) -> anyhow::Result<PlatformStatistics> {
        let key = keys::chain_platform_stats(&self.network);
        let ttl = Duration::from_secs(120);
        let endpoint = "platform_stats";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let ledger = self.latest_ledger().await.unwrap_or(0);
                let data: Value = self
                    .rpc_call(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": "platform:stats",
                        }),
                    )
                    .await
                    .unwrap_or_else(|_| {
                        json!({
                            "total_markets": 0,
                            "active_markets": 0,
                            "resolved_markets": 0,
                            "total_volume": "0"
                        })
                    });

                Ok(PlatformStatistics {
                    total_markets: data
                        .get("total_markets")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    active_markets: data
                        .get("active_markets")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    resolved_markets: data
                        .get("resolved_markets")
                        .and_then(Value::as_u64)
                        .unwrap_or(0),
                    total_volume: data
                        .get("total_volume")
                        .and_then(Value::as_str)
                        .unwrap_or("0")
                        .to_string(),
                    ledger,
                })
            })
            .await?;

        if hit {
            self.metrics.observe_hit("chain", endpoint);
        } else {
            self.metrics.observe_miss("chain", endpoint);
        }

        Ok(value)
    }

    pub async fn user_bets_cached(
        &self,
        user: &str,
        page: i64,
        page_size: i64,
    ) -> anyhow::Result<UserBetsPage> {
        let page = page.max(1);
        let page_size = page_size.clamp(1, 100);
        let key = keys::chain_user_bets(&self.network, user, page, page_size);
        let ttl = Duration::from_secs(30);
        let endpoint = "user_bets";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let ledger = self.latest_ledger().await.unwrap_or(0);
                let data: Value = self
                    .rpc_call(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": format!("user_bets:{}", user),
                        }),
                    )
                    .await
                    .unwrap_or_else(|_| json!({"bets": []}));

                let bets = data
                    .get("bets")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();

                let total = bets.len() as i64;
                let offset = ((page - 1) * page_size) as usize;
                let paged = bets
                    .into_iter()
                    .skip(offset)
                    .take(page_size as usize)
                    .collect::<Vec<_>>();

                let items = paged
                    .into_iter()
                    .map(|entry| UserBet {
                        market_id: entry
                            .get("market_id")
                            .and_then(Value::as_i64)
                            .unwrap_or_default(),
                        outcome: entry
                            .get("outcome")
                            .and_then(Value::as_u64)
                            .unwrap_or_default() as u32,
                        amount: entry
                            .get("amount")
                            .and_then(Value::as_str)
                            .unwrap_or("0")
                            .to_string(),
                        token: entry
                            .get("token")
                            .and_then(Value::as_str)
                            .map(ToOwned::to_owned),
                        ledger,
                    })
                    .collect::<Vec<_>>();

                Ok(UserBetsPage {
                    user: user.to_string(),
                    page,
                    page_size,
                    total,
                    items,
                })
            })
            .await?;

        if hit {
            self.metrics.observe_hit("chain", endpoint);
        } else {
            self.metrics.observe_miss("chain", endpoint);
        }

        Ok(value)
    }

    pub async fn oracle_result_cached(&self, market_id: i64) -> anyhow::Result<OracleResult> {
        let key = keys::chain_oracle_result(&self.network, market_id);
        let ttl = Duration::from_secs(30);
        let endpoint = "oracle_result";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let ledger = self.latest_ledger().await.unwrap_or(0);
                let data: Value = self
                    .rpc_call(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": format!("oracle_result:{}", market_id),
                        }),
                    )
                    .await
                    .unwrap_or_else(|_| json!({}));

                Ok(OracleResult {
                    market_id,
                    source: data
                        .get("source")
                        .and_then(Value::as_str)
                        .map(ToOwned::to_owned),
                    outcome: data
                        .get("outcome")
                        .and_then(Value::as_u64)
                        .map(|v| v as u32),
                    confidence_bps: data.get("confidence_bps").and_then(Value::as_u64),
                    ledger,
                })
            })
            .await?;

        if hit {
            self.metrics.observe_hit("chain", endpoint);
        } else {
            self.metrics.observe_miss("chain", endpoint);
        }

        Ok(value)
    }

    pub async fn transaction_status_cached(&self, hash: &str) -> anyhow::Result<TransactionStatus> {
        let key = keys::chain_tx_status(&self.network, hash);
        let ttl = Duration::from_secs(20);
        let endpoint = "tx_status";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                #[derive(Debug, Deserialize)]
                struct TxResponse {
                    status: String,
                    #[serde(rename = "ledger")]
                    ledger: Option<u32>,
                    #[serde(rename = "errorResultXdr")]
                    error_result_xdr: Option<String>,
                }

                let tx = self
                    .rpc_call::<TxResponse>("getTransaction", json!({ "hash": hash }))
                    .await
                    .unwrap_or(TxResponse {
                        status: "NOT_FOUND".to_string(),
                        ledger: None,
                        error_result_xdr: None,
                    });

                Ok(TransactionStatus {
                    hash: hash.to_string(),
                    status: tx.status,
                    ledger: tx.ledger,
                    error: tx.error_result_xdr,
                })
            })
            .await?;

        if hit {
            self.metrics.observe_hit("chain", endpoint);
        } else {
            self.metrics.observe_miss("chain", endpoint);
        }

        Ok(value)
    }

    pub async fn health_check_cached(&self) -> anyhow::Result<BlockchainHealth> {
        let key = keys::chain_health(&self.network);
        let ttl = Duration::from_secs(15);
        let endpoint = "health";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let latest = self.latest_ledger().await.unwrap_or(0);
                let contract_reachable = self
                    .rpc_call::<Value>(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": "platform:stats",
                        }),
                    )
                    .await
                    .is_ok();

                let checked_at_unix = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                Ok(BlockchainHealth {
                    network: self.network.clone(),
                    rpc_url: self.rpc_url.clone(),
                    latest_ledger: latest,
                    is_healthy: latest > 0,
                    contract_reachable,
                    checked_at_unix,
                })
            })
            .await?;

        if hit {
            self.metrics.observe_hit("chain", endpoint);
        } else {
            self.metrics.observe_miss("chain", endpoint);
        }

        Ok(value)
    }

    async fn fetch_events_since(&self, from_ledger: u32) -> anyhow::Result<Vec<ContractEvent>> {
        #[derive(Debug, Deserialize)]
        struct EventsResponse {
            events: Vec<Value>,
        }

        let result = self
            .rpc_call::<EventsResponse>(
                "getEvents",
                json!({
                    "startLedger": from_ledger,
                    "filters": [{"type": "contract", "contractIds": [self.contract_id]}],
                    "limit": 100,
                }),
            )
            .await
            .unwrap_or(EventsResponse { events: vec![] });

        let events = result
            .events
            .into_iter()
            .map(|e| ContractEvent {
                id: e
                    .get("id")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string(),
                ledger: e.get("ledger").and_then(Value::as_u64).unwrap_or_default() as u32,
                topic: e
                    .get("topic")
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                tx_hash: e
                    .get("txHash")
                    .and_then(Value::as_str)
                    .map(ToOwned::to_owned),
                value: e,
            })
            .collect::<Vec<_>>();

        Ok(events)
    }

    async fn handle_reorg_if_detected(&self, latest_ledger: u32) -> anyhow::Result<()> {
        let key = keys::chain_last_seen_ledger(&self.network);
        let previous = self.cache.get_json::<u32>(&key).await?.unwrap_or(0);

        if previous > 0 && latest_ledger + self.confirmation_ledger_lag < previous {
            let purged = self
                .cache
                .del_by_pattern(&format!("{}:*", keys::CHAIN_PREFIX))
                .await?;
            self.metrics.observe_invalidation("chain_reorg", purged);
        }

        self.cache
            .set_json(&key, &latest_ledger, Duration::from_secs(24 * 60 * 60))
            .await?;
        Ok(())
    }

    async fn sync_once(&self, cursor_ledger: u32) -> anyhow::Result<u32> {
        let latest = self.latest_ledger().await.unwrap_or(cursor_ledger);
        self.handle_reorg_if_detected(latest).await?;

        let confirmed_tip = latest.saturating_sub(self.confirmation_ledger_lag);
        if confirmed_tip <= cursor_ledger {
            return Ok(cursor_ledger);
        }

        let events = self.fetch_events_since(cursor_ledger + 1).await?;
        for event in events {
            let event_key = format!("{}:event:{}", keys::CHAIN_PREFIX, event.id);
            self.cache
                .set_json(&event_key, &event, Duration::from_secs(30 * 60))
                .await?;

            if let Some(hash) = event.tx_hash {
                self.watch_transaction(&hash).await;
            }
        }

        for market_id in &self.sync_market_ids {
            let _ = self.market_data_cached(*market_id).await;
            let _ = self.oracle_result_cached(*market_id).await;
        }

        let _ = self.platform_statistics_cached().await;

        Ok(confirmed_tip)
    }

    pub async fn run_sync_worker(self: Arc<Self>) {
        let cursor_key = keys::chain_sync_cursor(&self.network);
        let mut cursor = self
            .cache
            .get_json::<u32>(&cursor_key)
            .await
            .ok()
            .flatten()
            .unwrap_or(0);

        loop {
            match self.sync_once(cursor).await {
                Ok(next_cursor) => {
                    cursor = next_cursor;
                    let _ = self
                        .cache
                        .set_json(&cursor_key, &cursor, Duration::from_secs(24 * 60 * 60))
                        .await;
                }
                Err(err) => tracing::warn!("sync loop error: {err}"),
            }

            sleep(self.event_poll_interval).await;
        }
    }

    pub async fn run_transaction_monitor(self: Arc<Self>) {
        loop {
            let hashes = self
                .monitor
                .watched_txs
                .read()
                .await
                .iter()
                .cloned()
                .collect::<Vec<_>>();

            for hash in hashes {
                if let Ok(status) = self.transaction_status_cached(&hash).await {
                    if status.status != "NOT_FOUND" && status.status != "PENDING" {
                        self.monitor.watched_txs.write().await.remove(&hash);
                    }
                }
            }

            sleep(self.tx_poll_interval).await;
        }
    }

    pub async fn watch_transaction(&self, hash: &str) {
        self.monitor
            .watched_txs
            .write()
            .await
            .insert(hash.to_string());
    }

    pub fn start_background_tasks(self: Arc<Self>) {
        let sync_client = self.clone();
        tokio::spawn(async move {
            sync_client.run_sync_worker().await;
        });

        let monitor_client = self;
        tokio::spawn(async move {
            monitor_client.run_transaction_monitor().await;
        });
    }
}
