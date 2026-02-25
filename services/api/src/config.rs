use std::{env, net::SocketAddr, str::FromStr, time::Duration};

#[derive(Clone, Debug)]
pub enum BlockchainNetwork {
    Testnet,
    Mainnet,
    Custom,
}

impl FromStr for BlockchainNetwork {
    type Err = String;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_lowercase().as_str() {
            "testnet" => Ok(Self::Testnet),
            "mainnet" => Ok(Self::Mainnet),
            "custom" => Ok(Self::Custom),
            _ => Err(format!("unsupported BLOCKCHAIN_NETWORK: {value}")),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub redis_url: String,
    pub database_url: String,
    pub blockchain_rpc_url: String,
    pub blockchain_network: BlockchainNetwork,
    pub contract_id: String,
    pub retry_attempts: u32,
    pub retry_base_delay_ms: u64,
    pub event_poll_interval: Duration,
    pub tx_poll_interval: Duration,
    pub confirmation_ledger_lag: u32,
    pub sync_market_ids: Vec<i64>,
    pub featured_limit: i64,
    pub content_default_page_size: i64,
    pub sendgrid_api_key: Option<String>,
    pub from_email: Option<String>,
    pub base_url: String,
    pub recaptcha_secret_key: String,
    pub support_email: String,
}

impl Config {
    pub fn from_env() -> Self {
        let bind_addr = env::var("API_BIND_ADDR")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or_else(|| "0.0.0.0:8080".parse().expect("valid bind addr"));

        let blockchain_network = env::var("BLOCKCHAIN_NETWORK")
            .ok()
            .and_then(|s| BlockchainNetwork::from_str(&s).ok())
            .unwrap_or(BlockchainNetwork::Testnet);

        let blockchain_rpc_url = match env::var("BLOCKCHAIN_RPC_URL") {
            Ok(url) => url,
            Err(_) => match blockchain_network {
                BlockchainNetwork::Testnet => "https://soroban-testnet.stellar.org".to_string(),
                BlockchainNetwork::Mainnet => "https://mainnet.sorobanrpc.com".to_string(),
                BlockchainNetwork::Custom => "http://127.0.0.1:8000".to_string(),
            },
        };

        let sync_market_ids = env::var("SYNC_MARKET_IDS")
            .ok()
            .map(|raw| {
                raw.split(',')
                    .filter_map(|p| p.trim().parse::<i64>().ok())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        Self {
            bind_addr,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1/predictiq".to_string()),
            blockchain_rpc_url,
            blockchain_network,
            contract_id: env::var("PREDICTIQ_CONTRACT_ID")
                .unwrap_or_else(|_| "predictiq_contract".to_string()),
            retry_attempts: env::var("RPC_RETRY_ATTEMPTS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            retry_base_delay_ms: env::var("RPC_RETRY_BASE_DELAY_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(200),
            event_poll_interval: Duration::from_secs(
                env::var("EVENT_POLL_INTERVAL_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(5),
            ),
            tx_poll_interval: Duration::from_secs(
                env::var("TX_POLL_INTERVAL_SECS")
                    .ok()
                    .and_then(|s| s.parse().ok())
                    .unwrap_or(4),
            ),
            confirmation_ledger_lag: env::var("CONFIRMATION_LEDGER_LAG")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            sync_market_ids,
            featured_limit: env::var("FEATURED_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            content_default_page_size: env::var("CONTENT_DEFAULT_PAGE_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(20),
            sendgrid_api_key: env::var("SENDGRID_API_KEY").ok(),
            from_email: env::var("FROM_EMAIL").ok(),
            base_url: env::var("BASE_URL")
                .unwrap_or_else(|_| "http://localhost:8080".to_string()),
            recaptcha_secret_key: env::var("RECAPTCHA_SECRET_KEY")
                .unwrap_or_else(|_| "".to_string()),
            support_email: env::var("SUPPORT_EMAIL")
                .unwrap_or_else(|_| "support@predictiq.com".to_string()),
        }
    }

    pub fn network_name(&self) -> &'static str {
        match self.blockchain_network {
            BlockchainNetwork::Testnet => "testnet",
            BlockchainNetwork::Mainnet => "mainnet",
            BlockchainNetwork::Custom => "custom",
        }
    }
}
