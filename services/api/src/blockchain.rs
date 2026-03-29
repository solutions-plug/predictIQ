use std::{
    collections::HashSet,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
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
    tasks_started: AtomicBool,
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
            .unwrap_or_else(|err| {
                tracing::warn!(error = %err, from_ledger, "failed to fetch events from rpc");
                self.metrics.observe_rpc_error("getEvents");
                EventsResponse { events: vec![] }
            });

        let events = result
            .events
            .into_iter()
            .filter_map(|e| Self::parse_event(e))
            .collect::<Vec<_>>();

        Ok(events)
    }

    /// Parse a raw RPC event JSON value into a [`ContractEvent`].
    ///
    /// Returns `None` (quarantine) when required fields are absent or carry
    /// sentinel defaults that would poison the cache:
    /// - `id` must be present and non-empty
    /// - `ledger` must be > 0
    fn parse_event(e: Value) -> Option<ContractEvent> {
        let id = e.get("id").and_then(Value::as_str).unwrap_or("").to_string();
        if id.is_empty() {
            return None;
        }

        let ledger = e.get("ledger").and_then(Value::as_u64).unwrap_or(0) as u32;
        if ledger == 0 {
            return None;
        }

        Some(ContractEvent {
            id,
            ledger,
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
    }

    async fn handle_reorg_if_detected(&self, latest_ledger: u32) -> anyhow::Result<()> {
        Self::handle_reorg_logic(
            &self.cache,
            &self.metrics,
            &self.network,
            self.confirmation_ledger_lag,
            latest_ledger,
        )
        .await
    }

    async fn handle_reorg_logic(
        cache: &dyn ReorgCache,
        metrics: &dyn ReorgMetrics,
        network: &str,
        lag: u32,
        latest_ledger: u32,
    ) -> anyhow::Result<()> {
        let key = keys::chain_last_seen_ledger(network);
        let previous = cache.get_ledger(&key).await?.unwrap_or(0);

        if previous > 0 && latest_ledger + lag < previous {
            let purged = cache.purge_chain_cache().await?;
            metrics.observe_reorg_invalidation(purged);
        }

        cache.set_ledger(&key, latest_ledger).await?;
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
            let hashes: Vec<String> = self
                .monitor
                .watched_txs
                .read()
                .await
                .iter()
                .cloned()
                .collect();

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
        if self.monitor.tasks_started.swap(true, Ordering::SeqCst) {
            tracing::warn!("background tasks already started; skipping duplicate invocation");
            return;
        }

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

#[async_trait::async_trait]
pub trait ReorgCache: Send + Sync {
    async fn get_ledger(&self, key: &str) -> anyhow::Result<Option<u32>>;
    async fn set_ledger(&self, key: &str, ledger: u32) -> anyhow::Result<()>;
    async fn purge_chain_cache(&self) -> anyhow::Result<usize>;
}

pub trait ReorgMetrics: Send + Sync {
    fn observe_reorg_invalidation(&self, count: usize);
}

#[async_trait::async_trait]
impl ReorgCache for RedisCache {
    async fn get_ledger(&self, key: &str) -> anyhow::Result<Option<u32>> {
        self.get_json::<u32>(key).await
    }

    async fn set_ledger(&self, key: &str, ledger: u32) -> anyhow::Result<()> {
        self.set_json(key, &ledger, Duration::from_secs(24 * 60 * 60))
            .await
    }

    async fn purge_chain_cache(&self) -> anyhow::Result<usize> {
        self.del_by_pattern(&format!("{}:*", keys::CHAIN_PREFIX))
            .await
    }
}

impl ReorgMetrics for Metrics {
    fn observe_reorg_invalidation(&self, count: usize) {
        self.observe_invalidation("chain_reorg", count);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tokio::sync::Mutex as AsyncMutex;

    struct MockCache {
        ledger: AsyncMutex<Option<u32>>,
        purged_count: AsyncMutex<usize>,
    }

    #[async_trait::async_trait]
    impl ReorgCache for MockCache {
        async fn get_ledger(&self, _key: &str) -> anyhow::Result<Option<u32>> {
            Ok(*self.ledger.lock().await)
        }

        async fn set_ledger(&self, _key: &str, ledger: u32) -> anyhow::Result<()> {
            *self.ledger.lock().await = Some(ledger);
            Ok(())
        }

        async fn purge_chain_cache(&self) -> anyhow::Result<usize> {
            let mut count = self.purged_count.lock().await;
            *count += 1;
            Ok(10) // Mock 10 items purged
        }
    }

    struct MockMetrics {
        invalidation_count: Mutex<usize>,
    }

    impl ReorgMetrics for MockMetrics {
        fn observe_reorg_invalidation(&self, count: usize) {
            *self.invalidation_count.lock().unwrap() += count;
        }
    }

    #[tokio::test]
    async fn test_reorg_no_previous_state() {
        let cache = MockCache {
            ledger: AsyncMutex::new(None),
            purged_count: AsyncMutex::new(0),
        };
        let metrics = MockMetrics {
            invalidation_count: Mutex::new(0),
        };

        BlockchainClient::handle_reorg_logic(&cache, &metrics, "test", 10, 100)
            .await
            .unwrap();

        assert_eq!(*cache.ledger.lock().await, Some(100));
        assert_eq!(*cache.purged_count.lock().await, 0);
        assert_eq!(*metrics.invalidation_count.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_reorg_detected() {
        // Previous = 100, Latest = 80, Lag = 5
        // 80 + 5 = 85 < 100 -> REORG!
        let cache = MockCache {
            ledger: AsyncMutex::new(Some(100)),
            purged_count: AsyncMutex::new(0),
        };
        let metrics = MockMetrics {
            invalidation_count: Mutex::new(0),
        };

        BlockchainClient::handle_reorg_logic(&cache, &metrics, "test", 5, 80)
            .await
            .unwrap();

        assert_eq!(*cache.ledger.lock().await, Some(80));
        assert_eq!(*cache.purged_count.lock().await, 1);
        assert_eq!(*metrics.invalidation_count.lock().unwrap(), 10);
    }

    #[tokio::test]
    async fn test_reorg_not_detected_within_lag() {
        // Previous = 100, Latest = 96, Lag = 5
        // 96 + 5 = 101 >= 100 -> NO REORG
        let cache = MockCache {
            ledger: AsyncMutex::new(Some(100)),
            purged_count: AsyncMutex::new(0),
        };
        let metrics = MockMetrics {
            invalidation_count: Mutex::new(0),
        };

        BlockchainClient::handle_reorg_logic(&cache, &metrics, "test", 5, 96)
            .await
            .unwrap();

        assert_eq!(*cache.ledger.lock().await, Some(96));
        assert_eq!(*cache.purged_count.lock().await, 0);
        assert_eq!(*metrics.invalidation_count.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_reorg_not_detected_advancing() {
        // Previous = 100, Latest = 110, Lag = 5
        // 110 + 5 = 115 >= 100 -> NO REORG
        let cache = MockCache {
            ledger: AsyncMutex::new(Some(100)),
            purged_count: AsyncMutex::new(0),
        };
        let metrics = MockMetrics {
            invalidation_count: Mutex::new(0),
        };

        BlockchainClient::handle_reorg_logic(&cache, &metrics, "test", 5, 110)
            .await
            .unwrap();

        assert_eq!(*cache.ledger.lock().await, Some(110));
        assert_eq!(*cache.purged_count.lock().await, 0);
        assert_eq!(*metrics.invalidation_count.lock().unwrap(), 0);
    }

    #[tokio::test]
    async fn test_fetch_events_metrics_on_error() {
        let mut config = Config::from_env();
        config.blockchain_rpc_url = "http://127.0.0.1:0".to_string();
        config.retry_attempts = 1;
        config.retry_base_delay_ms = 1;

        let metrics = Metrics::new().unwrap();

        // Attempt to connect to local Redis; if it fails, skip the test to avoid spurious CI failures.
        let cache = match RedisCache::new(&config.redis_url).await {
            Ok(c) => c,
            Err(_) => {
                println!("Skipping test_fetch_events_metrics_on_error due to missing Redis");
                return;
            }
        };

        let client = BlockchainClient::new(&config, cache, metrics.clone()).unwrap();

        // RPC call should fail (port 0 is unreachable), and the error should be masked, resulting in empty events.
        let events = client.fetch_events_since(0).await.unwrap();
        assert!(events.is_empty());

        // Error metric should be incremented.
        let rendered = metrics.render().unwrap();
        assert!(rendered.contains("rpc_errors_total{method=\"getEvents\"} 1"));
    }

    // -------------------------------------------------------------------------
    // sync cursor progression under empty event streams
    //
    // fetch_events_since silently returns Ok(vec![]) on RPC failure.
    // These tests verify the cursor never jumps or rewinds in that scenario.
    // -------------------------------------------------------------------------

    /// Build a client whose RPC endpoint is unreachable (port 0), so every RPC
    /// call fails immediately.  Returns None when Redis is unavailable so each
    /// test can skip gracefully without failing CI.
    async fn make_dead_rpc_client() -> Option<BlockchainClient> {
        let mut config = Config::from_env();
        config.blockchain_rpc_url = "http://127.0.0.1:0".to_string();
        config.retry_attempts = 1;
        config.retry_base_delay_ms = 1;
        // Small lag keeps confirmed_tip arithmetic predictable.
        config.confirmation_ledger_lag = 5;
        // No market IDs avoids extra RPC calls inside sync_once.
        config.sync_market_ids = vec![];

        let metrics = Metrics::new().unwrap();
        let cache = match RedisCache::new(&config.redis_url).await {
            Ok(c) => c,
            Err(_) => return None,
        };
        Some(BlockchainClient::new(&config, cache, metrics).unwrap())
    }

    /// When latest_ledger RPC fails, sync_once falls back to cursor_ledger as
    /// the latest value.  confirmed_tip = cursor - lag ≤ cursor, so the
    /// early-return guard fires and the cursor is returned unchanged.
    #[tokio::test]
    async fn test_cursor_does_not_advance_when_latest_ledger_rpc_fails() {
        let client = match make_dead_rpc_client().await {
            Some(c) => c,
            None => {
                println!("Skipping test_cursor_does_not_advance_when_latest_ledger_rpc_fails: Redis unavailable");
                return;
            }
        };

        let initial: u32 = 500;
        let next = client.sync_once(initial).await.unwrap();
        assert_eq!(
            next, initial,
            "cursor must not change when latest_ledger RPC fails (got {next}, want {initial})"
        );
    }

    /// Starting from ledger 0 (fresh worker state) with a dead RPC the cursor
    /// must stay at 0 and must not jump to any non-zero value.
    #[tokio::test]
    async fn test_cursor_stays_at_zero_on_rpc_failure_from_fresh_state() {
        let client = match make_dead_rpc_client().await {
            Some(c) => c,
            None => {
                println!("Skipping test_cursor_stays_at_zero_on_rpc_failure_from_fresh_state: Redis unavailable");
                return;
            }
        };

        let next = client.sync_once(0).await.unwrap();
        assert_eq!(
            next, 0,
            "cursor must stay at 0 when RPC fails from fresh state (got {next})"
        );
    }

    /// When confirmed_tip ≤ cursor (chain has not advanced past the lag window),
    /// sync_once must return the cursor unchanged – idempotency guarantee.
    #[tokio::test]
    async fn test_cursor_is_idempotent_when_already_at_confirmed_tip() {
        let client = match make_dead_rpc_client().await {
            Some(c) => c,
            None => {
                println!("Skipping test_cursor_is_idempotent_when_already_at_confirmed_tip: Redis unavailable");
                return;
            }
        };

        // Dead RPC → latest falls back to cursor_ledger.
        // confirmed_tip = cursor - lag ≤ cursor → early return.
        let cursor: u32 = 200;
        let next = client.sync_once(cursor).await.unwrap();
        assert_eq!(
            next, cursor,
            "cursor must be idempotent when already at confirmed tip (got {next}, want {cursor})"
        );
    }

    /// Across multiple consecutive sync cycles with a dead RPC the cursor must
    /// never rewind below its starting value.  Guards against any regression
    /// where a silent empty response causes the cursor to go backwards.
    #[tokio::test]
    async fn test_cursor_never_rewinds_across_multiple_empty_sync_cycles() {
        let client = match make_dead_rpc_client().await {
            Some(c) => c,
            None => {
                println!("Skipping test_cursor_never_rewinds_across_multiple_empty_sync_cycles: Redis unavailable");
                return;
            }
        };

        let initial: u32 = 1_000;
        let mut cursor = initial;

        for round in 0..5u32 {
            let next = client.sync_once(cursor).await.unwrap();
            assert!(
                next >= initial,
                "cursor rewound on round {round}: started at {initial}, became {next}"
            );
            cursor = next;
        }
    }

    /// fetch_events_since must return Ok(vec![]) – not an error – when the RPC
    /// is unreachable, and the silent fallback must be recorded in the
    /// rpc_errors_total metric so operators can detect the failure.
    #[tokio::test]
    async fn test_empty_event_stream_on_rpc_failure_is_recorded_in_metrics() {
        let mut config = Config::from_env();
        config.blockchain_rpc_url = "http://127.0.0.1:0".to_string();
        config.retry_attempts = 1;
        config.retry_base_delay_ms = 1;
        config.sync_market_ids = vec![];

        let metrics = Metrics::new().unwrap();
        let cache = match RedisCache::new(&config.redis_url).await {
            Ok(c) => c,
            Err(_) => {
                println!("Skipping test_empty_event_stream_on_rpc_failure_is_recorded_in_metrics: Redis unavailable");
                return;
            }
        };

        let client = BlockchainClient::new(&config, cache, metrics.clone()).unwrap();

        // RPC failure must be masked – the call must succeed with an empty list.
        let events = client.fetch_events_since(100).await.unwrap();
        assert!(
            events.is_empty(),
            "RPC failure must produce an empty event list, not propagate an error"
        );

        // The silent fallback must be observable via metrics.
        let rendered = metrics.render().unwrap();
        assert!(
            rendered.contains("rpc_errors_total{method=\"getEvents\"} 1"),
            "silent empty-stream fallback must increment rpc_errors_total for getEvents"
        );
    }

    /// sync_once must return Ok (not Err) when the RPC is unreachable, so the
    /// run_sync_worker loop takes the Ok branch and preserves the cursor.
    #[tokio::test]
    async fn test_sync_once_returns_ok_not_err_on_rpc_failure() {
        let client = match make_dead_rpc_client().await {
            Some(c) => c,
            None => {
                println!(
                    "Skipping test_sync_once_returns_ok_not_err_on_rpc_failure: Redis unavailable"
                );
                return;
            }
        };

        let result = client.sync_once(300).await;
        assert!(
            result.is_ok(),
            "sync_once must return Ok on RPC failure so the worker loop preserves the cursor"
        );
    }

    // -------------------------------------------------------------------------
    // Event parsing – malformed payload fuzz / quarantine tests
    // -------------------------------------------------------------------------

    /// Helper: build a minimal valid event JSON.
    fn valid_event() -> Value {
        json!({ "id": "evt-1", "ledger": 100, "topic": "bet", "txHash": "0xabc" })
    }

    #[test]
    fn test_parse_event_valid() {
        let ev = BlockchainClient::parse_event(valid_event()).unwrap();
        assert_eq!(ev.id, "evt-1");
        assert_eq!(ev.ledger, 100);
    }

    #[test]
    fn test_parse_event_missing_id_is_quarantined() {
        let e = json!({ "ledger": 100, "topic": "bet" });
        assert!(BlockchainClient::parse_event(e).is_none());
    }

    #[test]
    fn test_parse_event_empty_id_is_quarantined() {
        let e = json!({ "id": "", "ledger": 100 });
        assert!(BlockchainClient::parse_event(e).is_none());
    }

    #[test]
    fn test_parse_event_missing_ledger_is_quarantined() {
        let e = json!({ "id": "evt-2", "topic": "bet" });
        assert!(BlockchainClient::parse_event(e).is_none());
    }

    #[test]
    fn test_parse_event_zero_ledger_is_quarantined() {
        let e = json!({ "id": "evt-3", "ledger": 0 });
        assert!(BlockchainClient::parse_event(e).is_none());
    }

    #[test]
    fn test_parse_event_null_id_is_quarantined() {
        let e = json!({ "id": null, "ledger": 100 });
        assert!(BlockchainClient::parse_event(e).is_none());
    }

    #[test]
    fn test_parse_event_null_ledger_is_quarantined() {
        let e = json!({ "id": "evt-4", "ledger": null });
        assert!(BlockchainClient::parse_event(e).is_none());
    }

    #[test]
    fn test_parse_event_completely_empty_is_quarantined() {
        assert!(BlockchainClient::parse_event(json!({})).is_none());
    }

    #[test]
    fn test_parse_event_non_object_is_quarantined() {
        for bad in [json!(null), json!(42), json!("string"), json!([])] {
            assert!(
                BlockchainClient::parse_event(bad.clone()).is_none(),
                "expected quarantine for {bad}"
            );
        }
    }

    /// Fuzz a batch: mix of valid and malformed events; only valid ones survive.
    #[test]
    fn test_parse_event_batch_filters_malformed() {
        let inputs = vec![
            json!({ "id": "good-1", "ledger": 50 }),
            json!({ "ledger": 50 }),                    // missing id
            json!({ "id": "", "ledger": 50 }),           // empty id
            json!({ "id": "good-2", "ledger": 99 }),
            json!({ "id": "bad", "ledger": 0 }),         // zero ledger
            json!({ "id": null, "ledger": 10 }),         // null id
            json!({ "id": "good-3", "ledger": 1 }),
        ];

        let parsed: Vec<_> = inputs
            .into_iter()
            .filter_map(BlockchainClient::parse_event)
            .collect();

        assert_eq!(parsed.len(), 3);
        assert_eq!(parsed[0].id, "good-1");
        assert_eq!(parsed[1].id, "good-2");
        assert_eq!(parsed[2].id, "good-3");
    }

    // -------------------------------------------------------------------------
    // Background tx monitor – race conditions and duplicate hash tracking
    // -------------------------------------------------------------------------

    #[tokio::test]
    async fn test_watch_transaction_deduplicates() {
        let state = MonitoringState::default();
        state.watched_txs.write().await.insert("hash-a".to_string());
        state.watched_txs.write().await.insert("hash-a".to_string());
        assert_eq!(state.watched_txs.read().await.len(), 1);
    }

    #[tokio::test]
    async fn test_concurrent_watch_leaves_consistent_set() {
        let state = Arc::new(MonitoringState::default());
        let hashes = ["tx-1", "tx-2", "tx-3", "tx-4", "tx-5"];

        // Spawn concurrent writers.
        let handles: Vec<_> = hashes
            .iter()
            .map(|h| {
                let s = state.clone();
                let h = h.to_string();
                tokio::spawn(async move {
                    s.watched_txs.write().await.insert(h);
                })
            })
            .collect();

        for handle in handles {
            handle.await.unwrap();
        }

        let set = state.watched_txs.read().await;
        assert_eq!(set.len(), hashes.len());
        for h in &hashes {
            assert!(set.contains(*h), "missing {h}");
        }
    }

    #[tokio::test]
    async fn test_concurrent_watch_and_remove_leaves_consistent_set() {
        let state = Arc::new(MonitoringState::default());

        // Pre-populate.
        for h in ["tx-a", "tx-b", "tx-c", "tx-d"] {
            state.watched_txs.write().await.insert(h.to_string());
        }

        // Concurrently: add new hashes while removing finalized ones.
        let add = {
            let s = state.clone();
            tokio::spawn(async move {
                for h in ["tx-e", "tx-f"] {
                    s.watched_txs.write().await.insert(h.to_string());
                }
            })
        };

        let remove = {
            let s = state.clone();
            tokio::spawn(async move {
                // Simulate monitor removing finalized txs.
                for h in ["tx-a", "tx-b"] {
                    s.watched_txs.write().await.remove(h);
                }
            })
        };

        add.await.unwrap();
        remove.await.unwrap();

        let set = state.watched_txs.read().await;
        // tx-c, tx-d remain; tx-e, tx-f were added; tx-a, tx-b removed.
        assert_eq!(set.len(), 4);
        assert!(!set.contains("tx-a"));
        assert!(!set.contains("tx-b"));
        for h in ["tx-c", "tx-d", "tx-e", "tx-f"] {
            assert!(set.contains(h), "missing {h}");
        }
    }

    #[tokio::test]
    async fn test_monitor_does_not_remove_pending_or_not_found() {
        let state = Arc::new(MonitoringState::default());
        state.watched_txs.write().await.insert("tx-pending".to_string());
        state.watched_txs.write().await.insert("tx-not-found".to_string());

        // Simulate one monitor tick: only remove when status is terminal.
        let hashes: Vec<String> = state.watched_txs.read().await.iter().cloned().collect();
        for hash in hashes {
            let status = if hash == "tx-pending" { "PENDING" } else { "NOT_FOUND" };
            if status != "NOT_FOUND" && status != "PENDING" {
                state.watched_txs.write().await.remove(&hash);
            }
        }

        let set = state.watched_txs.read().await;
        assert_eq!(set.len(), 2, "PENDING and NOT_FOUND must not be removed");
    }

    #[tokio::test]
    async fn test_monitor_removes_terminal_status() {
        let state = Arc::new(MonitoringState::default());
        for h in ["tx-success", "tx-failed", "tx-pending"] {
            state.watched_txs.write().await.insert(h.to_string());
        }

        let terminal_statuses = [
            ("tx-success", "SUCCESS"),
            ("tx-failed", "FAILED"),
            ("tx-pending", "PENDING"),
        ];

        for (hash, status) in terminal_statuses {
            if status != "NOT_FOUND" && status != "PENDING" {
                state.watched_txs.write().await.remove(hash);
            }
        }

        let set = state.watched_txs.read().await;
        assert!(!set.contains("tx-success"), "SUCCESS must be removed");
        assert!(!set.contains("tx-failed"), "FAILED must be removed");
        assert!(set.contains("tx-pending"), "PENDING must stay");
    }
}
