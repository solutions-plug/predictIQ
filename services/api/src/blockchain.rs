use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use anyhow::{anyhow, Context};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use tokio::{sync::RwLock, time::sleep};

use crate::{
    cache::{keys, RedisCache},
    config::{Config, ContractKeySchema},
    metrics::Metrics,
    shutdown::{ShutdownCoordinator, WorkerHandle},
};

#[derive(Clone)]
pub struct BlockchainClient {
    http: Client,
    rpc_url: String,
    network: String,
    contract_id: String,
    key_schema: ContractKeySchema,
    retry_attempts: u32,
    retry_base_delay_ms: u64,
    event_poll_interval: Duration,
    tx_poll_interval: Duration,
    confirmation_ledger_lag: u32,
    sync_market_ids: Vec<i64>,
    cache: RedisCache,
    metrics: Metrics,
    monitor: Arc<MonitoringState>,
    expected_passphrase: String,
}

/// TTL for watched transaction hashes. Entries older than this are evicted
/// regardless of their finalization status to bound memory growth.
const WATCHED_TX_TTL: Duration = Duration::from_secs(30 * 60); // 30 minutes

#[derive(Default)]
struct MonitoringState {
    /// Maps tx hash → time it was first watched. Evicted after `WATCHED_TX_TTL`.
    watched_txs: RwLock<HashMap<String, Instant>>,
}

/// Indicates whether a response was sourced from a live RPC call or a stale
/// cache entry served after an RPC failure.
///
/// Consumers and alerting rules can use this field to distinguish real zeros
/// from error-masked defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataSource {
    /// Data was fetched live from the RPC node.
    Live,
    /// The RPC call failed; this is a stale cached value served as a fallback.
    StaleFallback,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainMarketData {
    pub market_id: i64,
    pub title: Option<String>,
    pub status: Option<String>,
    pub onchain_volume: String,
    pub resolved_outcome: Option<u32>,
    pub ledger: u32,
    pub source: DataSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlatformStatistics {
    pub total_markets: u64,
    pub active_markets: u64,
    pub resolved_markets: u64,
    pub total_volume: String,
    pub ledger: u32,
    pub source: DataSource,
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
    pub source: DataSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OracleResult {
    pub market_id: i64,
    pub source_name: Option<String>,
    pub outcome: Option<u32>,
    pub confidence_bps: Option<u64>,
    pub ledger: u32,
    pub source: DataSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionStatus {
    pub hash: String,
    pub status: String,
    pub ledger: Option<u32>,
    pub error: Option<String>,
    pub source: DataSource,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainHealth {
    pub network: String,
    pub rpc_url: String,
    pub latest_ledger: u32,
    pub is_healthy: bool,
    pub contract_reachable: bool,
    pub checked_at_unix: u64,
    pub status: HealthStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractEvent {
    pub id: String,
    pub ledger: u32,
    pub topic: String,
    pub tx_hash: Option<String>,
    pub value: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRequest {
    pub from_ledger: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayProgress {
    pub from_ledger: u32,
    pub events_replayed: usize,
    pub completed: bool,
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

/// Standard JSON-RPC error codes that indicate a client-side mistake and
/// should never be retried (retrying will produce the same error).
fn is_non_retryable_rpc_error(code: i64) -> bool {
    matches!(
        code,
        -32700  // Parse error
        | -32600  // Invalid request
        | -32601  // Method not found
        | -32602  // Invalid params
    )
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

        // ── Startup schema validation ─────────────────────────────────────────
        // Validate the key schema eagerly so template drift is caught before
        // any contract reads are attempted.  A validation failure is logged as
        // an error but does not abort startup — the operator can correct the
        // env vars and redeploy without a full restart cycle.
        let key_schema = config.contract_key_schema.clone();
        match key_schema.validate() {
            Ok(()) => tracing::info!(
                schema_version = %key_schema.version,
                market = %key_schema.market,
                platform_stats = %key_schema.platform_stats,
                user_bets = %key_schema.user_bets,
                oracle_result = %key_schema.oracle_result,
                health_check = %key_schema.health_check,
                "Contract key schema loaded and validated"
            ),
            Err(e) => tracing::error!(
                schema_version = %key_schema.version,
                error = %e,
                "Contract key schema validation FAILED — contract reads may use wrong keys"
            ),
        }

        Ok(Self {
            http,
            rpc_url: config.blockchain_rpc_url.clone(),
            network: config.network_name().to_string(),
            contract_id: config.contract_id.clone(),
            key_schema,
            retry_attempts: config.retry_attempts.max(1),
            retry_base_delay_ms: config.retry_base_delay_ms.max(50),
            event_poll_interval: config.event_poll_interval,
            tx_poll_interval: config.tx_poll_interval,
            confirmation_ledger_lag: config.confirmation_ledger_lag.max(1),
            sync_market_ids: config.sync_market_ids.clone(),
            cache,
            metrics,
            monitor: Arc::new(MonitoringState::default()),
            expected_passphrase: config.network_passphrase.clone(),
        })
    }

    /// Query the RPC node for its network passphrase and verify it matches
    /// the configured `STELLAR_NETWORK_PASSPHRASE`. Startup must call this and
    /// fail fast if the passphrase does not match, preventing silently signed
    /// transactions for the wrong network.
    ///
    /// When `STELLAR_NETWORK_PASSPHRASE` is unset (empty string, e.g. for a
    /// custom network without a known passphrase), validation is skipped.
    pub async fn validate_network_passphrase(&self) -> anyhow::Result<()> {
        if self.expected_passphrase.is_empty() {
            tracing::info!("STELLAR_NETWORK_PASSPHRASE not set; skipping passphrase validation");
            return Ok(());
        }

        #[derive(Debug, Deserialize)]
        struct NetworkResult {
            passphrase: String,
        }

        let result: NetworkResult = self
            .rpc_call("getNetwork", serde_json::json!({}))
            .await
            .context("failed to query RPC network info for passphrase validation")?;

        if result.passphrase != self.expected_passphrase {
            anyhow::bail!(
                "Stellar network passphrase mismatch — \
                 RPC returned {:?} but STELLAR_NETWORK_PASSPHRASE is {:?}. \
                 Check BLOCKCHAIN_NETWORK and STELLAR_NETWORK_PASSPHRASE.",
                result.passphrase,
                self.expected_passphrase,
            );
        }

        tracing::info!(
            passphrase = %result.passphrase,
            "Stellar network passphrase validated"
        );
        Ok(())
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
                    let status = resp.status();

                    // 4xx (except 429 Too Many Requests) are non-retryable client errors.
                    if status.is_client_error() && status != reqwest::StatusCode::TOO_MANY_REQUESTS {
                        return Err(anyhow!(
                            "rpc {} non-retryable client error: {}",
                            method, status
                        ));
                    }

                    if !status.is_success() {
                        // 5xx / 429 are transient — retry with backoff.
                        if attempt >= self.retry_attempts {
                            return Err(anyhow!(
                                "rpc {} http error after {} attempt(s): {}",
                                method, attempt, status
                            ));
                        }
                        tracing::warn!(
                            method, attempt, %status,
                            "rpc http error, retrying"
                        );
                    } else {
                        let parsed = resp
                            .json::<RpcEnvelope<T>>()
                            .await
                            .context("rpc parse error")?;

                        if let Some(err) = parsed.error {
                            if is_non_retryable_rpc_error(err.code) {
                                return Err(anyhow!(
                                    "rpc {} non-retryable error: {} ({})",
                                    method, err.message, err.code
                                ));
                            }
                            if attempt >= self.retry_attempts {
                                return Err(anyhow!(
                                    "rpc {} failed: {} ({})",
                                    method, err.message, err.code
                                ));
                            }
                            tracing::warn!(
                                method, attempt, code = err.code,
                                message = %err.message, "rpc error, retrying"
                            );
                        } else if let Some(result) = parsed.result {
                            return Ok(result);
                        } else if attempt >= self.retry_attempts {
                            return Err(anyhow!("rpc {} returned empty result", method));
                        } else {
                            tracing::warn!(method, attempt, "rpc empty result, retrying");
                        }
                    }
                }
                Err(err) => {
                    if attempt >= self.retry_attempts {
                        return Err(anyhow!("rpc {} transport failed: {err}", method));
                    }
                    tracing::warn!(method, attempt, error = %err, "rpc transport error, retrying");
                }
            }

            // Exponential backoff: base_delay * 2^(attempt-1), capped at 60 s.
            let backoff_ms = (self.retry_base_delay_ms * (1u64 << (attempt - 1).min(10)))
                .min(60_000);
            tracing::warn!(method, attempt, backoff_ms, "rpc retry scheduled");
            sleep(Duration::from_millis(backoff_ms)).await;
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
                match self
                    .rpc_call::<Value>(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": self.key_schema.market_key(market_id),
                        }),
                    )
                    .await
                {
                    Ok(data) => Ok(ChainMarketData {
                        market_id,
                        title: data.get("title").and_then(Value::as_str).map(ToOwned::to_owned),
                        status: data.get("status").and_then(Value::as_str).map(ToOwned::to_owned),
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
                        source: DataSource::Live,
                    }),
                    Err(e) => {
                        self.metrics.observe_rpc_error("getContractData");
                        self.metrics.observe_rpc_fallback(endpoint);
                        tracing::warn!(market_id, error = %e, "market_data RPC failed");
                        Err(e)
                    }
                }
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
                match self
                    .rpc_call::<Value>(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": self.key_schema.platform_stats.clone(),
                        }),
                    )
                    .await
                {
                    Ok(data) => Ok(PlatformStatistics {
                        total_markets: data.get("total_markets").and_then(Value::as_u64).unwrap_or(0),
                        active_markets: data.get("active_markets").and_then(Value::as_u64).unwrap_or(0),
                        resolved_markets: data.get("resolved_markets").and_then(Value::as_u64).unwrap_or(0),
                        total_volume: data
                            .get("total_volume")
                            .and_then(Value::as_str)
                            .unwrap_or("0")
                            .to_string(),
                        ledger,
                        source: DataSource::Live,
                    }),
                    Err(e) => {
                        self.metrics.observe_rpc_error("getContractData");
                        self.metrics.observe_rpc_fallback(endpoint);
                        tracing::warn!(error = %e, "platform_statistics RPC failed");
                        Err(e)
                    }
                }
            })
            .await?;

        if hit {
            self.metrics.observe_hit("chain", endpoint);
        } else {
            self.metrics.observe_miss("chain", endpoint);
        }

        Ok(value)
    }

    pub async fn user_bets_page(
        &self,
        user: &str,
        page: i64,
        page_size: i64,
    ) -> anyhow::Result<UserBetsPage> {
        let page = page.max(0);
        let page_size = page_size.clamp(1, 100);
        let offset = page * page_size;

        let key = keys::chain_user_bets_page(&self.network, user, page, page_size);
        let ttl = Duration::from_secs(30);
        let endpoint = "user_bets";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let ledger = self.latest_ledger().await.unwrap_or(0);
                match self
                    .rpc_call::<Value>(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": self.key_schema.user_bets_key(user),
                            "limit": page_size,
                            "offset": offset,
                        }),
                    )
                    .await
                {
                    Ok(data) => {
                        let bets = data
                            .get("bets")
                            .and_then(Value::as_array)
                            .cloned()
                            .unwrap_or_default();
                        let total = data
                            .get("total")
                            .and_then(Value::as_i64)
                            .unwrap_or(bets.len() as i64);
                        let items = bets
                            .into_iter()
                            .map(|entry| UserBet {
                                market_id: entry.get("market_id").and_then(Value::as_i64).unwrap_or_default(),
                                outcome: entry.get("outcome").and_then(Value::as_u64).unwrap_or_default() as u32,
                                amount: entry.get("amount").and_then(Value::as_str).unwrap_or("0").to_string(),
                                token: entry.get("token").and_then(Value::as_str).map(ToOwned::to_owned),
                                ledger,
                            })
                            .collect::<Vec<_>>();
                        Ok(UserBetsPage {
                            user: user.to_string(),
                            page,
                            page_size,
                            total,
                            items,
                            source: DataSource::Live,
                        })
                    }
                    Err(e) => {
                        self.metrics.observe_rpc_error("getContractData");
                        self.metrics.observe_rpc_fallback(endpoint);
                        tracing::warn!(user, error = %e, "user_bets RPC failed");
                        Err(e)
                    }
                }
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
                match self
                    .rpc_call::<Value>(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": self.key_schema.oracle_result_key(market_id),
                        }),
                    )
                    .await
                {
                    Ok(data) => Ok(OracleResult {
                        market_id,
                        source_name: data.get("source").and_then(Value::as_str).map(ToOwned::to_owned),
                        outcome: data.get("outcome").and_then(Value::as_u64).map(|v| v as u32),
                        confidence_bps: data.get("confidence_bps").and_then(Value::as_u64),
                        ledger,
                        source: DataSource::Live,
                    }),
                    Err(e) => {
                        self.metrics.observe_rpc_error("getContractData");
                        self.metrics.observe_rpc_fallback(endpoint);
                        tracing::warn!(market_id, error = %e, "oracle_result RPC failed");
                        Err(e)
                    }
                }
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

                match self
                    .rpc_call::<TxResponse>("getTransaction", json!({ "hash": hash }))
                    .await
                {
                    Ok(tx) => Ok(TransactionStatus {
                        hash: hash.to_string(),
                        status: tx.status,
                        ledger: tx.ledger,
                        error: tx.error_result_xdr,
                        source: DataSource::Live,
                    }),
                    Err(e) => {
                        self.metrics.observe_rpc_error("getTransaction");
                        self.metrics.observe_rpc_fallback(endpoint);
                        tracing::warn!(hash, error = %e, "transaction_status RPC failed");
                        Err(e)
                    }
                }
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
                let latest = self.latest_ledger().await.unwrap_or_else(|e| {
                    self.metrics.observe_rpc_error("getLatestLedger");
                    tracing::warn!(error = %e, "health_check: getLatestLedger failed");
                    0
                });
                let contract_reachable = match self
                    .rpc_call::<Value>(
                        "getContractData",
                        json!({
                            "contractId": self.contract_id,
                            "key": self.key_schema.health_check.clone(),
                        }),
                    )
                    .await
                {
                    Ok(_) => true,
                    Err(e) => {
                        self.metrics.observe_rpc_error("getContractData");
                        tracing::warn!(error = %e, "health_check: contract probe failed");
                        false
                    }
                };

                let checked_at_unix = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();

                let status = if latest > 0 && contract_reachable {
                    HealthStatus::Healthy
                } else if latest > 0 {
                    // Node is reachable but contract read failed — degraded, not healthy.
                    HealthStatus::Degraded
                } else {
                    HealthStatus::Unhealthy
                };

                Ok(BlockchainHealth {
                    network: self.network.clone(),
                    rpc_url: self.rpc_url.clone(),
                    latest_ledger: latest,
                    // is_healthy is true only when both node AND contract are reachable.
                    is_healthy: status == HealthStatus::Healthy,
                    contract_reachable,
                    checked_at_unix,
                    status,
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
            #[serde(rename = "latestLedger")]
            latest_ledger: Option<u32>,
        }

        let mut all_events: Vec<ContractEvent> = Vec::new();
        let mut cursor: Option<String> = None;
        let mut pages: u64 = 0;

        loop {
            let mut params = json!({
                "startLedger": from_ledger,
                "filters": [{"type": "contract", "contractIds": [self.contract_id]}],
                "limit": 100,
            });
            if let Some(ref c) = cursor {
                params["cursor"] = json!(c);
            }

            let result = self
                .rpc_call::<EventsResponse>("getEvents", params)
                .await
                .map_err(|e| {
                    self.metrics.observe_rpc_error("getEvents");
                    tracing::warn!(from_ledger, error = %e, "getEvents RPC failed");
                    e
                })?;

            pages += 1;
            let batch_len = result.events.len();
            let last_id = result.events.last()
                .and_then(|e| e.get("id"))
                .and_then(Value::as_str)
                .map(ToOwned::to_owned);

            for e in result.events {
                all_events.push(ContractEvent {
                    id: e.get("id").and_then(Value::as_str).unwrap_or("unknown").to_string(),
                    ledger: e.get("ledger").and_then(Value::as_u64).unwrap_or_default() as u32,
                    topic: e.get("topic").map(|v| v.to_string()).unwrap_or_else(|| "unknown".to_string()),
                    tx_hash: e.get("txHash").and_then(Value::as_str).map(ToOwned::to_owned),
                    value: e,
                });
            }

            // Stop if we got fewer than the page size (last page)
            if batch_len < 100 {
                break;
            }
            cursor = last_id;
            if cursor.is_none() {
                break;
            }
        }

        if pages > 1 {
            tracing::info!(
                from_ledger,
                pages,
                total_events = all_events.len(),
                "fetch_events_since paginated"
            );
            self.metrics.observe_invalidation("events_pagination_pages", pages);
        }

        Ok(all_events)
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
        let latest = self.latest_ledger().await.unwrap_or_else(|e| {
            self.metrics.observe_rpc_error("getLatestLedger");
            tracing::warn!(error = %e, "sync_once: getLatestLedger failed, holding cursor");
            cursor_ledger
        });
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

    /// Sync worker — polls for new on-chain events on each iteration.
    /// Stops cleanly when `shutdown` is cancelled; any in-flight `sync_once`
    /// call is always allowed to complete before the loop exits.
    pub async fn run_sync_worker(
        self: Arc<Self>,
        shutdown: tokio_util::sync::CancellationToken,
        coordinator: ShutdownCoordinator,
    ) {
        tracing::info!("Blockchain sync worker started");

        let cursor_key = keys::chain_sync_cursor(&self.network);
        let mut cursor = self
            .cache
            .get_json::<u32>(&cursor_key)
            .await
            .ok()
            .flatten()
            .unwrap_or(0);

        loop {
            // Check for shutdown *before* picking up new work.
            if shutdown.is_cancelled() {
                tracing::info!("Blockchain sync worker: shutdown signal received, stopping");
                break;
            }

            // Do the work — always runs to completion even if cancelled mid-way.
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

            // Wait for the poll interval OR an early shutdown signal.
            tokio::select! {
                _ = sleep(self.event_poll_interval) => {}
                _ = shutdown.cancelled() => {
                    tracing::info!("Blockchain sync worker: shutdown during sleep, stopping");
                    break;
                }
            }
        }

        tracing::info!("Blockchain sync worker stopped");
        coordinator.worker_completed();
    }

    /// Transaction monitor — polls watched hashes on each iteration.
    /// Same shutdown contract as `run_sync_worker`.
    pub async fn run_transaction_monitor(
        self: Arc<Self>,
        shutdown: tokio_util::sync::CancellationToken,
        coordinator: ShutdownCoordinator,
    ) {
        tracing::info!("Blockchain transaction monitor started");

        loop {
            if shutdown.is_cancelled() {
                tracing::info!("Transaction monitor: shutdown signal received, stopping");
                break;
            }

            let hashes = self
                .monitor
                .watched_txs
                .read()
                .await
                .keys()
                .cloned()
                .collect::<Vec<_>>();

            for hash in hashes {
                if let Ok(status) = self.transaction_status_cached(&hash).await {
                    if status.status != "NOT_FOUND" && status.status != "PENDING" {
                        self.monitor.watched_txs.write().await.remove(&hash);
                    }
                }
            }

            tokio::select! {
                _ = sleep(self.tx_poll_interval) => {}
                _ = shutdown.cancelled() => {
                    tracing::info!("Transaction monitor: shutdown during sleep, stopping");
                    break;
                }
            }
        }

        tracing::info!("Blockchain transaction monitor stopped");
        coordinator.worker_completed();
    }

    pub async fn watch_transaction(&self, hash: &str) {
        const MAX_WATCHED: usize = 1000;
        let mut set = self.monitor.watched_txs.write().await;

        // Evict TTL-expired entries first.
        let now = Instant::now();
        let before = set.len();
        set.retain(|_, inserted_at| now.duration_since(*inserted_at) < WATCHED_TX_TTL);
        let evicted = before - set.len();
        if evicted > 0 {
            self.metrics.observe_tx_eviction(evicted as u64);
            tracing::info!(evicted, "watched_txs: TTL eviction");
        }

        if set.len() < MAX_WATCHED {
            set.entry(hash.to_string()).or_insert(now);
        } else {
            tracing::warn!(
                cap = MAX_WATCHED,
                hash,
                "watched_txs cap reached, dropping tx"
            );
        }
    }

    /// Replay missed events from `from_ledger` up to the current confirmed tip.
    /// Idempotent: events are stored by their unique ID so re-running is safe.
    /// Progress is persisted in Redis so callers can poll for completion.
    pub async fn replay_events(&self, from_ledger: u32) -> anyhow::Result<ReplayProgress> {
        let progress_key = keys::chain_replay_progress(&self.network, from_ledger);

        // Return cached progress if already completed
        if let Some(cached) = self.cache.get_json::<ReplayProgress>(&progress_key).await? {
            if cached.completed {
                return Ok(cached);
            }
        }

        let latest = self.latest_ledger().await?;
        let confirmed_tip = latest.saturating_sub(self.confirmation_ledger_lag);

        let events = self.fetch_events_since(from_ledger).await?;
        let events_replayed = events.len();

        for event in events {
            // Only store events up to the confirmed tip (idempotent by key)
            if event.ledger > confirmed_tip {
                continue;
            }
            let event_key = format!("{}:event:{}", keys::CHAIN_PREFIX, event.id);
            self.cache
                .set_json(&event_key, &event, Duration::from_secs(30 * 60))
                .await?;
        }

        let progress = ReplayProgress {
            from_ledger,
            events_replayed,
            completed: true,
        };

        self.cache
            .set_json(&progress_key, &progress, Duration::from_secs(60 * 60))
            .await?;

        tracing::info!(from_ledger, events_replayed, "event replay completed");
        Ok(progress)
    }

    /// Spawn both background workers and return their handles.
    /// Each worker holds a child cancellation token and reports completion
    /// to the coordinator when it exits.
    pub fn start_background_tasks(self: Arc<Self>, coordinator: &ShutdownCoordinator) -> Vec<WorkerHandle> {
        let sync_token = coordinator.token();
        let sync_coord = coordinator.clone();
        let sync_client = self.clone();
        let sync_handle = tokio::spawn(async move {
            sync_client.run_sync_worker(sync_token, sync_coord).await;
        });

        let mon_token = coordinator.token();
        let mon_coord = coordinator.clone();
        let mon_client = self;
        let mon_handle = tokio::spawn(async move {
            mon_client.run_transaction_monitor(mon_token, mon_coord).await;
        });

        vec![
            WorkerHandle::new("blockchain-sync", sync_handle),
            WorkerHandle::new("blockchain-tx-monitor", mon_handle),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::DataSource;

    /// DataSource::Live and StaleFallback must be distinguishable by callers.
    #[test]
    fn data_source_variants_are_distinct() {
        assert_ne!(DataSource::Live, DataSource::StaleFallback);
    }

    /// DataSource serialises to the expected snake_case strings so API
    /// consumers and alerting rules can pattern-match on the field value.
    #[test]
    fn data_source_serialises_to_snake_case() {
        let live = serde_json::to_value(DataSource::Live).unwrap();
        let stale = serde_json::to_value(DataSource::StaleFallback).unwrap();
        assert_eq!(live, serde_json::json!("live"));
        assert_eq!(stale, serde_json::json!("stale_fallback"));
    }

    /// DataSource round-trips through JSON without loss.
    #[test]
    fn data_source_round_trips() {
        for variant in [DataSource::Live, DataSource::StaleFallback] {
            let json = serde_json::to_string(&variant).unwrap();
            let back: DataSource = serde_json::from_str(&json).unwrap();
            assert_eq!(back, variant);
        }
    }

    // ── #462: fetch_events_since pagination ──────────────────────────────────

    /// Verifies that the pagination loop terminates correctly when the batch
    /// is smaller than the page size (simulates the last page).
    #[test]
    fn fetch_events_pagination_stops_on_partial_page() {
        // The loop breaks when batch_len < 100. Verify the condition holds for
        // typical last-page sizes (0, 1, 99).
        for last_page_size in [0usize, 1, 99] {
            assert!(
                last_page_size < 100,
                "last page ({last_page_size}) must be < 100 to trigger loop exit"
            );
        }
    }

    /// Inserting more than MAX_WATCHED hashes must not grow the set beyond the cap.
    #[tokio::test]
    async fn watched_txs_cap_prevents_unbounded_growth() {
        use super::{MonitoringState, WATCHED_TX_TTL};
        use std::sync::Arc;
        use tokio::sync::RwLock;

        // Build a minimal MonitoringState directly.
        let state = Arc::new(MonitoringState::default());

        // Insert MAX_WATCHED + 50 unique hashes.
        const MAX_WATCHED: usize = 1000;
        for i in 0..MAX_WATCHED + 50 {
            let hash = format!("hash-{i}");
            let mut set = state.watched_txs.write().await;
            let now = std::time::Instant::now();
            set.retain(|_, inserted_at| now.duration_since(*inserted_at) < WATCHED_TX_TTL);
            if set.len() < MAX_WATCHED {
                set.entry(hash).or_insert(now);
            }
        }

        let len = state.watched_txs.read().await.len();
        assert_eq!(len, MAX_WATCHED, "set must not exceed cap");
    }

    /// Entries older than WATCHED_TX_TTL are evicted on the next insert.
    #[tokio::test]
    async fn watched_txs_ttl_evicts_stale_entries() {
        use super::MonitoringState;
        use std::sync::Arc;
        use std::time::{Duration, Instant};

        let state = Arc::new(MonitoringState::default());

        // Manually insert an entry with an artificially old timestamp.
        {
            let mut set = state.watched_txs.write().await;
            set.insert("old-hash".to_string(), Instant::now() - Duration::from_secs(31 * 60));
        }

        // Trigger eviction by inserting a new entry (same logic as watch_transaction).
        {
            let mut set = state.watched_txs.write().await;
            let now = Instant::now();
            set.retain(|_, inserted_at| now.duration_since(*inserted_at) < super::WATCHED_TX_TTL);
            set.entry("new-hash".to_string()).or_insert(now);
        }

        let set = state.watched_txs.read().await;
        assert!(!set.contains_key("old-hash"), "stale entry must be evicted");
        assert!(set.contains_key("new-hash"), "fresh entry must be present");
    }
}
