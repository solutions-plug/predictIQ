use std::time::Duration;

use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

use crate::{
    cache::{keys, RedisCache},
    metrics::Metrics,
};

#[derive(Clone)]
pub struct Database {
    pool: PgPool,
    cache: RedisCache,
    metrics: Metrics,
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
}

impl Database {
    pub async fn new(
        database_url: &str,
        cache: RedisCache,
        metrics: Metrics,
    ) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(25)
            .min_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect(database_url)
            .await
            .context("failed to connect to postgres")?;

        Ok(Self {
            pool,
            cache,
            metrics,
        })
    }

    pub async fn statistics_cached(&self) -> anyhow::Result<Statistics> {
        let key = keys::dbq_statistics();
        let ttl = Duration::from_secs(5 * 60);
        let endpoint = "statistics";

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async {
                let row = sqlx::query(
                    "SELECT \
                        COUNT(*)::BIGINT AS total_markets, \
                        COUNT(*) FILTER (WHERE status = 'active')::BIGINT AS active_markets, \
                        COUNT(*) FILTER (WHERE status = 'resolved')::BIGINT AS resolved_markets, \
                        COALESCE(SUM(total_volume), 0)::DOUBLE PRECISION AS total_volume \
                    FROM markets",
                )
                .fetch_one(&self.pool)
                .await?;

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
                let rows = sqlx::query(
                    "SELECT id, title, total_volume, ends_at \
                    FROM markets \
                    WHERE status = 'active' \
                    ORDER BY total_volume DESC, ends_at ASC \
                    LIMIT $1",
                )
                .bind(limit)
                .fetch_all(&self.pool)
                .await?;

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

    pub async fn content_cached(&self, page: i64, page_size: i64) -> anyhow::Result<ContentPage> {
        let key = keys::dbq_content(page, page_size);
        let ttl = Duration::from_secs(60 * 60);
        let endpoint = "content";
        let offset = (page.saturating_sub(1)) * page_size;

        let (value, hit) = self
            .cache
            .get_or_set_json(&key, ttl, || async move {
                let total_row = sqlx::query(
                    "SELECT COUNT(*)::BIGINT AS total FROM content WHERE is_published = TRUE",
                )
                .fetch_one(&self.pool)
                .await?;
                let total = total_row.try_get::<i64, _>("total")?;

                let rows = sqlx::query(
                    "SELECT id, title, category, published_at \
                    FROM content \
                    WHERE is_published = TRUE \
                    ORDER BY published_at DESC \
                    LIMIT $1 OFFSET $2",
                )
                .bind(page_size)
                .bind(offset)
                .fetch_all(&self.pool)
                .await?;

                let mut items = Vec::with_capacity(rows.len());
                for row in rows {
                    items.push(ContentItem {
                        id: row.try_get::<i64, _>("id")?,
                        title: row.try_get::<String, _>("title")?,
                        category: row.try_get::<String, _>("category")?,
                        published_at: row.try_get::<DateTime<Utc>, _>("published_at")?,
                    });
                }

                Ok(ContentPage {
                    page,
                    page_size,
                    total,
                    items,
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

    pub async fn newsletter_get_by_email(
        &self,
        normalized_email: &str,
    ) -> anyhow::Result<Option<NewsletterSubscriber>> {
        let row = sqlx::query(
            "SELECT email, source, confirmed, confirmation_token, created_at, confirmed_at, unsubscribed_at
             FROM newsletter_subscribers
             WHERE email = $1",
        )
        .bind(normalized_email)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            return Ok(Some(NewsletterSubscriber {
                email: row.try_get::<String, _>("email")?,
                source: row.try_get::<String, _>("source")?,
                confirmed: row.try_get::<bool, _>("confirmed")?,
                confirmation_token: row.try_get::<Option<String>, _>("confirmation_token")?,
                created_at: row.try_get::<DateTime<Utc>, _>("created_at")?,
                confirmed_at: row.try_get::<Option<DateTime<Utc>>, _>("confirmed_at")?,
                unsubscribed_at: row.try_get::<Option<DateTime<Utc>>, _>("unsubscribed_at")?,
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
        sqlx::query(
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn newsletter_confirm_by_token(&self, token: &str) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "UPDATE newsletter_subscribers
             SET confirmed = TRUE, confirmation_token = NULL, confirmed_at = NOW(), unsubscribed_at = NULL
             WHERE confirmation_token = $1",
        )
        .bind(token)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn newsletter_unsubscribe(&self, normalized_email: &str) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "UPDATE newsletter_subscribers
             SET unsubscribed_at = NOW(), confirmed = FALSE
             WHERE email = $1",
        )
        .bind(normalized_email)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn newsletter_gdpr_delete(&self, normalized_email: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("DELETE FROM newsletter_subscribers WHERE email = $1")
            .bind(normalized_email)
            .execute(&self.pool)
            .await?;

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
        let row = sqlx::query(
            "INSERT INTO email_jobs (job_type, recipient_email, template_name, template_data, priority)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id",
        )
        .bind(job_type)
        .bind(recipient)
        .bind(template_name)
        .bind(template_data)
        .bind(priority)
        .fetch_one(&self.pool)
        .await?;

        Ok(row.try_get("id")?)
    }

    pub async fn email_get_job(&self, job_id: uuid::Uuid) -> anyhow::Result<Option<crate::email::EmailJob>> {
        let row = sqlx::query(
            "SELECT id, job_type, recipient_email, template_name, template_data, status, priority,
                    attempts, max_attempts, scheduled_at, started_at, completed_at, failed_at,
                    error_message, created_at, updated_at
             FROM email_jobs WHERE id = $1",
        )
        .bind(job_id)
        .fetch_optional(&self.pool)
        .await?;

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
        sqlx::query(
            "UPDATE email_jobs
             SET status = $2, error_message = $3, updated_at = NOW(),
                 completed_at = CASE WHEN $2 = 'completed' THEN NOW() ELSE completed_at END,
                 failed_at = CASE WHEN $2 = 'failed' THEN NOW() ELSE failed_at END
             WHERE id = $1",
        )
        .bind(job_id)
        .bind(status)
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn email_update_job_attempts(
        &self,
        job_id: uuid::Uuid,
        attempts: i32,
        error_message: Option<&str>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            "UPDATE email_jobs
             SET attempts = $2, error_message = $3, updated_at = NOW()
             WHERE id = $1",
        )
        .bind(job_id)
        .bind(attempts)
        .bind(error_message)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // Email event tracking
    pub async fn email_create_event(
        &self,
        job_id: Option<uuid::Uuid>,
        message_id: Option<&str>,
        event_type: &str,
        recipient: &str,
        metadata: serde_json::Value,
    ) -> anyhow::Result<uuid::Uuid> {
        let row = sqlx::query(
            "INSERT INTO email_events (email_job_id, message_id, event_type, recipient_email, metadata)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id",
        )
        .bind(job_id)
        .bind(message_id)
        .bind(event_type)
        .bind(recipient)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;

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
        sqlx::query(
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
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn email_is_suppressed(&self, email: &str) -> anyhow::Result<bool> {
        let row = sqlx::query("SELECT COUNT(*) as count FROM email_suppressions WHERE email = $1")
            .bind(email)
            .fetch_one(&self.pool)
            .await?;

        let count: i64 = row.try_get("count")?;
        Ok(count > 0)
    }

    pub async fn email_remove_suppression(&self, email: &str) -> anyhow::Result<bool> {
        let result = sqlx::query("DELETE FROM email_suppressions WHERE email = $1")
            .bind(email)
            .execute(&self.pool)
            .await?;

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

        sqlx::query(&query_str)
            .bind(template)
            .bind(today)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn email_get_analytics(
        &self,
        template_name: Option<&str>,
        days: i32,
    ) -> anyhow::Result<Vec<crate::email::EmailAnalytics>> {
        let start_date = chrono::Utc::now().date_naive() - chrono::Duration::days(days as i64);

        let query = if let Some(template) = template_name {
            sqlx::query(
                "SELECT template_name, variant_name, date, sent_count, delivered_count,
                        opened_count, clicked_count, bounced_count, complained_count, unsubscribed_count
                 FROM email_analytics
                 WHERE template_name = $1 AND date >= $2
                 ORDER BY date DESC",
            )
            .bind(template)
            .bind(start_date)
        } else {
            sqlx::query(
                "SELECT template_name, variant_name, date, sent_count, delivered_count,
                        opened_count, clicked_count, bounced_count, complained_count, unsubscribed_count
                 FROM email_analytics
                 WHERE date >= $1
                 ORDER BY date DESC",
            )
            .bind(start_date)
        };

        let rows = query.fetch_all(&self.pool).await?;

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
}
