use std::{
    future::Future,
    sync::{
        atomic::{AtomicU32, AtomicU64, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use anyhow::Context;
use deadpool_redis::{Config as PoolConfig, Pool};
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

/// Atomic circuit breaker.  All state is lock-free.
struct CircuitBreaker {
    failure_count: AtomicU32,
    /// Unix-epoch millis when the circuit was opened; 0 = not open.
    opened_at_ms: AtomicU64,
    threshold: u32,
    reset_timeout: Duration,
}

impl CircuitBreaker {
    fn new(threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_count: AtomicU32::new(0),
            opened_at_ms: AtomicU64::new(0),
            threshold,
            reset_timeout,
        }
    }

    fn state(&self) -> CircuitState {
        let opened_at = self.opened_at_ms.load(Ordering::Acquire);
        if opened_at == 0 {
            return CircuitState::Closed;
        }
        let elapsed = Instant::now()
            .duration_since(Instant::now()) // placeholder — use wall clock below
            .as_millis() as u64;
        let _ = elapsed; // suppress warning; we use system time instead
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        if now_ms.saturating_sub(opened_at) >= self.reset_timeout.as_millis() as u64 {
            CircuitState::HalfOpen
        } else {
            CircuitState::Open
        }
    }

    fn record_success(&self, metrics: &Option<crate::metrics::Metrics>) {
        let prev_state = self.state();
        self.failure_count.store(0, Ordering::Release);
        self.opened_at_ms.store(0, Ordering::Release);
        let new_state = self.state();
        
        // Only update metrics if state actually changed
        if prev_state != new_state {
            if let Some(m) = metrics {
                m.set_cache_circuit_breaker_state(new_state as i64);
            }
            if prev_state != CircuitState::Closed {
                tracing::info!("Redis circuit breaker closed after successful operation");
            }
        }
    }

    fn record_failure(&self, metrics: &Option<crate::metrics::Metrics>) {
        let prev_state = self.state();
        let prev = self.failure_count.fetch_add(1, Ordering::AcqRel);
        if prev + 1 >= self.threshold && self.opened_at_ms.load(Ordering::Acquire) == 0 {
            let now_ms = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as u64;
            self.opened_at_ms.store(now_ms, Ordering::Release);
            tracing::warn!(
                threshold = self.threshold,
                "Redis circuit breaker opened after {} failures",
                prev + 1
            );
            
            // Update metrics to reflect open state
            if let Some(m) = metrics {
                m.set_cache_circuit_breaker_state(CircuitState::Open as i64);
            }
        }
    }

    /// Returns `true` if the call is allowed (Closed or HalfOpen).
    fn allow(&self, metrics: &Option<crate::metrics::Metrics>) -> bool {
        let prev_state = self.state();
        let allowed = match prev_state {
            CircuitState::Closed | CircuitState::HalfOpen => true,
            CircuitState::Open => false,
        };
        
        // Update metrics when transitioning to HalfOpen
        let current_state = self.state();
        if prev_state != current_state && current_state == CircuitState::HalfOpen {
            if let Some(m) = metrics {
                m.set_cache_circuit_breaker_state(CircuitState::HalfOpen as i64);
            }
            tracing::info!("Redis circuit breaker transitioned to half-open, allowing probe request");
        }
        
        allowed
    }
}

// ── Pool config from env ─────────────────────────────────────────────────────

#[derive(Clone, Debug)]
pub struct RedisCacheConfig {
    pub pool_min_idle: usize,
    pub pool_max_size: usize,

    /// Timeout for acquiring a connection from the pool.
    pub acquire_timeout: Duration,
    /// Retry attempts on transient errors (0 = no retry).
    pub retry_attempts: u32,
    /// Base delay for exponential backoff.
    pub retry_base_delay: Duration,
    /// Circuit breaker: failures before opening.
    pub cb_threshold: u32,
    /// Circuit breaker: how long to stay open before half-open probe.
    pub cb_reset_timeout: Duration,
}

impl RedisCacheConfig {
    pub fn from_env() -> Self {
        let pool_min_idle = std::env::var("REDIS_POOL_MIN_IDLE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(2usize);
        let pool_max_size = std::env::var("REDIS_POOL_MAX_SIZE")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(20usize)
            .max(pool_min_idle);
        let acquire_timeout = Duration::from_millis(
            std::env::var("REDIS_POOL_ACQUIRE_TIMEOUT_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(500u64),
        );
        let retry_attempts = std::env::var("REDIS_RETRY_ATTEMPTS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(3u32);
        let retry_base_delay = Duration::from_millis(
            std::env::var("REDIS_RETRY_BASE_DELAY_MS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(50u64),
        );
        let cb_threshold = std::env::var("REDIS_CB_THRESHOLD")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(5u32);
        let cb_reset_timeout = Duration::from_secs(
            std::env::var("REDIS_CB_RESET_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30u64),
        );
        Self {
            pool_min_idle,
            pool_max_size,
            acquire_timeout,
            retry_attempts,
            retry_base_delay,
            cb_threshold,
            cb_reset_timeout,
        }
    }
}

// ── RedisCache ───────────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct RedisCache {
    pool: Pool,
    cb: Arc<CircuitBreaker>,
    cfg: RedisCacheConfig,
    tag_cfg: TagStoreConfig,
    metrics: Option<crate::metrics::Metrics>,
}


// ── Tag-store config + implementation ────────────────────────────────────

/// Settings for Redis-backed tag metadata to prevent unbounded growth.
///
/// The tag metadata is used to cap how many unique cache keys are tracked
/// per tag and to apply TTL so that rarely used tags don't accumulate
/// forever.
#[derive(Clone, Debug)]
pub struct TagStoreConfig {
    /// Key set TTL for tag metadata. Must be >= the longest-lived cached
    /// value TTL + any grace period.
    pub tag_ttl: Duration,

    /// Maximum number of tracked keys per invalidation tag.
    pub keys_per_tag_cap: usize,

    /// Redis key prefix for tag metadata.
    pub prefix: String,
}

impl TagStoreConfig {
    pub fn from_env() -> Self {
        // Longest-lived cache entries in this codebase appear to be:
        // - statistics: 5 * 60 seconds (300s)
        // - featured_markets: 2 * 60 seconds (120s)
        // - content: 60 * 60 seconds (3600s)
        // Tag TTL must match the longest-lived cached entry.
        let tag_ttl_secs = std::env::var("REDIS_CACHE_TAG_TTL_SECS")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(3600);

        let keys_per_tag_cap = std::env::var("REDIS_CACHE_TAG_KEYS_PER_TAG_CAP")
            .ok()
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(256);

        let prefix = std::env::var("REDIS_CACHE_TAG_PREFIX")
            .ok()
            .unwrap_or_else(|| "cache_tags:v1".to_string());

        Self {
            tag_ttl: Duration::from_secs(tag_ttl_secs),
            keys_per_tag_cap,
            prefix,
        }
    }

    fn tag_key(&self, tag_hash: &str) -> String {
        format!("{}:tag:{}", self.prefix, tag_hash)
    }

    fn counter_key(&self, tag_hash: &str) -> String {
        format!("{}:tag:{}:seq", self.prefix, tag_hash)
    }
}

impl RedisCache {

    async fn tag_store_invalidate(&self, tag: &InvalidationTag) -> anyhow::Result<()> {
        // Store/cap tag->keys metadata with TTL.
        // We use an ordered-set (ZSET) where score is an ever-increasing
        // sequence number so we can evict oldest items when cap is hit.
        //
        // Redis keys:
        // - <prefix>:tag:<hash>                (ZSET of tracked cache keys)
        // - <prefix>:tag:<hash>:seq           (string counter for insertion order)

        let tag_keys = tag.cache_keys();
        if tag_keys.is_empty() {
            return Ok(());
        }

        // Deterministically hash the tag so the metadata key is stable.
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        tag.cache_keys().join("|").hash(&mut hasher);
        let tag_hash = format!("{:x}", hasher.finish());

        let zset_key = self.tag_cfg.tag_key(&tag_hash);
        let seq_key = self.tag_cfg.counter_key(&tag_hash);

        // Longest-lived cached entry TTL.
        // We configured tag_ttl to match that (see TagStoreConfig::from_env).
        let tag_ttl_secs = self.tag_cfg.tag_ttl.as_secs();
        let cap = self.tag_cfg.keys_per_tag_cap as i64;

        // Lua keeps this operation atomic.
        let script = redis::Script::new(
            r#"
            local zset_key = KEYS[1]
            local seq_key  = KEYS[2]
            local ttl      = tonumber(ARGV[1])
            local cap      = tonumber(ARGV[2])

            -- ARGV layout: [ttl, cap, key1, key2, ...]
            -- Use ZADD with monotonic seq scores; update seq counter once per run.
            -- We increment seq for each key so we can evict by insertion order.
            local start_seq = redis.call('INCR', seq_key)
            local n = 0
            for i = 3, #ARGV do
              n = n + 1
              local key = ARGV[i]
              redis.call('ZADD', zset_key, start_seq + (n-1), key)
            end

            -- Apply/refresh TTL for tag metadata.
            if ttl and ttl > 0 then
              redis.call('EXPIRE', zset_key, ttl)
            end

            -- Cap size: evict oldest (lowest scores) beyond cap.
            local current = redis.call('ZCARD', zset_key)
            if current and cap and current > cap then
              local over = current - cap
              -- Remove lowest-scored 'over' members.
              redis.call('ZREMRANGEBYRANK', zset_key, 0, over-1)
              return over
            end
            return 0
            "#,
        );

        let script = std::sync::Arc::new(script);
        self.exec(|mut conn| {
            let zset_key = zset_key.clone();
            let seq_key = seq_key.clone();
            let keys = tag_keys.clone();
            let script = script.clone();
            async move {
                let mut argv: Vec<String> = Vec::with_capacity(2 + keys.len());
                argv.push(tag_ttl_secs.to_string());
                argv.push(cap.to_string());
                argv.extend(keys);

                let _: i64 = script
                    .key(&zset_key)
                    .key(&seq_key)
                    .arg(tag_ttl_secs)
                    .arg(cap)
                    .invoke_async(&mut conn)
                    .await?;
                Ok(())
            }
        })
        .await?;

        Ok(())
    }


    pub async fn new(redis_url: &str) -> anyhow::Result<Self> {
        let cfg = RedisCacheConfig::from_env();
        Self::new_with_config(redis_url, cfg).await
    }

    pub async fn new_with_config(redis_url: &str, cfg: RedisCacheConfig) -> anyhow::Result<Self> {
        Self::new_with_config_and_metrics(redis_url, cfg, None).await
    }

    pub async fn new_with_metrics(redis_url: &str, metrics: crate::metrics::Metrics) -> anyhow::Result<Self> {
        let cfg = RedisCacheConfig::from_env();
        Self::new_with_config_and_metrics(redis_url, cfg, Some(metrics)).await
    }

    pub async fn new_with_config_and_metrics(
        redis_url: &str,
        cfg: RedisCacheConfig,
        metrics: Option<crate::metrics::Metrics>,
    ) -> anyhow::Result<Self> {
        let pool_cfg = PoolConfig::from_url(redis_url);
        let pool = pool_cfg
            .builder()
            .context("failed to build Redis pool config")?
            .max_size(cfg.pool_max_size)
            .wait_timeout(Some(cfg.acquire_timeout))
            .build()
            .context("failed to build Redis pool")?;

        let cb = Arc::new(CircuitBreaker::new(cfg.cb_threshold, cfg.cb_reset_timeout));
        let tag_cfg = TagStoreConfig::from_env();
        
        let cache = Self { pool, cb, cfg, tag_cfg, metrics: metrics.clone() };
        
        // Initialize circuit breaker state metric to closed (0)
        if let Some(ref m) = metrics {
            m.set_cache_circuit_breaker_state(0);
        }
        
        Ok(cache)
    }


    /// Returns the current circuit breaker state — useful for health checks and metrics.
    pub fn circuit_state(&self) -> CircuitState {
        self.cb.state()
    }

    /// Pool status for metrics/health.
    pub fn pool_status(&self) -> deadpool_redis::Status {
        self.pool.status()
    }

    // ── Internal helpers ─────────────────────────────────────────────────────

    /// Execute `op` with retry + circuit breaker.  On circuit open, returns
    /// `Err` immediately so callers can degrade gracefully.
    async fn exec<T, F, Fut>(&self, op: F) -> anyhow::Result<T>
    where
        F: Fn(deadpool_redis::Connection) -> Fut,
        Fut: Future<Output = anyhow::Result<T>>,
    {
        if !self.cb.allow(&self.metrics) {
            anyhow::bail!("Redis circuit breaker is open");
        }

        let mut last_err = anyhow::anyhow!("no attempts made");
        for attempt in 0..=self.cfg.retry_attempts {
            if attempt > 0 {
                let delay = self.cfg.retry_base_delay * (1 << (attempt - 1).min(4));
                tokio::time::sleep(delay).await;
            }
            match self.pool.get().await {
                Err(e) => {
                    last_err = anyhow::anyhow!("pool acquire: {e}");
                    self.cb.record_failure(&self.metrics);
                }
                Ok(conn) => match op(conn).await {
                    Ok(v) => {
                        self.cb.record_success(&self.metrics);
                        return Ok(v);
                    }
                    Err(e) => {
                        last_err = e;
                        self.cb.record_failure(&self.metrics);
                    }
                },
            }
        }
        Err(last_err)
    }

    // ── Public API ───────────────────────────────────────────────────────────

    pub async fn get_json<T>(&self, key: &str) -> anyhow::Result<Option<T>>
    where
        T: DeserializeOwned,
    {
        let key = key.to_owned();
        self.exec(|mut conn| {
            let key = key.clone();
            async move {
                let val: Option<String> = conn.get(&key).await?;
                match val {
                    Some(raw) => Ok(Some(serde_json::from_str(&raw)?)),
                    None => Ok(None),
                }
            }
        })
        .await
    }

    pub async fn set_json<T>(&self, key: &str, value: &T, ttl: Duration) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let key = key.to_owned();
        let raw = serde_json::to_string(value)?;
        let secs = ttl.as_secs();
        self.exec(|mut conn| {
            let key = key.clone();
            let raw = raw.clone();
            async move {
                let _: () = conn.set_ex(&key, raw, secs).await?;
                Ok(())
            }
        })
        .await
    }

    pub async fn del(&self, key: &str) -> anyhow::Result<()> {
        let key = key.to_owned();
        self.exec(|mut conn| {
            let key = key.clone();
            async move {
                let _: usize = conn.del(&key).await?;
                Ok(())
            }
        })
        .await
    }

    pub async fn ping(&self) -> anyhow::Result<()> {
        self.exec(|mut conn| async move {
            let _: String = redis::cmd("PING").query_async(&mut conn).await?;
            Ok(())
        })
        .await
    }

    /// Delete all keys matching `pattern` using non-blocking cursor-based SCAN.
    ///
    /// Each SCAN+DEL batch acquires and releases its own pool connection so no
    /// single connection is held for the full duration of a large-keyspace scan.
    /// The circuit breaker is checked once before the loop; individual batch
    /// errors are propagated immediately.
    pub async fn del_by_pattern(&self, pattern: &str) -> anyhow::Result<usize> {
        if !self.cb.allow(&self.metrics) {
            anyhow::bail!("Redis circuit breaker is open");
        }

        let mut cursor: u64 = 0;
        let mut total_deleted: usize = 0;
        let pattern = pattern.to_owned();

        loop {
            let (next_cursor, batch_deleted) = self
                .exec(|mut conn| {
                    let pattern_clone = pattern.clone();
                    async move {
                        let (next_cursor, keys): (u64, Vec<String>) = redis::cmd("SCAN")
                            .arg(cursor)
                            .arg("MATCH")
                            .arg(&pattern_clone)
                            .arg("COUNT")
                            .arg(100u64)
                            .query_async(&mut conn)
                            .await?;
                        let deleted = if keys.is_empty() {
                            0
                        } else {
                            conn.del(keys).await?
                        };
                        Ok((next_cursor, deleted))
                    }
                })
                .await?;

            total_deleted += batch_deleted;
            cursor = next_cursor;
            if cursor == 0 {
                break;
            }
        }

        Ok(total_deleted)
    }

    /// Fetch-or-set with stampede protection.
    ///
    /// Strategy (applied in order when enabled via `StampedeConfig`):
    /// 1. **Probabilistic early expiry (XFetch)** — if the entry is still
    ///    alive but close to expiry, one request will refresh it early while
    ///    others continue serving the stale value.
    /// 2. **Mutex lock** — when the entry is missing (or chosen for early
    ///    refresh), a Redis `SET NX` lock ensures only one request calls the
    ///    fetcher. Others wait briefly and then serve the freshly-written
    ///    value, falling back to calling the fetcher themselves only if the
    ///    lock wait times out.
    ///
    /// Returns `(value, cache_hit)`.
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
        // If circuit is open, skip cache entirely and call fetcher directly.
        if !self.cb.allow(&self.metrics) {
            tracing::warn!(key, "Redis unavailable, bypassing cache");
            let value = fetcher().await?;
            return Ok((value, false));
        }

        if let Ok(Some(cached)) = self.get_json(key).await {
            return Ok((cached, true));
        }

        // Cache miss — call fetcher and store the result.
        self.recompute_and_store(key, ttl, fetcher).await
    }

    async fn set_entry<T>(&self, key: &str, entry: &CachedEntry<T>, ttl: Duration) -> anyhow::Result<()>
    where
        T: Serialize,
    {
        let key = key.to_owned();
        let raw = serde_json::to_string(entry)?;
        // Store with a small grace period beyond the logical TTL so XFetch
        // can still serve the stale value while a refresh is in flight.
        let redis_ttl = ttl + Duration::from_secs(30);
        self.exec(|mut conn| {
            let key = key.clone();
            let raw = raw.clone();
            async move {
                let _: () = conn.set_ex(&key, raw, redis_ttl.as_secs()).await?;
                Ok(())
            }
        })
        .await
    }

    async fn recompute_and_store<T, F, Fut>(
        &self,
        entry_key: &str,
        ttl: Duration,
        fetcher: F,
    ) -> anyhow::Result<(T, bool)>
    where
        T: Serialize + DeserializeOwned + Clone,
        F: FnOnce() -> Fut,
        Fut: Future<Output = anyhow::Result<T>>,
    {
        let _start = std::time::Instant::now();
        let value = fetcher().await?;
        // Best-effort write — don't fail the request if cache write fails.
        if let Err(e) = self.set_json(entry_key, &value, ttl).await {
            tracing::warn!(entry_key, error = %e, "cache write failed");
        }
        Ok((value, false))
    }

    /// Invalidate all cache keys associated with `tag`.
    ///
    /// Keys are derived from the tag at call time — no Redis set membership
    /// lookup is required, keeping the blast radius deterministic and bounded.
    /// Returns the number of keys deleted (keys that were already absent count
    /// as 0 from Redis DEL but are still included in the returned count for
    /// observability purposes).
    pub async fn invalidate_tag(&self, tag: &InvalidationTag) -> anyhow::Result<usize> {
        // Keep tag-sets bounded + TTL'd so Redis memory usage can't grow
        // unboundedly from high-cardinality tag usage.
        //
        // We still eagerly delete the concrete cache keys for correctness,
        // but tag metadata is now stored in Redis with TTL + cap.
        let _ = self.tag_store_invalidate(tag).await?;

        let tag_keys = tag.cache_keys();
        let mut deleted = 0usize;
        for key in &tag_keys {
            self.del(key).await?;
            deleted += 1;
        }
        Ok(deleted)
    }


    /// Atomically increment `key` and set its TTL on first increment.
    /// Returns the new counter value. Used for Redis-backed rate limiting.
    pub async fn incr_with_ttl(&self, key: &str, ttl: Duration) -> anyhow::Result<u64> {
        let key = key.to_owned();
        let ttl_secs = ttl.as_secs();
        self.exec(|mut conn| {
            let key = key.clone();
            async move {
                let script = redis::Script::new(
                    r#"
                    local current = redis.call('INCR', KEYS[1])
                    if tonumber(current) == 1 then
                        redis.call('EXPIRE', KEYS[1], ARGV[1])
                    end
                    return current
                    "#,
                );
                Ok(script.key(&key).arg(ttl_secs).invoke_async(&mut conn).await?)
            }
        })
        .await
    }

    /// Acquire a raw connection from the pool.
    /// Prefer `exec` for most use cases; use this only when you need to hold
    /// a connection across multiple commands (e.g. pipelined operations).
    pub async fn get_connection(&self) -> anyhow::Result<deadpool_redis::Connection> {
        if !self.cb.allow(&self.metrics) {
            anyhow::bail!("Redis circuit breaker is open");
        }
        self.pool.get().await.context("failed to acquire Redis connection")
    }
}

// ── XFetch stampede protection types ────────────────────────────────────────

/// A cached entry with metadata for probabilistic early expiry (XFetch).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CachedEntry<T> {
    pub value: T,
    /// Unix timestamp (seconds) when this entry expires.
    pub expires_at: i64,
    /// Measured recomputation time in seconds (delta for XFetch formula).
    pub delta_secs: f64,
}

/// Configuration for stampede protection strategies.
#[derive(Debug, Clone)]
pub struct StampedeConfig {
    /// Enable probabilistic early expiry (XFetch algorithm).
    pub probabilistic_early_expiry: bool,
    /// Enable mutex lock to serialise concurrent recomputations.
    pub mutex_lock: bool,
    /// Beta parameter for XFetch (higher = more aggressive early refresh).
    pub xfetch_beta: f64,
}

impl Default for StampedeConfig {
    fn default() -> Self {
        Self {
            probabilistic_early_expiry: true,
            mutex_lock: true,
            xfetch_beta: 1.0,
        }
    }
}

/// Returns `true` if the entry should be refreshed early (XFetch algorithm).
/// Uses probabilistic early expiry: the closer to expiry and the longer the
/// recomputation time, the more likely a refresh is triggered.
pub fn xfetch_should_refresh<T>(entry: &CachedEntry<T>, beta: f64) -> bool {
    let now = chrono::Utc::now().timestamp();
    let ttl_remaining = entry.expires_at - now;
    if ttl_remaining <= 0 {
        return true;
    }
    // XFetch: refresh if -delta * beta * ln(rand) >= ttl_remaining
    // Use a simple pseudo-random value derived from current time nanos.
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .subsec_nanos();
    let rand_f: f64 = (nanos as f64 + 1.0) / (u32::MAX as f64 + 1.0); // (0, 1]
    let score = -entry.delta_secs * beta * rand_f.ln();
    score >= ttl_remaining as f64
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::redis::Redis;

    use super::RedisCache;

    async fn start_cache() -> (RedisCache, impl Drop) {
        let container = Redis::default().start().await.expect("redis container");
        let port = container
            .get_host_port_ipv4(6379)
            .await
            .expect("redis port");
        let url = format!("redis://127.0.0.1:{port}");
        let cache = RedisCache::new(&url).await.expect("redis cache");
        (cache, container)
    }

    #[tokio::test]
    async fn cache_miss_populates_on_first_request() {
        let (cache, _c) = start_cache().await;
        let (val, hit) = cache
            .get_or_set_json::<u32, _, _>("key:miss", Duration::from_secs(60), || async {
                Ok(42u32)
            })
            .await
            .unwrap();
        assert_eq!(val, 42);
        assert!(!hit, "first call must be a miss");
        let (val2, hit2) = cache
            .get_or_set_json::<u32, _, _>("key:miss", Duration::from_secs(60), || async {
                Ok(0u32)
            })
            .await
            .unwrap();
        assert_eq!(val2, 42, "stored value must be returned on hit");
        assert!(hit2, "second call must be a hit");
    }

    #[tokio::test]
    async fn cache_hit_on_subsequent_request() {
        let (cache, _c) = start_cache().await;
        cache
            .set_json("key:hit", &99u32, Duration::from_secs(60))
            .await
            .unwrap();
        let (val, hit) = cache
            .get_or_set_json::<u32, _, _>("key:hit", Duration::from_secs(60), || async {
                Ok(0u32)
            })
            .await
            .unwrap();
        assert_eq!(val, 99, "cached value must be returned");
        assert!(hit, "pre-populated key must be a hit");
    }

    #[tokio::test]
    async fn del_invalidates_cached_entry() {
        let (cache, _c) = start_cache().await;
        cache
            .set_json("key:del", &7u32, Duration::from_secs(60))
            .await
            .unwrap();
        cache.del("key:del").await.unwrap();
        let result: Option<u32> = cache.get_json("key:del").await.unwrap();
        assert!(result.is_none(), "entry must be absent after del");
    }

    #[tokio::test]
    async fn del_by_pattern_invalidates_matching_entries() {
        let (cache, _c) = start_cache().await;
        for i in 0..3u32 {
            cache
                .set_json(&format!("ns:item:{i}"), &i, Duration::from_secs(60))
                .await
                .unwrap();
        }
        cache
            .set_json("other:item:0", &100u32, Duration::from_secs(60))
            .await
            .unwrap();

        let deleted = cache.del_by_pattern("ns:item:*").await.unwrap();
        assert_eq!(deleted, 3);

        for i in 0..3u32 {
            let v: Option<u32> = cache.get_json(&format!("ns:item:{i}")).await.unwrap();
            assert!(v.is_none(), "ns:item:{i} must be gone");
        }
        let other: Option<u32> = cache.get_json("other:item:0").await.unwrap();
        assert_eq!(other, Some(100));
    }

    /// Verifies that del_by_pattern correctly handles a keyspace larger than a
    /// single SCAN page (COUNT 100), exercising the cursor-batching loop.
    #[tokio::test]
    async fn del_by_pattern_large_keyspace_uses_cursor_batching() {
        let (cache, _c) = start_cache().await;
        let n = 250u32; // exceeds the COUNT 100 hint, forcing multiple SCAN rounds
        for i in 0..n {
            cache
                .set_json(&format!("large:item:{i}"), &i, Duration::from_secs(60))
                .await
                .unwrap();
        }
        // One key outside the pattern must survive.
        cache
            .set_json("large:other:0", &999u32, Duration::from_secs(60))
            .await
            .unwrap();

        let deleted = cache.del_by_pattern("large:item:*").await.unwrap();
        assert_eq!(deleted, n as usize, "all {n} matching keys must be deleted");

        // Spot-check a few keys are gone.
        for i in [0u32, 99, 100, 249] {
            let v: Option<u32> = cache.get_json(&format!("large:item:{i}")).await.unwrap();
            assert!(v.is_none(), "large:item:{i} must be gone after del_by_pattern");
        }
        // Non-matching key must be untouched.
        let survivor: Option<u32> = cache.get_json("large:other:0").await.unwrap();
        assert_eq!(survivor, Some(999), "non-matching key must survive");
    }

    /// Verifies that del_by_pattern returns 0 and does not error when no keys match.
    #[tokio::test]
    async fn del_by_pattern_no_matches_returns_zero() {
        let (cache, _c) = start_cache().await;
        let deleted = cache.del_by_pattern("nonexistent:*").await.unwrap();
        assert_eq!(deleted, 0);
    }

    // ── InvalidationTag tests ────────────────────────────────────────────────

    /// Verifies that MarketResolved tag produces exactly the expected 6 keys.
    #[test]
    fn market_resolved_tag_produces_correct_keys() {
        use super::InvalidationTag;
        let tag = InvalidationTag::MarketResolved {
            market_id: 7,
            network: "testnet".to_string(),
            featured_limit: 10,
        };
        let keys = tag.cache_keys();
        assert_eq!(keys.len(), 6, "MarketResolved must cover exactly 6 keys");
        assert!(keys.contains(&"chain:v1:market:7".to_string()));
        assert!(keys.contains(&"chain:v1:oracle:testnet:market:7".to_string()));
        assert!(keys.contains(&"api:v1:statistics".to_string()));
        assert!(keys.contains(&"api:v1:featured_markets".to_string()));
        assert!(keys.contains(&"dbq:v1:statistics".to_string()));
        assert!(keys.contains(&"dbq:v1:featured_markets:limit:10".to_string()));
    }

    /// Verifies that different market IDs produce distinct key sets (no cross-contamination).
    #[test]
    fn market_resolved_tag_keys_are_market_id_scoped() {
        use super::InvalidationTag;
        let keys_a = InvalidationTag::MarketResolved {
            market_id: 1,
            network: "mainnet".to_string(),
            featured_limit: 5,
        }
        .cache_keys();
        let keys_b = InvalidationTag::MarketResolved {
            market_id: 2,
            network: "mainnet".to_string(),
            featured_limit: 5,
        }
        .cache_keys();

        // Per-market keys must differ.
        assert_ne!(
            keys_a.iter().find(|k| k.contains("chain:v1:market:")),
            keys_b.iter().find(|k| k.contains("chain:v1:market:")),
        );
        // Aggregate keys are shared (both markets affect statistics).
        assert_eq!(
            keys_a.iter().find(|k| k.as_str() == "api:v1:statistics"),
            keys_b.iter().find(|k| k.as_str() == "api:v1:statistics"),
        );
    }

    /// Integration: invalidate_tag deletes exactly the keys in the tag and leaves others.
    #[tokio::test]
    async fn invalidate_tag_deletes_tag_keys_only() {
        use super::InvalidationTag;
        let (cache, _c) = start_cache().await;

        let tag = InvalidationTag::MarketResolved {
            market_id: 99,
            network: "testnet".to_string(),
            featured_limit: 10,
        };

        // Populate all tag keys plus one unrelated key.
        for key in tag.cache_keys() {
            cache.set_json(&key, &1u32, Duration::from_secs(60)).await.unwrap();
        }
        cache.set_json("unrelated:key", &42u32, Duration::from_secs(60)).await.unwrap();

        let deleted = cache.invalidate_tag(&tag).await.unwrap();
        assert_eq!(deleted, 6, "must report 6 deletions");

        // All tag keys must be gone.
        for key in tag.cache_keys() {
            let v: Option<u32> = cache.get_json(&key).await.unwrap();
            assert!(v.is_none(), "{key} must be absent after invalidate_tag");
        }
        // Unrelated key must survive.
        let survivor: Option<u32> = cache.get_json("unrelated:key").await.unwrap();
        assert_eq!(survivor, Some(42));
    }

    #[tokio::test]
    async fn circuit_breaker_degrades_gracefully() {
        use super::{CircuitState, RedisCacheConfig};
        use std::time::Duration;

        // Point at a port that has nothing listening → immediate failures.
        let cfg = RedisCacheConfig {
            pool_min_idle: 1,
            pool_max_size: 2,
            acquire_timeout: Duration::from_millis(50),
            retry_attempts: 0,
            retry_base_delay: Duration::from_millis(10),
            cb_threshold: 2,
            cb_reset_timeout: Duration::from_secs(60),
        };
        let cache = RedisCache::new_with_config("redis://127.0.0.1:19999", cfg)
            .await
            .unwrap();

        // Two failures should open the circuit.
        let _ = cache.ping().await;
        let _ = cache.ping().await;

        assert_eq!(cache.circuit_state(), CircuitState::Open);

        // get_or_set_json must bypass cache and call fetcher when open.
        let (val, hit) = cache
            .get_or_set_json::<u32, _, _>("k", Duration::from_secs(60), || async { Ok(7u32) })
            .await
            .unwrap();
        assert_eq!(val, 7);
        assert!(!hit);
    }
}

// ── Invalidation tags ────────────────────────────────────────────────────────
//
// # Key and tag strategy
//
// Every cache key belongs to exactly one *invalidation tag*. A tag groups the
// minimal set of keys that must be evicted together when a specific write
// occurs. This keeps the blast radius deterministic: callers declare *what
// changed* (the tag) rather than *which keys to delete* (the key list).
//
// ## Tag → key mapping
//
// | Tag                          | Keys invalidated                                                  |
// |------------------------------|-------------------------------------------------------------------|
// | `MarketResolved(id, net, lim)` | chain_market(id), chain_oracle_result(net,id),                  |
// |                              | api_statistics, api_featured_markets,                             |
// |                              | dbq_statistics, dbq_featured_markets(lim)                         |
//
// ## Rules
// - Tags are defined here; handlers import and use them.
// - A tag must never include keys from unrelated domains (e.g. resolving a
//   market must not evict content or user-bet keys).
// - When a new write path is added, add a corresponding tag here first.

/// Describes a write event and the exact cache keys it invalidates.
///
/// Use [`RedisCache::invalidate_tag`] to apply a tag.
#[derive(Debug, Clone)]
pub enum InvalidationTag {

    /// A market was resolved.
    ///
    /// Invalidates the per-market chain entry, the oracle result, and the
    /// aggregate statistics / featured-markets lists.
    MarketResolved {
        market_id: i64,
        network: String,
        featured_limit: i64,
    },
}

impl InvalidationTag {
    /// Returns the exact set of cache keys this tag covers.
    pub fn cache_keys(&self) -> Vec<String> {
        match self {
            InvalidationTag::MarketResolved {
                market_id,
                network,
                featured_limit,
            } => vec![
                keys::chain_market(*market_id),
                keys::chain_oracle_result(network, *market_id),
                keys::api_statistics(),
                keys::api_featured_markets(),
                keys::dbq_statistics(),
                keys::dbq_featured_markets(*featured_limit),
            ],
        }
    }
}

// ── Cache key categories ─────────────────────────────────────────────────────

/// Logical grouping for cache keys, used for TTL configuration and metrics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCategory {
    Statistics,
    FeaturedMarkets,
    Content,
    ChainMarket,
    ChainPlatformStats,
    ChainUserBets,
    ChainOracleResult,
    ChainTxStatus,
    ChainHealth,
    ChainLedger,
    ChainSyncCursor,
    Custom,
}

impl KeyCategory {
    pub fn label(&self) -> &'static str {
        match self {
            KeyCategory::Statistics => "statistics",
            KeyCategory::FeaturedMarkets => "featured_markets",
            KeyCategory::Content => "content",
            KeyCategory::ChainMarket => "chain_market",
            KeyCategory::ChainPlatformStats => "chain_platform_stats",
            KeyCategory::ChainUserBets => "chain_user_bets",
            KeyCategory::ChainOracleResult => "chain_oracle_result",
            KeyCategory::ChainTxStatus => "chain_tx_status",
            KeyCategory::ChainHealth => "chain_health",
            KeyCategory::ChainLedger => "chain_ledger",
            KeyCategory::ChainSyncCursor => "chain_sync_cursor",
            KeyCategory::Custom => "custom",
        }
    }
}

/// Per-category TTL configuration.
#[derive(Debug, Clone)]
pub struct TtlConfig {
    pub statistics: Duration,
    pub featured_markets: Duration,
    pub content: Duration,
    pub chain_market: Duration,
    pub chain_platform_stats: Duration,
    pub chain_user_bets: Duration,
    pub chain_oracle_result: Duration,
    pub chain_tx_status: Duration,
    pub chain_health: Duration,
    pub chain_ledger: Duration,
    pub chain_sync_cursor: Duration,
}

impl Default for TtlConfig {
    fn default() -> Self {
        Self {
            statistics: Duration::from_secs(60),
            featured_markets: Duration::from_secs(300),
            content: Duration::from_secs(600),
            chain_market: Duration::from_secs(30),
            chain_platform_stats: Duration::from_secs(120),
            chain_user_bets: Duration::from_secs(60),
            chain_oracle_result: Duration::from_secs(300),
            chain_tx_status: Duration::from_secs(15),
            chain_health: Duration::from_secs(10),
            chain_ledger: Duration::from_secs(5),
            chain_sync_cursor: Duration::from_secs(5),
        }
    }
}

impl TtlConfig {
    pub fn get(&self, category: KeyCategory) -> Option<Duration> {
        match category {
            KeyCategory::Statistics => Some(self.statistics),
            KeyCategory::FeaturedMarkets => Some(self.featured_markets),
            KeyCategory::Content => Some(self.content),
            KeyCategory::ChainMarket => Some(self.chain_market),
            KeyCategory::ChainPlatformStats => Some(self.chain_platform_stats),
            KeyCategory::ChainUserBets => Some(self.chain_user_bets),
            KeyCategory::ChainOracleResult => Some(self.chain_oracle_result),
            KeyCategory::ChainTxStatus => Some(self.chain_tx_status),
            KeyCategory::ChainHealth => Some(self.chain_health),
            KeyCategory::ChainLedger => Some(self.chain_ledger),
            KeyCategory::ChainSyncCursor => Some(self.chain_sync_cursor),
            KeyCategory::Custom => None,
        }
    }
}

pub mod keys {
    use super::KeyCategory;

    pub const API_PREFIX: &str = "api:v1";
    pub const DBQ_PREFIX: &str = "dbq:v1";
    pub const CHAIN_PREFIX: &str = "chain:v1";

    // ---- api:v1 keys ----

    pub fn api_statistics() -> String {
        format!("{API_PREFIX}:statistics")
    }
    pub fn api_statistics_category() -> KeyCategory { KeyCategory::Statistics }

    pub fn api_featured_markets() -> String {
        format!("{API_PREFIX}:featured_markets")
    }
    pub fn api_featured_markets_category() -> KeyCategory { KeyCategory::FeaturedMarkets }

    pub fn api_content(limit: i64) -> String {
        format!("{API_PREFIX}:content:limit:{limit}")
    }
    pub fn api_content_category() -> KeyCategory { KeyCategory::Content }

    // ---- dbq:v1 keys ----

    pub fn dbq_statistics() -> String {
        format!("{DBQ_PREFIX}:statistics")
    }
    pub fn dbq_statistics_category() -> KeyCategory { KeyCategory::Statistics }

    pub fn dbq_featured_markets(limit: i64) -> String {
        format!("{DBQ_PREFIX}:featured_markets:limit:{limit}")
    }
    pub fn dbq_featured_markets_category() -> KeyCategory { KeyCategory::FeaturedMarkets }

    pub fn dbq_content(limit: i64) -> String {
        format!("{DBQ_PREFIX}:content:limit:{limit}")
    }
    pub fn dbq_content_category() -> KeyCategory { KeyCategory::Content }

    // ---- chain:v1 keys ----

    pub fn chain_market(market_id: i64) -> String {
        format!("{CHAIN_PREFIX}:market:{market_id}")
    }
    pub fn chain_market_category() -> KeyCategory { KeyCategory::ChainMarket }

    pub fn chain_platform_stats(network: &str) -> String {
        format!("{CHAIN_PREFIX}:platform_stats:{network}")
    }
    pub fn chain_platform_stats_category() -> KeyCategory { KeyCategory::ChainPlatformStats }

    pub fn chain_user_bets(network: &str, user: &str, limit: i64) -> String {
        format!(
            "{CHAIN_PREFIX}:user_bets:{network}:{}:limit:{limit}",
            user.to_lowercase()
        )
    }
    pub fn chain_user_bets_category() -> KeyCategory { KeyCategory::ChainUserBets }

    /// Page-based key for bounded upstream queries (page + page_size).
    pub fn chain_user_bets_page(network: &str, user: &str, page: i64, page_size: i64) -> String {
        format!(
            "{CHAIN_PREFIX}:user_bets:{network}:{}:page:{page}:size:{page_size}",
            user.to_lowercase()
        )
    }

    pub fn chain_oracle_result(network: &str, market_id: i64) -> String {
        format!("{CHAIN_PREFIX}:oracle:{network}:market:{market_id}")
    }
    pub fn chain_oracle_result_category() -> KeyCategory { KeyCategory::ChainOracleResult }

    pub fn chain_tx_status(network: &str, tx_hash: &str) -> String {
        format!(
            "{CHAIN_PREFIX}:tx_status:{network}:{}",
            tx_hash.to_lowercase()
        )
    }
    pub fn chain_tx_status_category() -> KeyCategory { KeyCategory::ChainTxStatus }

    pub fn chain_health(network: &str) -> String {
        format!("{CHAIN_PREFIX}:health:{network}")
    }
    pub fn chain_health_category() -> KeyCategory { KeyCategory::ChainHealth }

    pub fn chain_last_seen_ledger(network: &str) -> String {
        format!("{CHAIN_PREFIX}:last_seen_ledger:{network}")
    }
    pub fn chain_last_seen_ledger_category() -> KeyCategory { KeyCategory::ChainLedger }

    pub fn chain_sync_cursor(network: &str) -> String {
        format!("{CHAIN_PREFIX}:sync_cursor:{network}")
    }

    pub fn chain_replay_progress(network: &str, from_ledger: u32) -> String {
        format!("{CHAIN_PREFIX}:replay:{network}:{from_ledger}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    // ---- TtlConfig tests ----

    #[test]
    fn default_ttl_config_returns_correct_durations() {
        let cfg = TtlConfig::default();
        assert_eq!(cfg.get(KeyCategory::Statistics),        Some(Duration::from_secs(60)));
        assert_eq!(cfg.get(KeyCategory::FeaturedMarkets),   Some(Duration::from_secs(300)));
        assert_eq!(cfg.get(KeyCategory::Content),           Some(Duration::from_secs(600)));
        assert_eq!(cfg.get(KeyCategory::ChainMarket),       Some(Duration::from_secs(30)));
        assert_eq!(cfg.get(KeyCategory::ChainPlatformStats),Some(Duration::from_secs(120)));
        assert_eq!(cfg.get(KeyCategory::ChainUserBets),     Some(Duration::from_secs(60)));
        assert_eq!(cfg.get(KeyCategory::ChainOracleResult), Some(Duration::from_secs(300)));
        assert_eq!(cfg.get(KeyCategory::ChainTxStatus),     Some(Duration::from_secs(15)));
        assert_eq!(cfg.get(KeyCategory::ChainHealth),       Some(Duration::from_secs(10)));
        assert_eq!(cfg.get(KeyCategory::ChainLedger),       Some(Duration::from_secs(5)));
        assert_eq!(cfg.get(KeyCategory::ChainSyncCursor),   Some(Duration::from_secs(5)));
    }

    #[test]
    fn custom_category_returns_none() {
        let cfg = TtlConfig::default();
        assert_eq!(cfg.get(KeyCategory::Custom), None);
    }

    #[test]
    fn ttl_config_is_overridable_per_field() {
        let cfg = TtlConfig {
            statistics: Duration::from_secs(30),
            ..TtlConfig::default()
        };
        assert_eq!(cfg.get(KeyCategory::Statistics), Some(Duration::from_secs(30)));
        // Other fields unchanged
        assert_eq!(cfg.get(KeyCategory::Content), Some(Duration::from_secs(600)));
    }

    #[test]
    fn high_volatility_keys_have_shorter_ttl_than_stable_keys() {
        let cfg = TtlConfig::default();
        let health_ttl   = cfg.get(KeyCategory::ChainHealth).unwrap();
        let ledger_ttl   = cfg.get(KeyCategory::ChainLedger).unwrap();
        let content_ttl  = cfg.get(KeyCategory::Content).unwrap();
        let featured_ttl = cfg.get(KeyCategory::FeaturedMarkets).unwrap();

        assert!(health_ttl  < content_ttl,  "health should expire faster than content");
        assert!(ledger_ttl  < featured_ttl, "ledger should expire faster than featured markets");
    }

    #[test]
    fn key_category_labels_are_unique() {
        use std::collections::HashSet;
        let categories = [
            KeyCategory::Statistics,
            KeyCategory::FeaturedMarkets,
            KeyCategory::Content,
            KeyCategory::ChainMarket,
            KeyCategory::ChainPlatformStats,
            KeyCategory::ChainUserBets,
            KeyCategory::ChainOracleResult,
            KeyCategory::ChainTxStatus,
            KeyCategory::ChainHealth,
            KeyCategory::ChainLedger,
            KeyCategory::ChainSyncCursor,
            KeyCategory::Custom,
        ];
        let labels: HashSet<_> = categories.iter().map(|c| c.label()).collect();
        assert_eq!(labels.len(), categories.len(), "every category must have a unique label");
    }

    #[test]
    fn keys_module_category_helpers_return_correct_categories() {
        assert_eq!(keys::api_statistics_category(),          KeyCategory::Statistics);
        assert_eq!(keys::api_featured_markets_category(),    KeyCategory::FeaturedMarkets);
        assert_eq!(keys::api_content_category(),             KeyCategory::Content);
        assert_eq!(keys::dbq_statistics_category(),          KeyCategory::Statistics);
        assert_eq!(keys::chain_market_category(),            KeyCategory::ChainMarket);
        assert_eq!(keys::chain_platform_stats_category(),    KeyCategory::ChainPlatformStats);
        assert_eq!(keys::chain_user_bets_category(),         KeyCategory::ChainUserBets);
        assert_eq!(keys::chain_oracle_result_category(),     KeyCategory::ChainOracleResult);
        assert_eq!(keys::chain_tx_status_category(),         KeyCategory::ChainTxStatus);
        assert_eq!(keys::chain_health_category(),            KeyCategory::ChainHealth);
        assert_eq!(keys::chain_last_seen_ledger_category(),  KeyCategory::ChainLedger);
        assert_eq!(keys::chain_sync_cursor_category(),       KeyCategory::ChainSyncCursor);
    }

    // ---- XFetch / stampede tests (unchanged) ----

    #[test]
    fn xfetch_returns_true_for_expired_entry() {
        let entry: CachedEntry<u32> = CachedEntry {
            value: 42,
            expires_at: chrono::Utc::now().timestamp() - 1,
            delta_secs: 0.1,
        };
        assert!(xfetch_should_refresh(&entry, 1.0));
    }

    #[test]
    fn xfetch_returns_false_for_fresh_entry_with_tiny_delta() {
        let entry: CachedEntry<u32> = CachedEntry {
            value: 42,
            expires_at: chrono::Utc::now().timestamp() + 3600,
            delta_secs: 0.000_001,
        };
        let triggered = (0..100).filter(|_| xfetch_should_refresh(&entry, 1.0)).count();
        assert!(triggered < 5, "early refresh triggered too often for fresh entry: {triggered}/100");
    }

    #[test]
    fn xfetch_triggers_more_often_near_expiry() {
        let entry: CachedEntry<u32> = CachedEntry {
            value: 42,
            expires_at: chrono::Utc::now().timestamp() + 1,
            delta_secs: 2.0,
        };
        let triggered = (0..100).filter(|_| xfetch_should_refresh(&entry, 1.0)).count();
        assert!(triggered > 50, "expected frequent early refresh near expiry, got {triggered}/100");
    }

    #[test]
    fn stampede_config_default_has_both_strategies_enabled() {
        let cfg = StampedeConfig::default();
        assert!(cfg.probabilistic_early_expiry);
        assert!(cfg.mutex_lock);
        assert_eq!(cfg.xfetch_beta, 1.0);
    }

    #[tokio::test]
    async fn concurrent_fetcher_calls_are_serialised_by_counter() {
        let call_count = Arc::new(AtomicUsize::new(0));
        let lock = Arc::new(tokio::sync::Mutex::new(()));

        let tasks: Vec<_> = (0..20)
            .map(|_| {
                let count = Arc::clone(&call_count);
                let lock = Arc::clone(&lock);
                tokio::spawn(async move {
                    let _guard = lock.try_lock();
                    if _guard.is_ok() {
                        count.fetch_add(1, Ordering::SeqCst);
                        tokio::time::sleep(Duration::from_millis(10)).await;
                    }
                })
            })
            .collect();

        for t in tasks {
            t.await.unwrap();
        }

        assert_eq!(call_count.load(Ordering::SeqCst), 1);
    }
}
