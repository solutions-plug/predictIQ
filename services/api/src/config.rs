use chrono;
use base64::Engine as _;
use ipnet::IpNet;
use secrecy::{ExposeSecret, SecretString};
use std::{
    collections::HashMap,
    env,
    net::{IpAddr, SocketAddr},
    str::FromStr,
    time::Duration,
};

// ── Database credentials ──────────────────────────────────────────────────────

/// Individual database connection components loaded from environment variables.
///
/// The password is wrapped in [`SecretString`] so it is never printed in logs
/// or `Debug` output. The full connection URL is assembled at the last possible
/// moment (when the pool is created) via [`DbCredentials::to_connection_string`]
/// and should never be stored long-term.
///
/// | Variable    | Example                 | Notes                          |
/// |-------------|-------------------------|--------------------------------|
/// | `DB_HOST`   | `rds-prod.example.com`  | Required                       |
/// | `DB_PORT`   | `5432`                  | Default: `5432`                |
/// | `DB_NAME`   | `predictiq`             | Required                       |
/// | `DB_USER`   | `predictiq_api`         | Required                       |
/// | `DB_PASSWORD` | *secret*              | Required; loaded from Secrets Manager |
#[derive(Clone)]
pub struct DbCredentials {
    pub host: String,
    pub port: u16,
    pub name: String,
    pub user: String,
    /// Password is wrapped in `SecretString` — it is never written to logs.
    pub password: SecretString,
}

impl std::fmt::Debug for DbCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DbCredentials")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("name", &self.name)
            .field("user", &self.user)
            .field("password", &"[redacted]")
            .finish()
    }
}

impl DbCredentials {
    /// Assemble the `postgres://` connection string from the individual
    /// components. This is the **only** place the password is exposed; callers
    /// should not store the returned string.
    pub fn to_connection_string(&self) -> SecretString {
        SecretString::new(
            format!(
                "postgres://{}:{}@{}:{}/{}",
                self.user,
                self.password.expose_secret(),
                self.host,
                self.port,
                self.name,
            )
            .into(),
        )
    }

    pub fn from_env() -> Self {
        Self {
            host: env::var("DB_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("DB_PORT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(5432),
            name: env::var("DB_NAME").unwrap_or_else(|_| "predictiq".to_string()),
            user: env::var("DB_USER").unwrap_or_else(|_| "postgres".to_string()),
            password: SecretString::new(
                env::var("DB_PASSWORD")
                    .unwrap_or_else(|_| "postgres".to_string())
                    .into(),
            ),
        }
    }
}

fn hmac_key_entropy(data: &[u8]) -> f64 {
    if data.is_empty() {
        return 0.0;
    }
    let mut counts = [0u32; 256];
    for &b in data {
        counts[b as usize] += 1;
    }
    let len = data.len() as f64;
    counts.iter()
        .filter(|&&c| c > 0)
        .map(|&c| { let p = c as f64 / len; -p * p.log2() })
        .sum()
}

/// Known `APP_ENV` values. Startup fails for anything else so a typo (or a
/// mismatched variable name in the deployment) cannot silently disable
/// production safety checks.
pub fn validate_app_env(value: &str) -> Result<(), String> {
    const KNOWN: [&str; 3] = ["development", "staging", "production"];
    if KNOWN.iter().any(|k| k.eq_ignore_ascii_case(value)) {
        Ok(())
    } else {
        Err(format!(
            "APP_ENV={value:?} is not a known environment (expected one of {KNOWN:?})"
        ))
    }
}

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
            .unwrap_or_else(|_| {
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
            .unwrap_or_else(|_| {
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
    /// PostgreSQL `statement_timeout` set on every connection via `after_connect`.
    /// Prevents runaway queries from holding connections indefinitely.
    /// Configured via `DB_STATEMENT_TIMEOUT_MS` (default: 30000 ms).
    pub statement_timeout_ms: u64,
    /// PostgreSQL `lock_timeout` set on every connection via `after_connect`.
    /// Prevents lock-contention hangs. Configured via `DB_LOCK_TIMEOUT_MS` (default: 10000 ms).
    pub lock_timeout_ms: u64,
}

/// Default maximum request body size: 1 MiB.
///
/// Used by the body-size middleware in `compression.rs`.  The value is resolved
/// once at startup (see [`Config::request_body_max_bytes`]) rather than being
/// read from the environment on every request.
pub const DEFAULT_REQUEST_BODY_MAX_BYTES: usize = 1024 * 1024;

/// Parse the `REQUEST_BODY_MAX_BYTES` environment variable.
///
/// Returns `DEFAULT_REQUEST_BODY_MAX_BYTES` when the variable is unset or
/// cannot be parsed as a positive integer.  A value of `0` is rejected (it
/// would block every request with a body) and falls back to the default.
pub fn parse_request_body_max_bytes(raw: Option<&str>) -> usize {
    raw.and_then(|s| s.trim().parse::<usize>().ok())
        .filter(|&n| n > 0)
        .unwrap_or(DEFAULT_REQUEST_BODY_MAX_BYTES)
}

#[derive(Clone)]
pub struct Config {
    pub bind_addr: SocketAddr,
    pub redis_url: String,
    /// Individual database connection components. The password is stored as a
    /// [`SecretString`] and the full connection URL is assembled at the last
    /// moment in [`Database::new`] — it is never stored as a plain string or
    /// written to logs.
    pub db_credentials: DbCredentials,
    /// Legacy field kept for callers that still need the assembled URL (e.g.
    /// sqlx migration runner). Assembled lazily from `db_credentials`.
    /// Do **not** log this value.
    #[deprecated(since = "0.1.0", note = "use db_credentials.to_connection_string() instead")]
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
    /// Jitter factor for RPC retry backoff. 1.0 = full jitter (default), 0.0 = no jitter.
    /// Configured via `RPC_BACKOFF_JITTER_FACTOR`.
    pub rpc_backoff_jitter_factor: f64,
    pub event_poll_interval: Duration,
    pub tx_poll_interval: Duration,
    pub confirmation_ledger_lag: u32,
    pub sync_market_ids: Vec<i64>,
    pub featured_limit: i64,
    pub content_default_page_size: i64,
    pub sendgrid_api_key: Option<String>,
    pub from_email: Option<String>,
    /// ISO-8601 date (YYYY-MM-DD) recording when SENDGRID_API_KEY was last
    /// rotated. Injected from Secrets Manager via `SENDGRID_KEY_ROTATED_AT`.
    /// When set and the key is older than 90 days the server logs a warning at
    /// startup so on-call engineers are notified before the key is revoked.
    pub sendgrid_key_rotated_at: Option<String>,
    pub base_url: String,
    /// Server-side secret for HMAC-SHA256 email idempotency keys.
    /// Configured via `EMAIL_IDEMPOTENCY_SECRET`. Falls back to `hmac_key` if unset.
    pub email_idempotency_secret: String,
    pub api_keys: Vec<String>,
    pub admin_whitelist_ips: Vec<IpAddr>,
    pub trust_proxy: bool,
    pub request_signing_secret: Option<String>,
    /// SendGrid Event Webhook signing key.
    ///
    /// SendGrid signs event payloads with ECDSA (NIST P-256 / SHA-256), not
    /// HMAC. This value is the **base64-encoded DER SubjectPublicKeyInfo**
    /// public key shown in the SendGrid dashboard under *Settings → Sender
    /// Authentication → Signed Event Webhook*. It is loaded from the
    /// `SENDGRID_WEBHOOK_SECRET` environment variable (injected from Secrets
    /// Manager) and must never be hard-coded.
    ///
    /// See `docs/webhook-security.md` for the verification procedure.
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
    /// Max rows deleted per newsletter cleanup run. Default: 500.
    /// Keep this low enough that the DELETE does not hold a table lock long
    /// enough to noticeably delay concurrent subscriber inserts.
    /// Set via `NEWSLETTER_CLEANUP_BATCH_SIZE`.
    pub newsletter_cleanup_batch_size: u64,
    /// Per-recipient hourly email send limit. Default: 10.
    /// Set via `EMAIL_PER_RECIPIENT_HOURLY_LIMIT`.
    pub email_per_recipient_hourly_limit: u64,
    /// HMAC secret for signing unsubscribe tokens.
    pub unsubscribe_signing_secret: Option<String>,
    /// CORS policy.  See [`CorsConfig`] for per-field documentation.
    pub cors: CorsConfig,
    /// Contract storage key schema.  See [`ContractKeySchema`] for per-field
    /// documentation and startup validation.
    pub contract_key_schema: ContractKeySchema,
    /// Expected Stellar network passphrase. Validated against the RPC node at
    /// startup. Configured via `STELLAR_NETWORK_PASSPHRASE`; defaults to the
    /// canonical passphrase for the configured `BLOCKCHAIN_NETWORK`.
    pub network_passphrase: String,
    /// TTL (in seconds) for entries in the watched-transaction map.
    /// Entries older than this are evicted on the next insert regardless of
    /// their finalization status, bounding memory growth.
    /// Default: 1800 (30 minutes). Set via `WATCHED_TX_TTL_SECS`.
    pub watched_tx_ttl_secs: u64,
    /// Maximum number of transaction hashes that may be tracked simultaneously
    /// in the in-memory watch map.  When the cap is reached new registrations
    /// are rejected with 503 Service Unavailable rather than silently evicting
    /// older entries.  Default: 10000.  Set via `WATCHED_TX_MAX_SIZE`.
    pub watched_tx_max_size: usize,
    /// Whether the service is running in a production environment.
    /// When `true` a Stellar network passphrase mismatch at startup causes a
    /// hard `process::exit(1)`.  When `false` only a warning is logged.
    /// Default: `false`.  Set `PREDICTIQ_ENV=production` to enable.
    pub is_production: bool,
    /// Deployment environment name. Configured via `APP_ENV`; defaults to
    /// `"development"`. When set to `"production"` certain checks (e.g. RPC
    /// reachability) become fatal rather than warnings.
    pub app_env: String,
    /// When `true`, the HTTPS redirect middleware enforces HTTPS by inspecting
    /// the `X-Forwarded-Proto` header and issuing a 301 redirect for plain HTTP
    /// requests. TLS termination is expected at the ALB, not at this process.
    /// Configured via `REQUIRE_HTTPS`. Default: `false`.
    pub require_https: bool,
    /// Admin rate limit: max requests per window. Default: 30.
    /// Set via `ADMIN_RATE_LIMIT_MAX`. Must be > 0 in production.
    pub admin_rate_limit_max: u32,
    /// Admin rate limit window in seconds. Default: 60.
    /// Set via `ADMIN_RATE_LIMIT_WINDOW_SECS`. Must be > 0.
    pub admin_rate_limit_window_secs: u64,
    /// Global rate limit: max requests per window per IP. Default: 100.
    /// Set via `GLOBAL_RATE_LIMIT_MAX`. Must be > 0 in production.
    pub global_rate_limit_max: u32,
    /// Global rate limit window in seconds. Default: 60.
    /// Set via `GLOBAL_RATE_LIMIT_WINDOW_SECS`. Must be > 0.
    pub global_rate_limit_window_secs: u64,
    /// Maximum accepted request body size in bytes, resolved **once** at
    /// startup from `REQUEST_BODY_MAX_BYTES`.  Defaults to
    /// [`DEFAULT_REQUEST_BODY_MAX_BYTES`] (1 MiB).  The body-size middleware
    /// reads this value from shared state instead of re-reading the
    /// environment on every request.
    pub request_body_max_bytes: usize,
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

        let network_passphrase = env::var("STELLAR_NETWORK_PASSPHRASE")
            .unwrap_or_else(|_| match &blockchain_network {
                BlockchainNetwork::Testnet => "Test SDF Network ; September 2015".to_string(),
                BlockchainNetwork::Mainnet => {
                    "Public Global Stellar Network ; September 2015".to_string()
                }
                BlockchainNetwork::Custom => String::new(),
            });

        let db_credentials = DbCredentials::from_env();
        // Assemble the legacy database_url from individual components so
        // existing callers (sqlx migration runner) continue to work.
        // The password is exposed here only to construct the pool; this field
        // must never be logged.
        #[allow(deprecated)]
        let database_url = db_credentials.to_connection_string().expose_secret().to_string();

        // Resolve the request body-size limit exactly once, at startup.
        let request_body_max_bytes =
            parse_request_body_max_bytes(env::var("REQUEST_BODY_MAX_BYTES").ok().as_deref());

        Self {
            bind_addr,
            redis_url: redis_url_from_env(),
            db_credentials,
            database_url,
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
                statement_timeout_ms: env::var("DB_STATEMENT_TIMEOUT_MS")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(30_000),
                lock_timeout_ms: env::var("DB_LOCK_TIMEOUT_MS")
                    .ok()
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(10_000),
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
            rpc_backoff_jitter_factor: env::var("RPC_BACKOFF_JITTER_FACTOR")
                .ok()
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(1.0)
                .clamp(0.0, 1.0),
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
            sendgrid_key_rotated_at: env::var("SENDGRID_KEY_ROTATED_AT").ok(),
            base_url: env::var("BASE_URL").unwrap_or_else(|_| "http://localhost:8080".to_string()),
            email_idempotency_secret: env::var("EMAIL_IDEMPOTENCY_SECRET")
                .or_else(|_| env::var("HMAC_KEY"))
                .unwrap_or_default(),
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
                .unwrap_or(false),
            request_signing_secret: env::var("REQUEST_SIGNING_SECRET").ok(),
            // SendGrid Event Webhook signing key: base64-encoded DER
            // SubjectPublicKeyInfo for the ECDSA P-256 public key. Loaded from
            // the environment (Secrets Manager) — never hard-coded.
            sendgrid_webhook_secret: env::var("SENDGRID_WEBHOOK_SECRET")
                .ok()
                .filter(|s| !s.trim().is_empty()),
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
            newsletter_cleanup_batch_size: env::var("NEWSLETTER_CLEANUP_BATCH_SIZE")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(500),
            email_per_recipient_hourly_limit: env::var("EMAIL_PER_RECIPIENT_HOURLY_LIMIT")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(10),
            unsubscribe_signing_secret: env::var("UNSUBSCRIBE_SIGNING_SECRET").ok(),
            cors: CorsConfig::from_env(),
            contract_key_schema: ContractKeySchema::from_env(),
            network_passphrase,
            watched_tx_ttl_secs: env::var("WATCHED_TX_TTL_SECS")
                .ok()
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(1800),
            watched_tx_max_size: env::var("WATCHED_TX_MAX_SIZE")
                .ok()
                .and_then(|s| s.parse::<usize>().ok())
                .unwrap_or(10_000),
            // Canonical production flag derived from APP_ENV so a single
            // environment variable controls all production-only behaviors.
            is_production: env::var("APP_ENV")
                .map(|v| v.eq_ignore_ascii_case("production"))
                .unwrap_or(false),
            app_env: {
                let value = env::var("APP_ENV").unwrap_or_else(|_| "development".to_string());
                if let Err(msg) = validate_app_env(&value) {
                    panic!("{msg}");
                }
                value
            },
            require_https: env::var("REQUIRE_HTTPS")
                .map(|v| v.eq_ignore_ascii_case("true") || v == "1")
                .unwrap_or(false),
            admin_rate_limit_max: env::var("ADMIN_RATE_LIMIT_MAX")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30),
            admin_rate_limit_window_secs: env::var("ADMIN_RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            global_rate_limit_max: env::var("GLOBAL_RATE_LIMIT_MAX")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(100),
            global_rate_limit_window_secs: env::var("GLOBAL_RATE_LIMIT_WINDOW_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(60),
            request_body_max_bytes,
        }
    }

    pub fn network_name(&self) -> &'static str {
        match self.blockchain_network {
            BlockchainNetwork::Testnet => "testnet",
            BlockchainNetwork::Mainnet => "mainnet",
            BlockchainNetwork::Custom => "custom",
        }
    }

    pub fn is_production(&self) -> bool {
        self.app_env.eq_ignore_ascii_case("production")
    }

    /// Return the raw base64-encoded DER public key used to verify SendGrid
    /// Event Webhook signatures, if configured.
    ///
    /// The value is the base64 DER SubjectPublicKeyInfo of SendGrid's ECDSA
    /// P-256 signing key, as shown in the SendGrid dashboard. Verification
    /// (ECDSA P-256 / SHA-256 over `timestamp || payload`) is performed by the
    /// newsletter webhook handler; see `docs/webhook-security.md`.
    ///
    /// Returns `None` when `SENDGRID_WEBHOOK_SECRET` is unset or empty, in
    /// which case the handler must reject all webhook requests.
    pub fn sendgrid_webhook_public_key(&self) -> Option<&str> {
        self.sendgrid_webhook_secret
    }

    /// Emit a `tracing::warn!` if `SENDGRID_KEY_ROTATED_AT`
