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
}
