use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row};
use std::net::IpAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{interval, Duration};

use super::types::*;

const BATCH_SIZE: usize = 100;
const FLUSH_INTERVAL_SECS: u64 = 5;

#[derive(Clone)]
pub struct AnalyticsService {
    pool: PgPool,
    batch_buffer: Arc<Mutex<Vec<BatchEvent>>>,
}

#[derive(Debug, Clone)]
struct BatchEvent {
    time: DateTime<Utc>,
    event_type: String,
    event_data: serde_json::Value,
    session_id: String,
    anonymized_ip: Option<String>,
    user_agent: Option<String>,
    page_url: Option<String>,
    referrer: Option<String>,
}

impl AnalyticsService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            batch_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Anonymize IP address for privacy compliance
    fn anonymize_ip(ip: &str) -> String {
        if let Ok(addr) = ip.parse::<IpAddr>() {
            match addr {
                IpAddr::V4(ipv4) => {
                    let octets = ipv4.octets();
                    format!("{}.{}.{}.0", octets[0], octets[1], octets[2])
                }
                IpAddr::V6(ipv6) => {
                    let segments = ipv6.segments();
                    format!(
                        "{:x}:{:x}:{:x}:{:x}:0:0:0:0",
                        segments[0], segments[1], segments[2], segments[3]
                    )
                }
            }
        } else {
            "unknown".to_string()
        }
    }

    /// Add event to batch buffer
    pub async fn queue_event(
        &self,
        time: DateTime<Utc>,
        event_type: String,
        event_data: serde_json::Value,
        session_id: String,
        ip_address: Option<String>,
        user_agent: Option<String>,
        page_url: Option<String>,
        referrer: Option<String>,
    ) -> Result<()> {
        let anonymized_ip = ip_address.map(|ip| Self::anonymize_ip(&ip));

        let mut buffer = self.batch_buffer.lock().await;
        buffer.push(BatchEvent {
            time,
            event_type,
            event_data,
            session_id,
            anonymized_ip,
            user_agent,
            page_url,
            referrer,
        });

        // Auto-flush if buffer is full
        if buffer.len() >= BATCH_SIZE {
            drop(buffer);
            self.flush_batch().await?;
        }

        Ok(())
    }

    /// Flush batch buffer to database
    pub async fn flush_batch(&self) -> Result<usize> {
        let mut buffer = self.batch_buffer.lock().await;
        if buffer.is_empty() {
            return Ok(0);
        }

        let events: Vec<BatchEvent> = buffer.drain(..).collect();
        drop(buffer);

        let count = events.len();

        for event in events {
            sqlx::query(
                "INSERT INTO analytics_events 
                 (time, event_type, event_data, session_id, anonymized_ip, user_agent, page_url, referrer)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
            )
            .bind(event.time)
            .bind(event.event_type)
            .bind(event.event_data)
            .bind(event.session_id)
            .bind(event.anonymized_ip)
            .bind(event.user_agent)
            .bind(event.page_url)
            .bind(event.referrer)
            .execute(&self.pool)
            .await
            .context("failed to insert analytics event")?;
        }

        tracing::info!("Flushed {} analytics events to database", count);
        Ok(count)
    }

    /// Start background worker to periodically flush buffer
    pub fn start_background_worker(self) {
        tokio::spawn(async move {
            let mut ticker = interval(Duration::from_secs(FLUSH_INTERVAL_SECS));
            loop {
                ticker.tick().await;
                if let Err(e) = self.flush_batch().await {
                    tracing::error!("Background flush failed: {}", e);
                }
            }
        });
    }

    /// Get dashboard statistics
    pub async fn get_dashboard_stats(
        &self,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Result<DashboardStats> {
        // Total events
        let total_row = sqlx::query(
            "SELECT COUNT(*)::BIGINT as count FROM analytics_events 
             WHERE time >= $1 AND time <= $2",
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await?;
        let total_events: i64 = total_row.try_get("count")?;

        // Unique sessions
        let sessions_row = sqlx::query(
            "SELECT COUNT(DISTINCT session_id)::BIGINT as count FROM analytics_events 
             WHERE time >= $1 AND time <= $2",
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_one(&self.pool)
        .await?;
        let unique_sessions: i64 = sessions_row.try_get("count")?;

        // Events by type
        let type_rows = sqlx::query(
            "SELECT event_type, COUNT(*)::BIGINT as count FROM analytics_events 
             WHERE time >= $1 AND time <= $2
             GROUP BY event_type ORDER BY count DESC",
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await?;

        let mut events_by_type = Vec::new();
        for row in type_rows {
            events_by_type.push(EventTypeCount {
                event_type: row.try_get("event_type")?,
                count: row.try_get("count")?,
            });
        }

        // Hourly events from continuous aggregate
        let hourly_rows = sqlx::query(
            "SELECT bucket as hour, SUM(event_count)::BIGINT as count 
             FROM analytics_hourly 
             WHERE bucket >= $1 AND bucket <= $2
             GROUP BY bucket ORDER BY bucket DESC
             LIMIT 24",
        )
        .bind(start_time)
        .bind(end_time)
        .fetch_all(&self.pool)
        .await?;

        let mut hourly_events = Vec::new();
        for row in hourly_rows {
            hourly_events.push(HourlyCount {
                hour: row.try_get("hour")?,
                count: row.try_get("count")?,
            });
        }

        Ok(DashboardStats {
            total_events,
            unique_sessions,
            events_by_type,
            hourly_events,
        })
    }
}
