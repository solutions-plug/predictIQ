use ipnet::IpNet;
use std::{
    env,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

// ── CORS configuration ────────────────────────────────────────────────────────

/// CORS policy loaded from environment variables.
///
/// | Variable              | Default (production-safe)                          |
/// |-----------------------|----------------------------------------------------|
/// | `CORS_DEV_MODE`       | `false` — set `true` only in local dev             |
/// | `CORS_ALLOWED_ORIGINS`| *(empty)* — must be set explicitly in production   |
/// | `CORS_ALLOWED_METHODS`| `GET,POST,PUT,PATCH,DELETE,OPTIONS`                |
/// | `CORS_ALLOWED_HEADERS`| `content-type,authorization`                       |
/// | `CORS_ALLOW_CREDENTIALS` | `false`                                         |
/// | `CORS_MAX_AGE_SECS`   | `3600`                                             |
///
/// When `CORS_DEV_MODE=true` the layer becomes fully permissive and all other
/// settings are ignored.  This is logged at startup so it is never silent.
///
/// When `CORS_DEV_MODE` is `false` (the default) and `CORS_ALLOWED_ORIGINS` is
/// empty, **no** `Access-Control-Allow-Origin` header is emitted — cross-origin
/// requests will be blocked by browsers.  Set the variable to a
/// comma-separated list of exact origins, e.g.
/// `CORS_ALLOWED_ORIGINS=https://app.predictiq.com,https://staging.predictiq.com`.
#[derive(Clone, Debug)]
pub struct CorsConfig {
    /// When `true` the CORS layer is fully permissive (wildcard origin, all
    /// methods/headers).  Must never be `true` in production.
    pub dev_mode: bool,
    /// Exact origins that are allowed.  Empty means no cross-origin access.
    pub allowed_origins: Vec<String>,
    /// HTTP methods to expose via CORS.
    pub allowed_methods: Vec<String>,
    /// Request headers to expose via CORS.
    pub allowed_headers: Vec<String>,
    /// Whether to allow cookies / credentials.
    pub allow_credentials: bool,
    /// Preflight cache lifetime in seconds (`Access-Control-Max-Age`).
    pub max_age_secs: u64,
}

impl CorsConfig {
    pub fn from_env() -> Self {
        let dev_mode = env::var("CORS_DEV_MODE")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let allowed_origins = env::var("CORS_ALLOWED_ORIGINS")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let allowed_methods = env::var("CORS_ALLOWED_METHODS")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_uppercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| {
                ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

        let allowed_headers = env::var("CORS_ALLOWED_HEADERS")
            .map(|v| {
                v.split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect()
            })
            .unwrap_or_else(|| {
                ["content-type", "authorization"]
                    .iter()
                    .map(|s| s.to_string())
                    .collect()
            });

        let allow_credentials = env::var("CORS_ALLOW_CREDENTIALS")
            .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
            .unwrap_or(false);

        let max_age_secs = env::var("CORS_MAX_AGE_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3600u64);

        Self {
            dev_mode,
            allowed_origins,
            allowed_methods,
            allowed_headers,
            allow_credentials,
            max_age_secs,
        }
    }
}

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

/// PostgreSQL connection pool settings for the API (`sqlx::PgPool`).
///
/// Environment variables are documented in `services/api/DATABASE.md`.
#[derive(Clone, Debug)]
pub struct DbPoolConfig {
    pub min_connections: u32,
    pub max_connections: u32,
    pub acquire_timeout: Duration,
    /// When `None`, sqlx uses its default for idle reaping.
    pub idle_timeout: Option<Duration>,
    /// When `None`, sqlx uses its default connection lifetime.
    pub max_lifetime: Option<Duration>,
    /// Per-query execution timeout. Queries that exceed this are cancelled and
    /// return an error. Configured via `DB_QUERY_TIMEOUT_SECS` (default: 30).
    pub query_timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub redis_url: String,
    pub database_url: String,
    pub hmac_key: String,
    /// Previous HMAC key for zero-downtime key rotation. Optional.
    pub hmac_key_previous: Option<String>,
    /// Grace period in seconds for accepting tokens signed with the previous key.
    /// Default: 3600 (1 hour). Set via `HMAC_KEY_ROTATION_GRACE_SECONDS`.
    pub hmac_key_rotation_grace_seconds: u64,
    pub db_pool: DbPoolConfig,
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
    pub api_keys: Vec<String>,
    pub admin_whitelist_ips: Vec<IpAddr>,
    pub trust_proxy: bool,
    pub request_signing_secret: Option<String>,
    pub sendgrid_webhook_secret: Option<String>,
    /// Webhook replay protection window in seconds. Default: 300 (5 minutes).
    pub webhook_replay_window_secs: u64,
    pub trusted_proxy_cidrs: Vec<IpNet>,
    /// When `true` the `/metrics` endpoint is publicly accessible (no auth).
    /// Defaults to `false`. Set `METRICS_PUBLIC=true` only in trusted environments.
    pub metrics_public: bool,
    /// Optional IP allowlist for the `/metrics` endpoint.
    /// When non-empty, requests must originate from one of these IPs even if
    /// `metrics_public` is `false` (the API key check still applies unless
    /// `metrics_public` is `true`).
    /// Configured via `METRICS_ALLOWLIST_IPS` (comma-separated).
    pub metrics_allowlist_ips: Vec<IpAddr>,
    // Distributed tracing configuration
    pub otlp_endpoint: Option<String>,
    pub trace_sample_rate: f64,
    /// How long (in seconds) an idempotency key is retained in Redis.
    /// Defaults to 86400 (24 hours). Set via `IDEMPOTENCY_WINDOW_SECS`.
    pub idempotency_window_secs: u64,
    /// TTL for newsletter confirmation tokens (seconds). Default: 86400 (24h).
    pub newsletter_token_ttl_secs: u64,
    /// GDPR export rate limit: max requests per window per IP/email. Default: 3.
    pub gdpr_export_rate_limit: u32,
    /// GDPR export rate limit window (seconds). Default: 3600.
    pub gdpr_export_rate_window_secs: u64,
    /// Newsletter subscribe rate limit: max requests per window per IP. Default: 5.
    pub newsletter_rate_limit_max: usize,
    /// Newsletter subscribe rate limit window (seconds). Default: 3600.
    pub newsletter_rate_limit_window_secs: u64,
    /// Email job stale threshold (seconds). Jobs in processing set longer than this
    /// are considered orphaned and will be re-queued on worker startup.
    /// Default: 3600 (1 hour). Set via `EMAIL_STALE_JOB_THRESHOLD_SECS`.
    pub email_stale_job_threshold_secs: u64,
    /// HMAC secret for signing unsubscribe tokens.
    pub unsubscribe_signing_secret: Option<String>,
    /// CORS policy.  See [`CorsConfig`] for per-field documentation.
    pub cors: CorsConfig,
    /// Contract storage key schema.  See [`ContractKeySchema`] for per-field
    /// documentation and startup validation.
    pub contract_key_schema: ContractKeySchema,
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

        let mut db_pool_min = env::var("DB_POOL_MIN_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5u32);
        let mut db_pool_max = env::var("DB_POOL_MAX_CONNECTIONS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(25u32);
        if db_pool_min > db_pool_max {
            std::mem::swap(&mut db_pool_min, &mut db_pool_max);
        }
        // sqlx requires at least one connection in the pool.
        let db_pool_max = db_pool_max.max(1);

        let db_pool_acquire_secs: u64 = env::var("DB_POOL_ACQUIRE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5);
        let db_pool_acquire_timeout = Duration::from_secs(db_pool_acquire_secs.max(1));

        let db_pool_idle_timeout = env::var("DB_POOL_IDLE_TIMEOUT_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|&s| s > 0)
            .map(Duration::from_secs);

        let db_pool_max_lifetime = env::var("DB_POOL_MAX_LIFETIME_SECS")
            .ok()
            .and_then(|s| s.parse::<u64>().ok())
            .filter(|&s| s > 0)
            .map(Duration::from_secs);

        let trusted_proxy_cidrs = env::var("TRUSTED_PROXY_CIDRS")
            .map(|v| v.split(',').filter_map(|s| s.trim().parse().ok()).collect())
            .unwrap_or_else(|_| vec![]);

        Self {
            bind_addr,
            redis_url: env::var("REDIS_URL")
                .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string()),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://postgres:postgres@127.0.0.1/predictiq".to_string()),
            hmac_key: env::var("HMAC_KEY").unwrap_or_default(),
            hmac_key_previous: env::var("HMAC_KEY_PREVIOUS").ok(),
            hmac_key_rotation_grace_seconds: env::var("HMAC_KEY_ROTATION_GRACE_SECONDS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            db_pool: DbPoolConfig {
                min_connections: db_pool_min,
                max_connections: db_pool_max,
                acquire_timeout: db_pool_acquire_timeout,
                idle_timeout: db_pool_idle_timeout,
                max_lifetime: db_pool_max_lifetime,
                query_timeout: Duration::from_secs(
                    env::var("DB_QUERY_TIMEOUT_SECS")
                        .ok()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(30)
                        .max(1),
                ),
            },
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
            base_url: env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()),
            api_keys: env::var("API_KEYS")
                .ok()
                .map(|keys| keys.split(',').map(|k| k.trim().to_string()).collect())
                .unwrap_or_default(),
            admin_whitelist_ips: env::var("ADMIN_WHITELIST_IPS")
                .ok()
                .map(|ips| {
                    ips.split(',')
                        .filter_map(|ip| ip.trim().parse::<IpAddr>().ok())
                        .collect()
                })
                .unwrap_or_default(),
            trust_proxy: env::var("TRUST_PROXY")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(true),
            request_signing_secret: env::var("REQUEST_SIGNING_SECRET").ok(),
            sendgrid_webhook_secret: env::var("SENDGRID_WEBHOOK_SECRET").ok(),
            webhook_replay_window_secs: env::var("WEBHOOK_REPLAY_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(300),
            trusted_proxy_cidrs,
            metrics_public: env::var("METRICS_PUBLIC")
                .ok()
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(false),
            metrics_allowlist_ips: env::var("METRICS_ALLOWLIST_IPS")
                .ok()
                .map(|ips| {
                    ips.split(',')
                        .filter_map(|ip| ip.trim().parse::<IpAddr>().ok())
                        .collect()
                })
                .unwrap_or_default(),
            otlp_endpoint: env::var("OTLP_ENDPOINT").ok(),
            trace_sample_rate: env::var("TRACE_SAMPLE_RATE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.1),
            idempotency_window_secs: env::var("IDEMPOTENCY_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86400),
            newsletter_token_ttl_secs: env::var("NEWSLETTER_TOKEN_TTL_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(86400),
            gdpr_export_rate_limit: env::var("GDPR_EXPORT_RATE_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3),
            gdpr_export_rate_window_secs: env::var("GDPR_EXPORT_RATE_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            newsletter_rate_limit_max: env::var("NEWSLETTER_RATE_LIMIT_MAX")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5),
            newsletter_rate_limit_window_secs: env::var("NEWSLETTER_RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            email_stale_job_threshold_secs: env::var("EMAIL_STALE_JOB_THRESHOLD_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(3600),
            unsubscribe_signing_secret: env::var("UNSUBSCRIBE_SIGNING_SECRET").ok(),
            cors: CorsConfig::from_env(),
            contract_key_schema: ContractKeySchema::from_env(),
        }
    }

    pub fn network_name(&self) -> &'static str {
        match self.blockchain_network {
            BlockchainNetwork::Testnet => "testnet",
            BlockchainNetwork::Mainnet => "mainnet",
            BlockchainNetwork::Custom => "custom",
        }
    }

    /// Validate all required environment variables at startup.
    ///
    /// Checks that:
    /// - DATABASE_URL is set and non-empty, and is a valid PostgreSQL connection string
    /// - REDIS_URL is set and non-empty, and is a valid Redis connection string
    /// - HMAC_KEY is set and non-empty
    ///
    /// Returns `Err` if any required var is missing, empty, or malformed.
    /// On error, prints a clear message to stderr and exits with code 1.
    pub fn validate(&self) -> Result<(), Box<dyn std::error::Error>> {
        let mut errors = Vec::new();

        // Validate DATABASE_URL
        if self.database_url.is_empty() {
            errors.push("DATABASE_URL: environment variable is not set or is empty".to_string());
        } else {
            // Check if it's a valid PostgreSQL connection string (basic validation)
            if !self.database_url.starts_with("postgres://")
                && !self.database_url.starts_with("postgresql://")
            {
                errors.push(format!("DATABASE_URL: invalid format, expected 'postgres://' or 'postgresql://', got '{}'", self.database_url));
            }
        }

        // Validate REDIS_URL
        if self.redis_url.is_empty() {
            errors.push("REDIS_URL: environment variable is not set or is empty".to_string());
        } else {
            // Check if it's a valid Redis connection string (basic validation)
            if !self.redis_url.starts_with("redis://") && !self.redis_url.starts_with("rediss://") {
                errors.push(format!(
                    "REDIS_URL: invalid format, expected 'redis://' or 'rediss://', got '{}'",
                    self.redis_url
                ));
            }
        }

        // Validate HMAC_KEY
        if self.hmac_key.is_empty() {
            errors.push("HMAC_KEY: environment variable is not set or is empty".to_string());
        }

        if !errors.is_empty() {
            for error in &errors {
                eprintln!("Configuration error: {}", error);
            }
            return Err("Required configuration variables are missing or invalid".into());
        }

        Ok(())
    }
}

/// Versioned contract storage key schema.
///
/// Each field is a key template where `{id}` is replaced at call time.
/// Defaults match the v1 deployed schema; override via env vars for per-network
/// divergence (e.g. a testnet that uses a different naming convention).
///
/// Schema version is bumped whenever a key template changes, so callers can
/// detect mismatches at startup.
///
/// | Variable                    | Default              | `{id}` required |
/// |-----------------------------|----------------------|-----------------|
/// | `CONTRACT_KEY_VERSION`      | `"1.0.0"`            | —               |
/// | `CONTRACT_KEY_MARKET`       | `"market:{id}"`      | ✓               |
/// | `CONTRACT_KEY_PLATFORM_STATS` | `"platform:stats"` | —               |
/// | `CONTRACT_KEY_USER_BETS`    | `"user_bets:{id}"`   | ✓               |
/// | `CONTRACT_KEY_ORACLE_RESULT`| `"oracle_result:{id}"` | ✓             |
/// | `CONTRACT_KEY_HEALTH_CHECK` | `"platform:stats"`   | —               |
///
/// Call [`ContractKeySchema::validate`] at startup to detect template drift
/// before any contract reads are attempted.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContractKeySchema {
    /// Semver string, e.g. "1.0.0".  Bump when any template changes.
    pub version: String,
    /// Key for a single market, `{id}` → market_id.
    pub market: String,
    /// Key for platform-wide statistics.
    pub platform_stats: String,
    /// Key for a user's bets, `{id}` → user address.
    pub user_bets: String,
    /// Key for an oracle result, `{id}` → market_id.
    pub oracle_result: String,
    /// Key used by the health-check probe to verify contract reachability.
    /// Defaults to `platform_stats` so no extra storage slot is needed.
    pub health_check: String,
}

/// Error returned by [`ContractKeySchema::validate`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaValidationError {
    /// Human-readable description of every problem found.
    pub errors: Vec<String>,
}

impl std::fmt::Display for SchemaValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "contract key schema validation failed: {}",
            self.errors.join("; ")
        )
    }
}

impl std::error::Error for SchemaValidationError {}

impl ContractKeySchema {
    /// Load from environment, falling back to v1 defaults.
    ///
    /// Override env vars:
    /// - `CONTRACT_KEY_VERSION`
    /// - `CONTRACT_KEY_MARKET`            (default: `"market:{id}"`)
    /// - `CONTRACT_KEY_PLATFORM_STATS`    (default: `"platform:stats"`)
    /// - `CONTRACT_KEY_USER_BETS`         (default: `"user_bets:{id}"`)
    /// - `CONTRACT_KEY_ORACLE_RESULT`     (default: `"oracle_result:{id}"`)
    /// - `CONTRACT_KEY_HEALTH_CHECK`      (default: same as `CONTRACT_KEY_PLATFORM_STATS`)
    pub fn from_env() -> Self {
        let platform_stats = env::var("CONTRACT_KEY_PLATFORM_STATS")
            .unwrap_or_else(|_| "platform:stats".to_string());

        let health_check =
            env::var("CONTRACT_KEY_HEALTH_CHECK").unwrap_or_else(|_| platform_stats.clone());

        Self {
            version: env::var("CONTRACT_KEY_VERSION").unwrap_or_else(|_| "1.0.0".to_string()),
            market: env::var("CONTRACT_KEY_MARKET").unwrap_or_else(|_| "market:{id}".to_string()),
            platform_stats,
            user_bets: env::var("CONTRACT_KEY_USER_BETS")
                .unwrap_or_else(|_| "user_bets:{id}".to_string()),
            oracle_result: env::var("CONTRACT_KEY_ORACLE_RESULT")
                .unwrap_or_else(|_| "oracle_result:{id}".to_string()),
            health_check,
        }
    }

    /// Validate that all templates that require `{id}` actually contain it,
    /// and that no template is empty.
    ///
    /// Returns `Err` with a list of every problem found so all issues are
    /// surfaced in a single startup log line rather than discovered one by one.
    pub fn validate(&self) -> Result<(), SchemaValidationError> {
        let mut errors: Vec<String> = Vec::new();

        // Templates that must contain the `{id}` placeholder.
        let id_required = [
            ("market", &self.market),
            ("user_bets", &self.user_bets),
            ("oracle_result", &self.oracle_result),
        ];
        for (name, template) in &id_required {
            if template.is_empty() {
                errors.push(format!("{name}: template must not be empty"));
            } else if !template.contains("{id}") {
                errors.push(format!(
                    "{name}: template \"{template}\" is missing the {{id}} placeholder"
                ));
            }
        }

        // Templates that must be non-empty but don't need `{id}`.
        let non_empty = [
            ("platform_stats", &self.platform_stats),
            ("health_check", &self.health_check),
        ];
        for (name, template) in &non_empty {
            if template.is_empty() {
                errors.push(format!("{name}: template must not be empty"));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SchemaValidationError { errors })
        }
    }

    // ── Key builders ──────────────────────────────────────────────────────────

    /// Resolve the market key for `market_id`.
    pub fn market_key(&self, market_id: i64) -> String {
        self.market.replace("{id}", &market_id.to_string())
    }

    /// Resolve the user-bets key for `user`.
    pub fn user_bets_key(&self, user: &str) -> String {
        self.user_bets.replace("{id}", user)
    }

    /// Resolve the oracle-result key for `market_id`.
    pub fn oracle_result_key(&self, market_id: i64) -> String {
        self.oracle_result.replace("{id}", &market_id.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validate_all_required_vars_present() {
        let config = Config {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            database_url: "postgres://postgres@localhost/predictiq".to_string(),
            hmac_key: "secret-key-value".to_string(),
            hmac_key_previous: None,
            hmac_key_rotation_grace_seconds: 3600,
            db_pool: DbPoolConfig {
                min_connections: 5,
                max_connections: 25,
                acquire_timeout: Duration::from_secs(5),
                idle_timeout: None,
                max_lifetime: None,
                query_timeout: Duration::from_secs(30),
            },
            blockchain_rpc_url: "https://testnet.soroban.org".to_string(),
            blockchain_network: BlockchainNetwork::Testnet,
            contract_id: "contract_id".to_string(),
            retry_attempts: 3,
            retry_base_delay_ms: 200,
            event_poll_interval: Duration::from_secs(5),
            tx_poll_interval: Duration::from_secs(4),
            confirmation_ledger_lag: 3,
            sync_market_ids: vec![],
            featured_limit: 10,
            content_default_page_size: 20,
            sendgrid_api_key: None,
            from_email: None,
            base_url: "http://localhost:8080".to_string(),
            api_keys: vec![],
            admin_whitelist_ips: vec![],
            trust_proxy: true,
            request_signing_secret: None,
            sendgrid_webhook_secret: None,
            webhook_replay_window_secs: 300,
            trusted_proxy_cidrs: vec![],
            metrics_public: false,
            metrics_allowlist_ips: vec![],
            otlp_endpoint: None,
            trace_sample_rate: 0.1,
            idempotency_window_secs: 86400,
            newsletter_token_ttl_secs: 86400,
            gdpr_export_rate_limit: 3,
            gdpr_export_rate_window_secs: 3600,
            newsletter_rate_limit_max: 5,
            newsletter_rate_limit_window_secs: 3600,
            email_stale_job_threshold_secs: 3600,
            unsubscribe_signing_secret: None,
            cors: CorsConfig {
                dev_mode: false,
                allowed_origins: vec![],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec!["content-type".to_string()],
                allow_credentials: false,
                max_age_secs: 3600,
            },
            contract_key_schema: ContractKeySchema {
                version: "1.0.0".to_string(),
                market: "market:{id}".to_string(),
                platform_stats: "platform:stats".to_string(),
                user_bets: "user_bets:{id}".to_string(),
                oracle_result: "oracle_result:{id}".to_string(),
                health_check: "platform:stats".to_string(),
            },
        };
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_config_validate_missing_hmac_key() {
        let config = Config {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            database_url: "postgres://postgres@localhost/predictiq".to_string(),
            hmac_key: "".to_string(),
            hmac_key_previous: None,
            hmac_key_rotation_grace_seconds: 3600,
            db_pool: DbPoolConfig {
                min_connections: 5,
                max_connections: 25,
                acquire_timeout: Duration::from_secs(5),
                idle_timeout: None,
                max_lifetime: None,
                query_timeout: Duration::from_secs(30),
            },
            blockchain_rpc_url: "https://testnet.soroban.org".to_string(),
            blockchain_network: BlockchainNetwork::Testnet,
            contract_id: "contract_id".to_string(),
            retry_attempts: 3,
            retry_base_delay_ms: 200,
            event_poll_interval: Duration::from_secs(5),
            tx_poll_interval: Duration::from_secs(4),
            confirmation_ledger_lag: 3,
            sync_market_ids: vec![],
            featured_limit: 10,
            content_default_page_size: 20,
            sendgrid_api_key: None,
            from_email: None,
            base_url: "http://localhost:8080".to_string(),
            api_keys: vec![],
            admin_whitelist_ips: vec![],
            trust_proxy: true,
            request_signing_secret: None,
            sendgrid_webhook_secret: None,
            webhook_replay_window_secs: 300,
            trusted_proxy_cidrs: vec![],
            metrics_public: false,
            metrics_allowlist_ips: vec![],
            otlp_endpoint: None,
            trace_sample_rate: 0.1,
            idempotency_window_secs: 86400,
            newsletter_token_ttl_secs: 86400,
            gdpr_export_rate_limit: 3,
            gdpr_export_rate_window_secs: 3600,
            newsletter_rate_limit_max: 5,
            newsletter_rate_limit_window_secs: 3600,
            email_stale_job_threshold_secs: 3600,
            unsubscribe_signing_secret: None,
            cors: CorsConfig {
                dev_mode: false,
                allowed_origins: vec![],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec!["content-type".to_string()],
                allow_credentials: false,
                max_age_secs: 3600,
            },
            contract_key_schema: ContractKeySchema {
                version: "1.0.0".to_string(),
                market: "market:{id}".to_string(),
                platform_stats: "platform:stats".to_string(),
                user_bets: "user_bets:{id}".to_string(),
                oracle_result: "oracle_result:{id}".to_string(),
                health_check: "platform:stats".to_string(),
            },
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_malformed_database_url() {
        let config = Config {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            redis_url: "redis://127.0.0.1:6379".to_string(),
            database_url: "mysql://localhost/predictiq".to_string(),
            hmac_key: "secret".to_string(),
            hmac_key_previous: None,
            hmac_key_rotation_grace_seconds: 3600,
            db_pool: DbPoolConfig {
                min_connections: 5,
                max_connections: 25,
                acquire_timeout: Duration::from_secs(5),
                idle_timeout: None,
                max_lifetime: None,
                query_timeout: Duration::from_secs(30),
            },
            blockchain_rpc_url: "https://testnet.soroban.org".to_string(),
            blockchain_network: BlockchainNetwork::Testnet,
            contract_id: "contract_id".to_string(),
            retry_attempts: 3,
            retry_base_delay_ms: 200,
            event_poll_interval: Duration::from_secs(5),
            tx_poll_interval: Duration::from_secs(4),
            confirmation_ledger_lag: 3,
            sync_market_ids: vec![],
            featured_limit: 10,
            content_default_page_size: 20,
            sendgrid_api_key: None,
            from_email: None,
            base_url: "http://localhost:8080".to_string(),
            api_keys: vec![],
            admin_whitelist_ips: vec![],
            trust_proxy: true,
            request_signing_secret: None,
            sendgrid_webhook_secret: None,
            webhook_replay_window_secs: 300,
            trusted_proxy_cidrs: vec![],
            metrics_public: false,
            metrics_allowlist_ips: vec![],
            otlp_endpoint: None,
            trace_sample_rate: 0.1,
            idempotency_window_secs: 86400,
            newsletter_token_ttl_secs: 86400,
            gdpr_export_rate_limit: 3,
            gdpr_export_rate_window_secs: 3600,
            newsletter_rate_limit_max: 5,
            newsletter_rate_limit_window_secs: 3600,
            email_stale_job_threshold_secs: 3600,
            unsubscribe_signing_secret: None,
            cors: CorsConfig {
                dev_mode: false,
                allowed_origins: vec![],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec!["content-type".to_string()],
                allow_credentials: false,
                max_age_secs: 3600,
            },
            contract_key_schema: ContractKeySchema {
                version: "1.0.0".to_string(),
                market: "market:{id}".to_string(),
                platform_stats: "platform:stats".to_string(),
                user_bets: "user_bets:{id}".to_string(),
                oracle_result: "oracle_result:{id}".to_string(),
                health_check: "platform:stats".to_string(),
            },
        };
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_config_validate_malformed_redis_url() {
        let config = Config {
            bind_addr: "127.0.0.1:8080".parse().unwrap(),
            redis_url: "memcached://127.0.0.1:11211".to_string(),
            database_url: "postgres://postgres@localhost/predictiq".to_string(),
            hmac_key: "secret".to_string(),
            hmac_key_previous: None,
            hmac_key_rotation_grace_seconds: 3600,
            db_pool: DbPoolConfig {
                min_connections: 5,
                max_connections: 25,
                acquire_timeout: Duration::from_secs(5),
                idle_timeout: None,
                max_lifetime: None,
                query_timeout: Duration::from_secs(30),
            },
            blockchain_rpc_url: "https://testnet.soroban.org".to_string(),
            blockchain_network: BlockchainNetwork::Testnet,
            contract_id: "contract_id".to_string(),
            retry_attempts: 3,
            retry_base_delay_ms: 200,
            event_poll_interval: Duration::from_secs(5),
            tx_poll_interval: Duration::from_secs(4),
            confirmation_ledger_lag: 3,
            sync_market_ids: vec![],
            featured_limit: 10,
            content_default_page_size: 20,
            sendgrid_api_key: None,
            from_email: None,
            base_url: "http://localhost:8080".to_string(),
            api_keys: vec![],
            admin_whitelist_ips: vec![],
            trust_proxy: true,
            request_signing_secret: None,
            sendgrid_webhook_secret: None,
            webhook_replay_window_secs: 300,
            trusted_proxy_cidrs: vec![],
            metrics_public: false,
            metrics_allowlist_ips: vec![],
            otlp_endpoint: None,
            trace_sample_rate: 0.1,
            idempotency_window_secs: 86400,
            newsletter_token_ttl_secs: 86400,
            gdpr_export_rate_limit: 3,
            gdpr_export_rate_window_secs: 3600,
            newsletter_rate_limit_max: 5,
            newsletter_rate_limit_window_secs: 3600,
            email_stale_job_threshold_secs: 3600,
            unsubscribe_signing_secret: None,
            cors: CorsConfig {
                dev_mode: false,
                allowed_origins: vec![],
                allowed_methods: vec!["GET".to_string()],
                allowed_headers: vec!["content-type".to_string()],
                allow_credentials: false,
                max_age_secs: 3600,
            },
            contract_key_schema: ContractKeySchema {
                version: "1.0.0".to_string(),
                market: "market:{id}".to_string(),
                platform_stats: "platform:stats".to_string(),
                user_bets: "user_bets:{id}".to_string(),
                oracle_result: "oracle_result:{id}".to_string(),
                health_check: "platform:stats".to_string(),
            },
        };
        assert!(config.validate().is_err());
    }
}
