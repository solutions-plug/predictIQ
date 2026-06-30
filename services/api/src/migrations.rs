//! Database migration runner with version tracking.
//!
//! Reads `*.sql` files from the `migrations/` directory embedded at compile
//! time, skips already-applied versions, and records each applied migration
//! in the `schema_migrations` table.
//!
//! # Usage
//!
//! ```rust
//! let runner = MigrationRunner::new(&pool);
//! runner.run().await?;          // apply pending migrations
//! runner.status().await?;       // print applied / pending state
//! ```

use anyhow::{bail, Context};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tracing::{info, warn};

/// A single migration file embedded at compile time.
#[derive(Debug, Clone)]
pub struct Migration {
    /// Numeric prefix extracted from the filename, e.g. `"001"`.
    pub version: &'static str,
    /// Full filename stem, e.g. `"001_enable_pgcrypto"`.
    pub name: &'static str,
    /// Raw SQL content.
    pub sql: &'static str,
}

/// Applied migration record stored in `schema_migrations`.
#[derive(Debug, sqlx::FromRow)]
pub struct AppliedMigration {
    pub version: String,
    pub name: String,
    pub applied_at: chrono::DateTime<chrono::Utc>,
    pub checksum: String,
}

// ---------------------------------------------------------------------------
// Embed all migration files at compile time so the binary is self-contained.
// Files are listed in ascending order — add new entries here as migrations grow.
// ---------------------------------------------------------------------------
const MIGRATIONS: &[Migration] = &[
    Migration {
        version: "000",
        name: "000_create_schema_migrations",
        sql: include_str!("../database/migrations/000_create_schema_migrations.sql"),
    },
    Migration {
        version: "001",
        name: "001_enable_pgcrypto",
        sql: include_str!("../database/migrations/001_enable_pgcrypto.sql"),
    },
    Migration {
        version: "002",
        name: "002_create_newsletter_subscriptions",
        sql: include_str!("../database/migrations/002_create_newsletter_subscriptions.sql"),
    },
    Migration {
        version: "003",
        name: "003_create_contact_form_submissions",
        sql: include_str!("../database/migrations/003_create_contact_form_submissions.sql"),
    },
    Migration {
        version: "004",
        name: "004_create_waitlist_entries",
        sql: include_str!("../database/migrations/004_create_waitlist_entries.sql"),
    },
    Migration {
        version: "005",
        name: "005_create_content_management",
        sql: include_str!("../database/migrations/005_create_content_management.sql"),
    },
    Migration {
        version: "006",
        name: "006_create_analytics_events",
        sql: include_str!("../database/migrations/006_create_analytics_events.sql"),
    },
    Migration {
        version: "007",
        name: "007_create_audit_logs",
        sql: include_str!("../database/migrations/007_create_audit_logs.sql"),
    },
    Migration {
        version: "008",
        name: "008_create_email_tracking",
        sql: include_str!("../database/migrations/008_create_email_tracking.sql"),
    },
    Migration {
        version: "011",
        name: "011_create_markets",
        sql: include_str!("../database/migrations/011_create_markets.sql"),
    },
    Migration {
        version: "017",
        name: "017_create_email_dead_letter_jobs",
        sql: include_str!("../database/migrations/017_create_email_dead_letter_jobs.sql"),
    },
];

// ---------------------------------------------------------------------------

pub struct MigrationRunner<'a> {
    pool: &'a PgPool,
}

impl<'a> MigrationRunner<'a> {
    pub fn new(pool: &'a PgPool) -> Self {
        Self { pool }
    }

    /// Ensure the tracking table exists, then apply every pending migration.
    /// Already-applied migrations are skipped. Returns the number of newly
    /// applied migrations.
    ///
    /// Uses a PostgreSQL session-level advisory lock to serialize concurrent
    /// invocations (e.g. multiple instances starting simultaneously). If another
    /// instance already holds the lock, this call aborts with an error so the
    /// caller can surface it and halt startup cleanly.
    pub async fn run(&self) -> anyhow::Result<usize> {
        self.ensure_tracking_table().await?;

        // Stable lock key — chosen to be unique to this codebase.
        const MIGRATION_LOCK_KEY: i64 = 0x7072_6564_6963_7471_u64 as i64;

        let mut lock_conn = self
            .pool
            .acquire()
            .await
            .context("acquire advisory lock connection")?;

        let locked: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(MIGRATION_LOCK_KEY)
            .fetch_one(&mut *lock_conn)
            .await
            .context("acquire migration advisory lock")?;

        if !locked {
            bail!(
                "another instance holds the migration advisory lock — \
                 aborting to prevent concurrent migration execution"
            );
        }

        let result = self.run_inner().await;

        // Always release the lock, even on failure, before the connection
        // returns to the pool (session-level locks survive pool reuse).
        let _ = sqlx::query("SELECT pg_advisory_unlock($1)")
            .bind(MIGRATION_LOCK_KEY)
            .execute(&mut *lock_conn)
            .await;

        result
    }

    async fn run_inner(&self) -> anyhow::Result<usize> {
        let mut applied = 0usize;

        for migration in MIGRATIONS {
            if self.is_applied(migration.version).await? {
                info!(version = migration.version, "migration already applied — skipping");
                continue;
            }

            self.apply(migration).await.with_context(|| {
                format!("failed to apply migration {}", migration.name)
            })?;

            applied += 1;
        }

        info!(applied, "migration run complete");
        Ok(applied)
    }

    /// Return the list of applied migrations from the tracking table.
    pub async fn status(&self) -> anyhow::Result<Vec<AppliedMigration>> {
        self.ensure_tracking_table().await?;

        let rows = sqlx::query_as::<_, AppliedMigration>(
            "SELECT version, name, applied_at, checksum
             FROM schema_migrations
             ORDER BY version ASC",
        )
        .fetch_all(self.pool)
        .await
        .context("failed to query schema_migrations")?;

        Ok(rows)
    }

    // -----------------------------------------------------------------------
    // Private helpers
    // -----------------------------------------------------------------------

    /// Create the tracking table if it does not exist yet.
    /// This is the only migration that is applied outside of the normal loop
    /// because it must exist before we can record anything.
    async fn ensure_tracking_table(&self) -> anyhow::Result<()> {
        let bootstrap = MIGRATIONS
            .iter()
            .find(|m| m.version == "000")
            .expect("000_create_schema_migrations must be present");

        sqlx::raw_sql(bootstrap.sql)
            .execute(self.pool)
            .await
            .context("failed to create schema_migrations table")?;

        Ok(())
    }

    async fn is_applied(&self, version: &str) -> anyhow::Result<bool> {
        let count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM schema_migrations WHERE version = $1",
        )
        .bind(version)
        .fetch_one(self.pool)
        .await
        .context("failed to check migration version")?;

        Ok(count > 0)
    }

    /// Apply a single migration inside a transaction and record it.
    async fn apply(&self, migration: &Migration) -> anyhow::Result<()> {
        let checksum = hex::encode(Sha256::digest(migration.sql.as_bytes()));

        info!(
            version = migration.version,
            name = migration.name,
            "applying migration"
        );

        let mut tx = self.pool.begin().await.context("begin transaction")?;

        // Execute the migration SQL
        sqlx::raw_sql(migration.sql)
            .execute(&mut *tx)
            .await
            .with_context(|| format!("SQL error in migration {}", migration.name))?;

        // Record in tracking table (skip version 000 — it IS the tracking table)
        if migration.version != "000" {
            sqlx::query(
                "INSERT INTO schema_migrations (version, name, applied_at, checksum)
                 VALUES ($1, $2, NOW(), $3)
                 ON CONFLICT (version) DO NOTHING",
            )
            .bind(migration.version)
            .bind(migration.name)
            .bind(&checksum)
            .execute(&mut *tx)
            .await
            .context("failed to record migration in schema_migrations")?;
        }

        tx.commit().await.context("commit transaction")?;

        info!(
            version = migration.version,
            checksum = &checksum[..12],
            "migration applied"
        );

        Ok(())
    }
}
