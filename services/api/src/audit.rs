use std::net::IpAddr;

pub mod client_ip;
pub use client_ip::{extract_client_ip, trusted_cidrs_from_env};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLogEntry {
    pub id: Option<i64>,
    pub timestamp: DateTime<Utc>,
    pub actor: String,
    pub actor_ip: Option<IpAddr>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<String>,
    pub details: Option<serde_json::Value>,
    pub status: AuditStatus,
    pub error_message: Option<String>,
    pub request_id: Option<Uuid>,
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditStatus {
    Success,
    Failure,
    Partial,
}

impl std::fmt::Display for AuditStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuditStatus::Success => write!(f, "success"),
            AuditStatus::Failure => write!(f, "failure"),
            AuditStatus::Partial => write!(f, "partial"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct AuditLogger {
    pool: PgPool,
}

impl AuditLogger {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Log an admin operation to the audit log
    pub async fn log(&self, entry: AuditLogEntry) -> anyhow::Result<i64> {
        let id = sqlx::query_scalar::<_, i64>(
            r#"
            INSERT INTO audit_log (
                timestamp, actor, actor_ip, action, resource_type, resource_id,
                details, status, error_message, request_id, user_agent
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            RETURNING id
            "#,
        )
        .bind(entry.timestamp)
        .bind(&entry.actor)
        .bind(entry.actor_ip.map(|ip| ip.to_string()))
        .bind(&entry.action)
        .bind(&entry.resource_type)
        .bind(&entry.resource_id)
        .bind(&entry.details)
        .bind(entry.status.to_string())
        .bind(&entry.error_message)
        .bind(entry.request_id)
        .bind(&entry.user_agent)
        .fetch_one(&self.pool)
        .await?;

        tracing::info!(
            audit_id = id,
            actor = %entry.actor,
            action = %entry.action,
            resource_type = %entry.resource_type,
            resource_id = ?entry.resource_id,
            status = %entry.status,
            "Audit log entry created"
        );

        Ok(id)
    }

    /// Query audit log entries with filters
    pub async fn query(
        &self,
        actor: Option<&str>,
        action: Option<&str>,
        resource_type: Option<&str>,
        from: Option<DateTime<Utc>>,
        to: Option<DateTime<Utc>>,
        limit: i64,
        offset: i64,
    ) -> anyhow::Result<Vec<AuditLogEntry>> {
        let mut query = String::from(
            r#"
            SELECT id, timestamp, actor, actor_ip, action, resource_type, resource_id,
                   details, status, error_message, request_id, user_agent
            FROM audit_log
            WHERE 1=1
            "#,
        );

        let mut bind_count = 0;
        let mut bindings: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send>> = Vec::new();

        if let Some(a) = actor {
            bind_count += 1;
            query.push_str(&format!(" AND actor = ${}", bind_count));
        }

        if let Some(a) = action {
            bind_count += 1;
            query.push_str(&format!(" AND action = ${}", bind_count));
        }

        if let Some(rt) = resource_type {
            bind_count += 1;
            query.push_str(&format!(" AND resource_type = ${}", bind_count));
        }

        if let Some(f) = from {
            bind_count += 1;
            query.push_str(&format!(" AND timestamp >= ${}", bind_count));
        }

        if let Some(t) = to {
            bind_count += 1;
            query.push_str(&format!(" AND timestamp <= ${}", bind_count));
        }

        query.push_str(" ORDER BY timestamp DESC");
        
        bind_count += 1;
        query.push_str(&format!(" LIMIT ${}", bind_count));
        
        bind_count += 1;
        query.push_str(&format!(" OFFSET ${}", bind_count));

        let mut q = sqlx::query_as::<_, (
            i64,
            DateTime<Utc>,
            String,
            Option<String>,
            String,
            String,
            Option<String>,
            Option<serde_json::Value>,
            String,
            Option<String>,
            Option<Uuid>,
            Option<String>,
        )>(&query);

        if let Some(a) = actor {
            q = q.bind(a);
        }
        if let Some(a) = action {
            q = q.bind(a);
        }
        if let Some(rt) = resource_type {
            q = q.bind(rt);
        }
        if let Some(f) = from {
            q = q.bind(f);
        }
        if let Some(t) = to {
            q = q.bind(t);
        }
        q = q.bind(limit).bind(offset);

        let rows = q.fetch_all(&self.pool).await?;

        Ok(rows
            .into_iter()
            .map(
                |(
                    id,
                    timestamp,
                    actor,
                    actor_ip_str,
                    action,
                    resource_type,
                    resource_id,
                    details,
                    status,
                    error_message,
                    request_id,
                    user_agent,
                )| {
                    AuditLogEntry {
                        id: Some(id),
                        timestamp,
                        actor,
                        actor_ip: actor_ip_str.and_then(|s| s.parse().ok()),
                        action,
                        resource_type,
                        resource_id,
                        details,
                        status: match status.as_str() {
                            "success" => AuditStatus::Success,
                            "failure" => AuditStatus::Failure,
                            "partial" => AuditStatus::Partial,
                            _ => AuditStatus::Success,
                        },
                        error_message,
                        request_id,
                        user_agent,
                    }
                },
            )
            .collect())
    }

    /// Get audit log statistics
    pub async fn statistics(
        &self,
        from: DateTime<Utc>,
        to: DateTime<Utc>,
    ) -> anyhow::Result<AuditStatistics> {
        let row = sqlx::query_as::<_, (i64, i64, i64)>(
            r#"
            SELECT 
                COUNT(*) as total,
                COUNT(*) FILTER (WHERE status = 'success') as successful,
                COUNT(*) FILTER (WHERE status = 'failure') as failed
            FROM audit_log
            WHERE timestamp >= $1 AND timestamp <= $2
            "#,
        )
        .bind(from)
        .bind(to)
        .fetch_one(&self.pool)
        .await?;

        Ok(AuditStatistics {
            total: row.0,
            successful: row.1,
            failed: row.2,
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditStatistics {
    pub total: i64,
    pub successful: i64,
    pub failed: i64,
}

/// Helper to create audit log entry from request context
pub fn create_audit_entry(
    actor: String,
    actor_ip: Option<IpAddr>,
    action: String,
    resource_type: String,
    resource_id: Option<String>,
    details: Option<serde_json::Value>,
    request_id: Option<Uuid>,
    user_agent: Option<String>,
) -> AuditLogEntry {
    AuditLogEntry {
        id: None,
        timestamp: Utc::now(),
        actor,
        actor_ip,
        action,
        resource_type,
        resource_id,
        details,
        status: AuditStatus::Success,
        error_message: None,
        request_id,
        user_agent,
    }
}
