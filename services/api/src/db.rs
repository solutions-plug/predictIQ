use std::time::Duration;

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use tokio::time::error::Elapsed;

use crate::{
    cache::{keys, RedisCache},
    metrics::Metrics,
};

/// Errors that can be returned by [`Database`] methods.
#[derive(Debug)]
pub enum DbError {
    /// A query exceeded the per-operation timeout.
    Timeout,
    /// The connection pool had no connections available within the acquire timeout.
    PoolExhausted,
    /// A database constraint was violated (unique, foreign-key, not-null, check).
    /// The inner string is the database error message for logging.
    ConstraintViolation(String),
    /// Any other database error.
    Other(anyhow::Error),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DbError::Timeout => write!(f, "database query timed out"),
            DbError::PoolExhausted => write!(f, "database connection pool exhausted"),
            DbError::ConstraintViolation(msg) => {
                write!(f, "database constraint violation: {msg}")
            }
            DbError::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<sqlx::Error> for DbError {
    fn from(e: sqlx::Error) -> Self {
        match &e {
            sqlx::Error::PoolTimedOut => DbError::PoolExhausted,
            sqlx::Error::Database(db_err) => {
                // PostgreSQL constraint violation SQLSTATE codes start with "23"
                // (23000 integrity constraint, 23505 unique violation, etc.).
                if db_err.code().map(|c| c.starts_with("23")).unwrap_or(false) {
                    DbError::ConstraintViolation(db_err.message().to_string())
                } else {
                    DbError::Other(anyhow::Error::from(e))
                }
            }
            _ => DbError::Other(anyhow::Error::from(e)),
        }
    }
}

impl From<Elapsed> for DbError {
    fn from(_: Elapsed) -> Self {
        DbError::Timeout
    }
}

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
    cache: RedisCache,
    metrics: Metrics,
    query_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Statistics {
    pub total_markets: i64,
    pub active_markets: i64,
    pub resolved_markets: i64,
    pub total_volume: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturedMarket {
    pub id: i64,
    pub title: String,
    pub volume: f64,
    pub ends_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentItem {
    pub id: i64,
    pub title: String,
    pub category: String,
    pub published_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentPage {
    pub page: i64,
    pub page_size: i64,
    pub total: i64,
    pub items: Vec<ContentItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewsletterSubscriber {
    pub email: String,
    pub source: String,
    pub confirmed: bool,
    pub confirmation_token: Option<String>,
    pub created_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub unsubscribed_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl Database {
    pub fn pool(&self) -> PgPool {
        self.pool.clone()
    }

    /// Snapshot pool size/idle into Prometheus gauges.
    /// Call this just before rendering `/metrics` so the values are current.
    pub fn record_pool_metrics(&self) {
        self.metrics.record_pool_metrics(self.pool.size(), self.pool.num_idle());
    }

    pub async fn new(
        database_url: &str,
        cache: RedisCache,
        metrics: Metrics,
        pool_config: &crate::config::DbPoolConfig,
    ) -> anyhow::Result<Self> {
        let stmt_timeout_ms = pool_config.statement_timeout_ms;
        let lock_timeout_ms = pool_config.lock_timeout_ms;

        let mut builder = PgPoolOptions::new()
            .max_connections(pool_config.max_connections)
            .min_connections(pool_config.min_connections)
            .acquire_timeout(pool_config.acquire_timeout)
            .after_connect(move |conn, _meta| {
                Box::pin(async move {
                    sqlx::query(&format!("SET statement_timeout = {stmt_timeout_ms}"))
                        .execute(&mut *conn)
                        .await?;
                    sqlx::query(&format!("SET lock_timeout = {lock_timeout_ms}"))
                        .execute(&mut *conn)
                        .await?;
                    Ok(())
                })
            });

        if let Some(idle) = pool_config.idle_timeout {
            builder = builder.idle_timeout(idle);
        }
        if let Some(lifetime) = pool_config.max_lifetime {
            builder = builder.max_lifetime(lifetime);
        }

        let pool = builder
            .connect(database_url)
            .await
            .context("failed to connect to postgres")?;

        Ok(Self {
            pool,
            cache,
            metrics,
            query_timeout: pool_config.query_timeout,
        })
    }

    /// Run `fut` with the configured query timeout.
    /// On timeout, increments the `db_timeouts` metric and logs a warning.
    async fn with_timeout<F, T>(&self, operation: &str, fut: F) -> Result<T, DbError>
    where
        F: std::future::Future<Output = Result<T, sqlx::Error>>,
    {
        match tokio::time::timeout(self.query_timeout, fut).await {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(e)) => Err(DbError::Other(anyhow::Error::from(e))),
            Err(_elapsed) => {
                self.metrics.observe_db_timeout(operation);
                tracing::warn!(operation, timeout_secs = ?self.query_timeout, "db query timed out");
                Err(DbError::Timeout)
            }
        }
    }

    pub async fn statistics_cached(&self) -> anyhow::Result<Statistics> {
        let key = keys::dbq_statistics();
        let ttl = Duration::from_secs(5 * 60);
        let endpoint = "statistics";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async {
                let row = self.with_timeout("statistics", sqlx::query(
                    "SELECT \
                        COUNT(*)::BIGINT AS total_markets, \
                        COUNT(*) FILTER (WHERE status = 'active')::BIGINT AS active_markets, \
                        COUNT(*) FILTER (WHERE status = 'resolved')::BIGINT AS resolved_markets, \
                        COALESCE(SUM(total_volume), 0)::DOUBLE PRECISION AS total_volume \
                    FROM markets",
                )
                .fetch_one(&self.pool)).await.map_err(anyhow::Error::from)?;

                Ok(Statistics {
                    total_markets: row.try_get::<i64, _>("total_markets")?,
                    active_markets: row.try_get::<i64, _>("active_markets")?,
                    resolved_markets: row.try_get::<i64, _>("resolved_markets")?,
                    total_volume: row.try_get::<f64, _>("total_volume")?,
                })
            })
            .await?;

        if hit {
            self.metrics.observe_hit("db", endpoint);
        } else {
            self.metrics.observe_miss("db", endpoint);
        }
        Ok(value)
    }

    pub async fn featured_markets_cached(&self, limit: i64) -> anyhow::Result<Vec<FeaturedMarket>> {
        let key = keys::dbq_featured_markets(limit);
        let ttl = Duration::from_secs(2 * 60);
        let endpoint = "featured_markets";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let rows = self.with_timeout("featured_markets", sqlx::query(
                    "SELECT id, title, total_volume, ends_at \
                    FROM markets \
                    WHERE status = 'active' \
                    ORDER BY total_volume DESC, ends_at ASC \
                    LIMIT $1",
                )
                .bind(limit)
                .fetch_all(&self.pool)).await.map_err(anyhow::Error::from)?;

                let mut markets = Vec::with_capacity(rows.len());
                for row in rows {
                    markets.push(FeaturedMarket {
                        id: row.try_get::<i64, _>("id")?,
                        title: row.try_get::<String, _>("title")?,
                        volume: row.try_get::<f64, _>("total_volume")?,
                        ends_at: row.try_get::<DateTime<Utc>, _>("ends_at")?,
                    });
                }

                Ok(markets)
            })
            .await?;

        if hit {
            self.metrics.observe_hit("db", endpoint);
        } else {
            self.metrics.observe_miss("db", endpoint);
        }

        Ok(value)
    }

    pub async fn content_cached(&self, limit: i64) -> anyhow::Result<Vec<ContentItem>> {
        let key = keys::dbq_content(limit);
        let ttl = Duration::from_secs(60 * 60);
        let endpoint = "content";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let rows = self.with_timeout("content", sqlx::query(
                    "SELECT id, title, category, published_at \
                    FROM content \
                    WHERE is_published = TRUE \
                    ORDER BY published_at DESC \
                    LIMIT $1",
                )
                .bind(limit)
                .fetch_all(&self.pool)).await.map_err(anyhow::Error::from)?;

                let mut items = Vec::with_capacity(rows.len());
                for row in rows {
                    items.push(ContentItem {
                        id: row.try_get::<i64, _>("id")?,
                        title: row.try_get::<String, _>("title")?,
                        category: row.try_get::<String, _>("category")?,
                        published_at: row.try_get::<DateTime<Utc>, _>("published_at")?,
                    });
                }

                Ok(items)
            })
            .await?;

        if hit {
            self.metrics.observe_hit("db", endpoint);
        } else {
            self.metrics.observe_miss("db", endpoint);
        }

        Ok(value)
    }

    pub async fn newsletter_get_by_email(
        &self,
        normalized_email: &str,
    ) -> anyhow::Result<Option<NewsletterSubscriber>> {
        let row = self.with_timeout("newsletter_get_by_email", sqlx::query(
            "SELECT email, source, confirmed, confirmation_token, created_at, confirmed_at, unsubscribed_at, deleted_at
             FROM newsletter_subscribers
             WHERE email = $1 AND deleted_at IS NULL",
        )
        .bind(normalized_email)
        .fetch_optional(&self.pool)).await.map_err(anyhow::Error::from)?;

        if let Some(row) = row {
            return Ok(Some(NewsletterSubscriber {
                email: row.try_get::<String, _>("email")?,
                source: row.try_get::<String, _>("source")?,
                confirmed: row.try_get::<bool, _>("confirmed")?,
                confirmation_token: row.try_get::<Option<String>, _>("confirmation_token")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
                confirmed_at: row.try_get::<Option<DateTime<Utc>>, _>("confirmed_at")?,
                unsubscribed_at: row.try_get::<Option<DateTime<Utc>>, _>("unsubscribed_at")?,
                deleted_at: row.try_get::<Option<DateTime<Utc>>, _>("deleted_at")?,
            }));
        }

        Ok(None)
    }

    pub async fn newsletter_upsert_pending(
        &self,
        normalized_email: &str,
        source: &str,
        confirmation_token: &str,
    ) -> anyhow::Result<()> {
        self.with_timeout("newsletter_upsert_pending", sqlx::query(
            "INSERT INTO newsletter_subscribers (email, source, confirmed, confirmation_token, created_at, confirmed_at, unsubscribed_at)
             VALUES ($1, $2, FALSE, $3, NOW(), NULL, NULL)
             ON CONFLICT (email) DO UPDATE SET
                 source = EXCLUDED.source,
                 confirmed = FALSE,
                 confirmation_token = EXCLUDED.confirmation_token,
                 created_at = NOW(),
                 confirmed_at = NULL,
                 unsubscribed_at = NULL",
        )
        .bind(normalized_email)
        .bind(source)
        .bind(confirmation_token)
        .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(())
    }

    pub async fn newsletter_confirm_by_token(
        &self,
        token: &str,
        token_ttl_secs: u64,
    ) -> anyhow::Result<bool> {
        let result = self.with_timeout("newsletter_confirm_by_token", sqlx::query(
            "UPDATE newsletter_subscribers
             SET confirmed = TRUE, confirmation_token = NULL, confirmed_at = NOW(), unsubscribed_at = NULL
             WHERE confirmation_token = $1
               AND created_at > NOW() - ($2 || ' seconds')::INTERVAL",
        )
        .bind(token)
        .bind(token_ttl_secs as i64)
        .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(result.rows_affected() > 0)
    }

    /// Remove pending (unconfirmed) subscriptions whose token has expired.
    /// `batch_size` caps the number of rows deleted per call to prevent long
    /// table locks on large datasets; callers should loop until 0 rows are
    /// returned if they need to drain the full backlog.
    pub async fn newsletter_delete_expired_pending(
        &self,
        token_ttl_secs: u64,
        batch_size: u64,
    ) -> anyhow::Result<u64> {
        let result = self.with_timeout("newsletter_delete_expired_pending", sqlx::query(
            "DELETE FROM newsletter_subscribers
             WHERE id IN (
                 SELECT id FROM newsletter_subscribers
                 WHERE confirmed = FALSE
                   AND created_at <= NOW() - ($1 || ' seconds')::INTERVAL
                 LIMIT $2
             )",
        )
        .bind(token_ttl_secs as i64)
        .bind(batch_size as i64)
        .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(result.rows_affected())
    }

    pub async fn newsletter_unsubscribe(&self, normalized_email: &str) -> anyhow::Result<bool> {
        let result = self.with_timeout("newsletter_unsubscribe", sqlx::query(
            "UPDATE newsletter_subscribers
             SET unsubscribed_at = NOW(), confirmed = FALSE
             WHERE email = $1 AND deleted_at IS NULL",
        )
        .bind(normalized_email)
        .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn newsletter_soft_delete(&self, normalized_email: &str) -> anyhow::Result<bool> {
        let result = self.with_timeout("newsletter_soft_delete", sqlx::query(
            "UPDATE newsletter_subscribers
             SET deleted_at = NOW()
             WHERE email = $1 AND deleted_at IS NULL",
        )
        .bind(normalized_email)
        .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn newsletter_gdpr_delete(&self, normalized_email: &str) -> anyhow::Result<bool> {
        let result = self.with_timeout("newsletter_gdpr_delete", sqlx::query("DELETE FROM newsletter_subscribers WHERE email = $1")
            .bind(normalized_email)
            .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(result.rows_affected() > 0)
    }

    // Email job management
    pub async fn email_create_job(
        &self,
        job_type: &str,
        recipient: &str,
        template_name: &str,
        template_data: serde_json::Value,
        priority: i32,
    ) -> anyhow::Result<uuid::Uuid> {
        let row = self.with_timeout("email_create_job", sqlx::query(
            "INSERT INTO email_jobs (job_type, recipient_email, template_name, template_data, priority)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id",
        )
        .bind(job_type)
        .bind(recipient)
        .bind(template_name)
        .bind(template_data)
        .bind(priority)
        .fetch_one(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(row.try_get("id")?)
    }

    pub async fn email_get_job(&self, job_id: uuid::Uuid) -> anyhow::Result<Option<crate::email::EmailJob>> {
        let row = self.with_timeout("email_get_job", sqlx::query(
            "SELECT id, job_type, recipient_email, template_name, template_data, status, priority,
                    attempts, max_attempts, scheduled_at, started_at, completed_at, failed_at,
                    error_message, created_at, updated_at
             FROM email_jobs WHERE id = $1",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)).await.map_err(anyhow::Error::from)?;

        if let Some(row) = row {
            return Ok(Some(crate::email::EmailJob {
                id: row.try_get("id")?,
                job_type: row.try_get("job_type")?,
                recipient_email: row.try_get("recipient_email")?,
                template_name: row.try_get("template_name")?,
                template_data: row.try_get("template_data")?,
                status: row.try_get("status")?,
                priority: row.try_get("priority")?,
                attempts: row.try_get("attempts")?,
                max_attempts: row.try_get("max_attempts")?,
                scheduled_at: row.try_get("scheduled_at")?,
                started_at: row.try_get("started_at")?,
                completed_at: row.try_get("completed_at")?,
                failed_at: row.try_get("failed_at")?,
                error_message: row.try_get("error_message")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }));
        }

        Ok(None)
    }

    pub async fn email_update_job_status(
        &self,
        job_id: uuid::Uuid,
        status: &str,
        error_message: Option<&str>,
    ) -> anyhow::Result<()> {
        self.with_timeout("email_update_job_status", sqlx::query(
            "UPDATE email_jobs
             SET status = $2, error_message = $3, updated_at = NOW(),
                 completed_at = CASE WHEN $2 = 'completed' THEN NOW() ELSE completed_at END,
                 failed_at = CASE WHEN $2 = 'failed' THEN NOW() ELSE failed_at END
             WHERE id = $1",
        )
        .bind(job_id)
        .bind(status)
        .bind(error_message)
        .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(())
    }

    pub async fn email_update_job_attempts(
        &self,
        job_id: uuid::Uuid,
        attempts: i32,
        error_message: Option<&str>,
    ) -> anyhow::Result<()> {
        self.with_timeout("email_update_job_attempts", sqlx::query(
            "UPDATE email_jobs
             SET attempts = $2, error_message = $3, updated_at = NOW()
             WHERE id = $1",
        )
        .bind(job_id)
        .bind(attempts)
        .bind(error_message)
        .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(())
    }

    // Email event tracking
    /// Create an email event record.
    ///
    /// ## PII Considerations
    ///
    /// The `recipient` field contains a personally identifiable email address.
    /// This is stored for analytics purposes: to correlate events with recipients,
    /// calculate delivery success rates, and track engagement per email domain.
    ///
    /// Important:
    /// - The email_events table should be protected from unauthorized access
    /// - Analytics queries should anonymize or filter recipient data for reports
    /// - Deletion policies must align with privacy regulations (GDPR, etc)
    /// - Consider hashing or tokenizing recipient emails in analytics aggregates
    pub async fn email_create_event(
        &self,
        job_id: Option<uuid::Uuid>,
        message_id: Option<&str>,
        event_type: &str,
        recipient: &str,
        metadata: serde_json::Value,
    ) -> anyhow::Result<uuid::Uuid> {
        let row = self.with_timeout("email_create_event", sqlx::query(
            "INSERT INTO email_events (email_job_id, message_id, event_type, recipient_email, metadata)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id",
        )
        .bind(job_id)
        .bind(message_id)
        .bind(event_type)
        .bind(recipient)
        .bind(metadata)
        .fetch_one(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(row.try_get("id")?)
    }

    // Email suppression management
    pub async fn email_add_suppression(
        &self,
        email: &str,
        suppression_type: &str,
        reason: Option<&str>,
        bounce_type: Option<&str>,
    ) -> anyhow::Result<()> {
        self.with_timeout("email_add_suppression", sqlx::query(
            "INSERT INTO email_suppressions (email, suppression_type, reason, bounce_type)
             VALUES ($1, $2, $3, $4)
             ON CONFLICT (email) DO UPDATE SET
                 suppression_type = EXCLUDED.suppression_type,
                 reason = EXCLUDED.reason,
                 bounce_type = EXCLUDED.bounce_type,
                 updated_at = NOW()",
        )
        .bind(email)
        .bind(suppression_type)
        .bind(reason)
        .bind(bounce_type)
        .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(())
    }

    pub async fn email_is_suppressed(&self, email: &str) -> anyhow::Result<bool> {
        let row = self.with_timeout("email_is_suppressed", sqlx::query("SELECT COUNT(*) as count FROM email_suppressions WHERE email = $1")
            .bind(email)
            .fetch_one(&self.pool)).await.map_err(anyhow::Error::from)?;

        let count: i64 = row.try_get("count")?;
        Ok(count > 0)
    }

    pub async fn email_remove_suppression(&self, email: &str) -> anyhow::Result<bool> {
        let result = self.with_timeout("email_remove_suppression", sqlx::query("DELETE FROM email_suppressions WHERE email = $1")
            .bind(email)
            .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(result.rows_affected() > 0)
    }

    // Email analytics
    pub async fn email_increment_analytics_counter(
        &self,
        counter_type: &str,
        template_name: Option<&str>,
    ) -> anyhow::Result<()> {
        let template = template_name.unwrap_or("unknown");
        let today = chrono::Utc::now().date_naive();

        let column = match counter_type {
            "sent" => "sent_count",
            "delivered" => "delivered_count",
            "opened" => "opened_count",
            "clicked" => "clicked_count",
            "bounced" => "bounced_count",
            "complained" => "complained_count",
            "unsubscribed" => "unsubscribed_count",
            _ => return Ok(()),
        };

        let query_str = format!(
            "INSERT INTO email_analytics (template_name, date, {})
             VALUES ($1, $2, 1)
             ON CONFLICT (template_name, variant_name, date) DO UPDATE SET
                 {} = email_analytics.{} + 1,
                 updated_at = NOW()",
            column, column, column
        );

        self.with_timeout("email_increment_analytics_counter", sqlx::query(&query_str)
            .bind(template)
            .bind(today)
            .execute(&self.pool)).await.map_err(anyhow::Error::from)?;

        Ok(())
    }

    pub async fn email_get_analytics(
        &self,
        template_name: Option<&str>,
        days: i32,
    ) -> anyhow::Result<Vec<crate::email::EmailAnalytics>> {
        let start_date = chrono::Utc::now().date_naive() - chrono::Duration::days(days as i64);

        let rows = if let Some(template) = template_name {
            self.with_timeout("email_get_analytics", sqlx::query(
                "SELECT template_name, variant_name, date, sent_count, delivered_count,
                        opened_count, clicked_count, bounced_count, complained_count, unsubscribed_count
                 FROM email_analytics
                 WHERE template_name = $1 AND date >= $2
                 ORDER BY date DESC",
            )
            .bind(template)
            .bind(start_date)
            .fetch_all(&self.pool)).await.map_err(anyhow::Error::from)?
        } else {
            self.with_timeout("email_get_analytics", sqlx::query(
                "SELECT template_name, variant_name, date, sent_count, delivered_count,
                        opened_count, clicked_count, bounced_count, complained_count, unsubscribed_count
                 FROM email_analytics
                 WHERE date >= $1
                 ORDER BY date DESC",
            )
            .bind(start_date)
            .fetch_all(&self.pool)).await.map_err(anyhow::Error::from)?
        };

        let mut analytics = Vec::new();
        for row in rows {
            analytics.push(crate::email::EmailAnalytics {
                template_name: row.try_get("template_name")?,
                variant_name: row.try_get("variant_name")?,
                date: row.try_get("date")?,
                sent_count: row.try_get("sent_count")?,
                delivered_count: row.try_get("delivered_count")?,
                opened_count: row.try_get("opened_count")?,
                clicked_count: row.try_get("clicked_count")?,
                bounced_count: row.try_get("bounced_count")?,
                complained_count: row.try_get("complained_count")?,
                unsubscribed_count: row.try_get("unsubscribed_count")?,
            });
        }

        Ok(analytics)
    }

    /// Resolve a market by persisting the winning outcome to the database.
    ///
    /// Returns an error if the market does not exist or is not in `active` status.
    pub async fn resolve_market(&self, market_id: i64, outcome_index: u32) -> anyhow::Result<()> {
        let rows_affected = self
            .with_timeout(
                "resolve_market",
                sqlx::query(
                    "UPDATE markets \
                     SET status = 'resolved', outcome_index = $1, resolved_at = NOW() \
                     WHERE id = $2 AND status = 'active'",
                )
                .bind(outcome_index as i32)
                .bind(market_id)
                .execute(&self.pool),
            )
            .await
            .map_err(anyhow::Error::from)?
            .rows_affected();

        if rows_affected == 0 {
            anyhow::bail!("market {market_id} not found or not in active status");
        }

        Ok(())
    }

    /// Ping the database with a bounded timeout. Returns Ok(()) if reachable.
    pub async fn ping(&self) -> anyhow::Result<()> {
        tokio::time::timeout(
            Duration::from_secs(2),
            sqlx::query("SELECT 1").execute(&self.pool),
        )
        .await
        .context("db ping timed out")?
        .context("db ping failed")?;
        Ok(())
    }

    /// Check whether an email event already exists (replay-attack guard).
    pub async fn email_event_exists(
        &self,
        message_id: Option<&str>,
        event_type: &str,
        email: &str,
        timestamp: i64,
    ) -> anyhow::Result<bool> {
        let count: i64 = self.with_timeout("email_event_exists", sqlx::query_scalar(
            "SELECT COUNT(*) FROM email_events
             WHERE message_id IS NOT DISTINCT FROM $1
               AND event_type = $2
               AND recipient_email = $3
               AND created_at = to_timestamp($4)",
        )
        .bind(message_id)
        .bind(event_type)
        .bind(email)
        .bind(timestamp as f64)
        .fetch_one(&self.pool)).await.unwrap_or(0);
        Ok(count > 0)
    }
}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_error_timeout_display() {
        let e = DbError::Timeout;
        assert_eq!(e.to_string(), "database query timed out");
    }

    #[test]
    fn db_error_pool_exhausted_display() {
        let e = DbError::PoolExhausted;
        assert_eq!(e.to_string(), "database connection pool exhausted");
    }

    #[test]
    fn db_error_constraint_violation_display() {
        let e = DbError::ConstraintViolation("duplicate key value".to_string());
        assert!(e.to_string().contains("constraint violation"));
        assert!(e.to_string().contains("duplicate key value"));
    }

    #[test]
    fn from_sqlx_pool_timed_out_maps_to_pool_exhausted() {
        let e = DbError::from(sqlx::Error::PoolTimedOut);
        assert!(matches!(e, DbError::PoolExhausted));
    }

    #[test]
    fn from_sqlx_other_maps_to_other() {
        let e = DbError::from(sqlx::Error::RowNotFound);
        assert!(matches!(e, DbError::Other(_)));
    }
}
