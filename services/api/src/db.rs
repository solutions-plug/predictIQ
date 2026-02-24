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

    // Waitlist management
    pub async fn waitlist_get_by_email(
        &self,
        normalized_email: &str,
    ) -> anyhow::Result<Option<crate::waitlist::WaitlistEntry>> {
        let row = sqlx::query(
            "SELECT id, email, name, role, status, source, referral_code, referred_by_code,
                    position, priority_score, joined_at, invited_at, invitation_accepted_at,
                    converted_at, created_at, updated_at
             FROM waitlist_entries
             WHERE email = $1",
        )
        .bind(normalized_email)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            return Ok(Some(crate::waitlist::WaitlistEntry {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                name: row.try_get("name")?,
                role: row.try_get("role")?,
                status: row.try_get("status")?,
                source: row.try_get("source")?,
                referral_code: row.try_get("referral_code")?,
                referred_by_code: row.try_get("referred_by_code")?,
                position: row.try_get("position")?,
                priority_score: row.try_get("priority_score")?,
                joined_at: row.try_get("joined_at")?,
                invited_at: row.try_get("invited_at")?,
                invitation_accepted_at: row.try_get("invitation_accepted_at")?,
                converted_at: row.try_get("converted_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            }));
        }

        Ok(None)
    }

    pub async fn waitlist_create_entry(
        &self,
        email: &str,
        name: Option<&str>,
        role: Option<&str>,
        source: Option<&str>,
        referral_code: &str,
        referred_by_code: Option<&str>,
    ) -> anyhow::Result<crate::waitlist::WaitlistEntry> {
        // Get current max position
        let position_row = sqlx::query(
            "SELECT COALESCE(MAX(position), 0) + 1 as next_position FROM waitlist_entries"
        )
        .fetch_one(&self.pool)
        .await?;
        let position: i32 = position_row.try_get("next_position")?;

        // Calculate priority score based on referral
        let mut priority_score = 0;
        if let Some(ref_code) = referred_by_code {
            // Increment referral count for referrer
            let _ = sqlx::query(
                "INSERT INTO waitlist_referrals (referrer_code, referral_count)
                 VALUES ($1, 1)
                 ON CONFLICT (referrer_code) DO UPDATE SET
                     referral_count = waitlist_referrals.referral_count + 1,
                     updated_at = NOW()"
            )
            .bind(ref_code)
            .execute(&self.pool)
            .await;

            priority_score = 10; // Bonus for being referred
        }

        let row = sqlx::query(
            "INSERT INTO waitlist_entries 
             (email, name, role, status, source, referral_code, referred_by_code, position, priority_score)
             VALUES ($1, $2, $3, 'pending', $4, $5, $6, $7, $8)
             RETURNING id, email, name, role, status, source, referral_code, referred_by_code,
                       position, priority_score, joined_at, invited_at, invitation_accepted_at,
                       converted_at, created_at, updated_at",
        )
        .bind(email)
        .bind(name)
        .bind(role)
        .bind(source)
        .bind(referral_code)
        .bind(referred_by_code)
        .bind(position)
        .bind(priority_score)
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::waitlist::WaitlistEntry {
            id: row.try_get("id")?,
            email: row.try_get("email")?,
            name: row.try_get("name")?,
            role: row.try_get("role")?,
            status: row.try_get("status")?,
            source: row.try_get("source")?,
            referral_code: row.try_get("referral_code")?,
            referred_by_code: row.try_get("referred_by_code")?,
            position: row.try_get("position")?,
            priority_score: row.try_get("priority_score")?,
            joined_at: row.try_get("joined_at")?,
            invited_at: row.try_get("invited_at")?,
            invitation_accepted_at: row.try_get("invitation_accepted_at")?,
            converted_at: row.try_get("converted_at")?,
            created_at: row.try_get("created_at")?,
            updated_at: row.try_get("updated_at")?,
        })
    }

    pub async fn waitlist_get_stats(&self) -> anyhow::Result<crate::waitlist::WaitlistStats> {
        let row = sqlx::query(
            "SELECT 
                COUNT(*)::BIGINT as total_entries,
                COUNT(*) FILTER (WHERE status = 'pending')::BIGINT as pending_entries,
                COUNT(*) FILTER (WHERE status = 'invited')::BIGINT as invited_entries,
                COUNT(*) FILTER (WHERE invitation_accepted_at IS NOT NULL)::BIGINT as accepted_entries,
                (SELECT COALESCE(SUM(referral_count), 0)::BIGINT FROM waitlist_referrals) as total_referrals
             FROM waitlist_entries"
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(crate::waitlist::WaitlistStats {
            total_entries: row.try_get("total_entries")?,
            pending_entries: row.try_get("pending_entries")?,
            invited_entries: row.try_get("invited_entries")?,
            accepted_entries: row.try_get("accepted_entries")?,
            total_referrals: row.try_get("total_referrals")?,
        })
    }

    pub async fn waitlist_get_all_for_export(&self) -> anyhow::Result<Vec<crate::waitlist::WaitlistExportEntry>> {
        let rows = sqlx::query(
            "SELECT 
                w.email, w.name, w.role, w.status, w.position, w.referral_code,
                w.joined_at, w.invited_at, w.invitation_accepted_at,
                COALESCE(r.referral_count, 0) as referral_count
             FROM waitlist_entries w
             LEFT JOIN waitlist_referrals r ON w.referral_code = r.referrer_code
             ORDER BY w.position ASC"
        )
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(crate::waitlist::WaitlistExportEntry {
                email: row.try_get("email")?,
                name: row.try_get("name")?,
                role: row.try_get("role")?,
                status: row.try_get("status")?,
                position: row.try_get("position")?,
                referral_code: row.try_get("referral_code")?,
                referral_count: row.try_get("referral_count")?,
                joined_at: row.try_get("joined_at")?,
                invited_at: row.try_get("invited_at")?,
                invitation_accepted_at: row.try_get("invitation_accepted_at")?,
            });
        }

        Ok(entries)
    }

    pub async fn waitlist_invite_by_positions(&self, positions: Vec<i32>) -> anyhow::Result<i32> {
        let result = sqlx::query(
            "UPDATE waitlist_entries
             SET status = 'invited', invited_at = NOW(), updated_at = NOW()
             WHERE position = ANY($1) AND status = 'pending'
             RETURNING id"
        )
        .bind(&positions)
        .fetch_all(&self.pool)
        .await?;

        Ok(result.len() as i32)
    }

    pub async fn waitlist_invite_top_n(&self, count: i32) -> anyhow::Result<Vec<crate::waitlist::WaitlistEntry>> {
        let rows = sqlx::query(
            "UPDATE waitlist_entries
             SET status = 'invited', invited_at = NOW(), updated_at = NOW()
             WHERE id IN (
                 SELECT id FROM waitlist_entries
                 WHERE status = 'pending'
                 ORDER BY priority_score DESC, position ASC
                 LIMIT $1
             )
             RETURNING id, email, name, role, status, source, referral_code, referred_by_code,
                       position, priority_score, joined_at, invited_at, invitation_accepted_at,
                       converted_at, created_at, updated_at"
        )
        .bind(count)
        .fetch_all(&self.pool)
        .await?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(crate::waitlist::WaitlistEntry {
                id: row.try_get("id")?,
                email: row.try_get("email")?,
                name: row.try_get("name")?,
                role: row.try_get("role")?,
                status: row.try_get("status")?,
                source: row.try_get("source")?,
                referral_code: row.try_get("referral_code")?,
                referred_by_code: row.try_get("referred_by_code")?,
                position: row.try_get("position")?,
                priority_score: row.try_get("priority_score")?,
                joined_at: row.try_get("joined_at")?,
                invited_at: row.try_get("invited_at")?,
                invitation_accepted_at: row.try_get("invitation_accepted_at")?,
                converted_at: row.try_get("converted_at")?,
                created_at: row.try_get("created_at")?,
                updated_at: row.try_get("updated_at")?,
            });
        }

        Ok(entries)
    }

    pub async fn waitlist_mark_invitation_accepted(&self, email: &str) -> anyhow::Result<bool> {
        let result = sqlx::query(
            "UPDATE waitlist_entries
             SET invitation_accepted_at = NOW(), updated_at = NOW()
             WHERE email = $1 AND status = 'invited' AND invitation_accepted_at IS NULL"
        )
        .bind(email)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() > 0)
    }

    pub async fn waitlist_get_referral_count(&self, referral_code: &str) -> anyhow::Result<i32> {
        let row = sqlx::query(
            "SELECT COALESCE(referral_count, 0) as count FROM waitlist_referrals WHERE referrer_code = $1"
        )
        .bind(referral_code)
        .fetch_optional(&self.pool)
        .await?;

        if let Some(row) = row {
            Ok(row.try_get("count")?)
        } else {
            Ok(0)
        }
    }
}
