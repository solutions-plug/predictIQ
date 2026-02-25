use anyhow::{Context, Result};
use redis::AsyncCommands;
use serde_json::Value;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use crate::cache::RedisCache;
use crate::db::Database;
use crate::email::types::{EmailJobStatus, EmailJobType};

const EMAIL_QUEUE_KEY: &str = "email:queue";
const EMAIL_PROCESSING_KEY: &str = "email:processing";
const EMAIL_RETRY_KEY: &str = "email:retry";

#[derive(Clone)]
pub struct EmailQueue {
    cache: RedisCache,
    db: Database,
}

impl EmailQueue {
    pub fn new(cache: RedisCache, db: Database) -> Self {
        Self { cache, db }
    }

    /// Enqueue a new email job
    pub async fn enqueue(
        &self,
        job_type: EmailJobType,
        recipient: &str,
        template_name: &str,
        template_data: Value,
        priority: i32,
    ) -> Result<Uuid> {
        let job_id = self
            .db
            .email_create_job(
                job_type.as_str(),
                recipient,
                template_name,
                template_data,
                priority,
            )
            .await?;

        // Add to Redis queue for processing
        let score = if priority > 0 {
            // Higher priority = lower score (processed first)
            -(priority as f64)
        } else {
            chrono::Utc::now().timestamp() as f64
        };

        let mut conn = self.cache.manager.clone();
        let _: () = conn.zadd(EMAIL_QUEUE_KEY, job_id.to_string(), score)
            .await
            .context("Failed to add job to queue")?;

        tracing::info!("Enqueued email job: {} for {}", job_id, recipient);
        Ok(job_id)
    }

    /// Dequeue the next job for processing
    pub async fn dequeue(&self) -> Result<Option<Uuid>> {
        let mut conn = self.cache.manager.clone();

        // Use ZPOPMIN to atomically get and remove the lowest score item
        let result: Option<(String, f64)> = conn
            .zpopmin(EMAIL_QUEUE_KEY, 1)
            .await
            .context("Failed to dequeue job")?;

        if let Some((job_id_str, _score)) = result {
            let job_id = Uuid::parse_str(&job_id_str)?;

            // Add to processing set
            let _: () = conn.sadd(EMAIL_PROCESSING_KEY, job_id.to_string())
                .await
                .context("Failed to mark job as processing")?;

            return Ok(Some(job_id));
        }

        Ok(None)
    }

    /// Mark a job as completed
    pub async fn mark_completed(&self, job_id: Uuid, message_id: Option<String>) -> Result<()> {
        self.db
            .email_update_job_status(job_id, EmailJobStatus::Completed.as_str(), None)
            .await?;

        // Remove from processing set
        let mut conn = self.cache.manager.clone();
        let _: () = conn.srem(EMAIL_PROCESSING_KEY, job_id.to_string())
            .await
            .context("Failed to remove from processing set")?;

        // Track sent event
        if let Some(msg_id) = message_id {
            self.db
                .email_create_event(Some(job_id), Some(&msg_id), "sent", "", serde_json::json!({}))
                .await?;
        }

        tracing::info!("Marked email job as completed: {}", job_id);
        Ok(())
    }

    /// Mark a job as failed and schedule retry if attempts remain
    pub async fn mark_failed(&self, job_id: Uuid, error: &str) -> Result<()> {
        let job = self.db.email_get_job(job_id).await?;

        if let Some(job) = job {
            let new_attempts = job.attempts + 1;

            if new_attempts < job.max_attempts {
                // Schedule retry with exponential backoff
                let backoff_seconds = 2_u64.pow(new_attempts as u32) * 60; // 2min, 4min, 8min...
                let retry_at = chrono::Utc::now() + chrono::Duration::seconds(backoff_seconds as i64);

                self.db
                    .email_update_job_attempts(job_id, new_attempts, Some(error))
                    .await?;

                // Add to retry queue
                let mut conn = self.cache.manager.clone();
                let _: () = conn.zadd(
                    EMAIL_RETRY_KEY,
                    job_id.to_string(),
                    retry_at.timestamp() as f64,
                )
                .await
                .context("Failed to schedule retry")?;

                tracing::warn!(
                    "Email job {} failed (attempt {}/{}), retrying in {}s: {}",
                    job_id,
                    new_attempts,
                    job.max_attempts,
                    backoff_seconds,
                    error
                );
            } else {
                // Max attempts reached, mark as permanently failed
                self.db
                    .email_update_job_status(job_id, EmailJobStatus::Failed.as_str(), Some(error))
                    .await?;

                tracing::error!(
                    "Email job {} permanently failed after {} attempts: {}",
                    job_id,
                    new_attempts,
                    error
                );
            }

            // Remove from processing set
            let mut conn = self.cache.manager.clone();
            let _: () = conn.srem(EMAIL_PROCESSING_KEY, job_id.to_string())
                .await
                .context("Failed to remove from processing set")?;
        }

        Ok(())
    }

    /// Process retry queue - move jobs back to main queue if retry time has passed
    pub async fn process_retries(&self) -> Result<usize> {
        let mut conn = self.cache.manager.clone();
        let now = chrono::Utc::now().timestamp() as f64;

        // Get all jobs that are ready to retry
        let jobs: Vec<String> = conn
            .zrangebyscore(EMAIL_RETRY_KEY, "-inf", now)
            .await
            .context("Failed to get retry jobs")?;

        let count = jobs.len();

        for job_id_str in jobs {
            // Move back to main queue
            let job_id = Uuid::parse_str(&job_id_str)?;
            let _: () = conn.zadd(EMAIL_QUEUE_KEY, &job_id_str, now)
                .await
                .context("Failed to re-queue job")?;

            // Remove from retry queue
            let _: () = conn.zrem(EMAIL_RETRY_KEY, &job_id_str)
                .await
                .context("Failed to remove from retry queue")?;

            tracing::info!("Re-queued email job for retry: {}", job_id);
        }

        Ok(count)
    }

    /// Get queue statistics
    pub async fn get_stats(&self) -> Result<QueueStats> {
        let mut conn = self.cache.manager.clone();

        let pending: usize = conn
            .zcard(EMAIL_QUEUE_KEY)
            .await
            .context("Failed to get queue size")?;

        let processing: usize = conn
            .scard(EMAIL_PROCESSING_KEY)
            .await
            .context("Failed to get processing count")?;

        let retry: usize = conn
            .zcard(EMAIL_RETRY_KEY)
            .await
            .context("Failed to get retry count")?;

        Ok(QueueStats {
            pending,
            processing,
            retry,
        })
    }

    /// Background worker to process email queue
    pub async fn start_worker(&self, service: crate::email::EmailService) {
        tracing::info!("Starting email queue worker");

        loop {
            // Process retries first
            if let Err(e) = self.process_retries().await {
                tracing::error!("Error processing retries: {}", e);
            }

            // Process next job
            match self.dequeue().await {
                Ok(Some(job_id)) => {
                    if let Err(e) = self.process_job(job_id, &service).await {
                        tracing::error!("Error processing job {}: {}", job_id, e);
                        let _ = self.mark_failed(job_id, &e.to_string()).await;
                    }
                }
                Ok(None) => {
                    // No jobs available, sleep briefly
                    sleep(Duration::from_secs(1)).await;
                }
                Err(e) => {
                    tracing::error!("Error dequeuing job: {}", e);
                    sleep(Duration::from_secs(5)).await;
                }
            }
        }
    }

    async fn process_job(&self, job_id: Uuid, service: &crate::email::EmailService) -> Result<()> {
        let job = self
            .db
            .email_get_job(job_id)
            .await?
            .context("Job not found")?;

        // Check if email is suppressed
        if self.db.email_is_suppressed(&job.recipient_email).await? {
            tracing::warn!(
                "Skipping email to suppressed address: {}",
                job.recipient_email
            );
            return self.mark_completed(job_id, None).await;
        }

        // Update status to processing
        self.db
            .email_update_job_status(job_id, EmailJobStatus::Processing.as_str(), None)
            .await?;

        // Send email
        let message_id = service
            .send_email(
                &job.recipient_email,
                &job.template_name,
                &job.template_data,
            )
            .await?;

        // Mark as completed
        self.mark_completed(job_id, Some(message_id)).await?;

        Ok(())
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct QueueStats {
    pub pending: usize,
    pub processing: usize,
    pub retry: usize,
}
