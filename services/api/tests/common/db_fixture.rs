//! Database test fixture — transaction-scoped rollback.
//!
//! Each call to [`with_test_transaction`] opens a Postgres transaction,
//! runs the test closure, then **rolls back** unconditionally so no test
//! leaves rows that can affect subsequent tests regardless of execution order.
//!
//! # Requirements
//!
//! Set `TEST_DATABASE_URL` before running integration tests:
//!
//! ```bash
//! TEST_DATABASE_URL=postgres://predictiq_test:predictiq_test@localhost:5433/predictiq_test \
//!   cargo test --test '*' -- --test-threads=1
//! ```
//!
//! The easiest way to start the required services is:
//!
//! ```bash
//! make test-integration
//! ```

use std::future::Future;
use sqlx::{postgres::PgPoolOptions, PgPool, Postgres, Transaction};

/// Return a connection pool backed by `TEST_DATABASE_URL`.
///
/// The pool is intentionally small (max 5 connections) so parallel test
/// runs surface connection-exhaustion issues early.
pub async fn test_pool() -> PgPool {
    let url = std::env::var("TEST_DATABASE_URL").expect(
        "TEST_DATABASE_URL must be set to run database integration tests. \
         Start the test stack with `make test-integration` or \
         `docker compose -f docker-compose.test.yml up -d --wait`.",
    );
    PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .expect("Failed to connect to test database")
}

/// Run `f` inside a Postgres transaction that is **always rolled back**.
///
/// The closure receives a mutable reference to the transaction, which
/// implements `sqlx::Executor` so you can pass `&mut *conn` directly to
/// any `sqlx::query` call.
///
/// # Example
///
/// ```rust,no_run
/// # use common::db_fixture::{test_pool, with_test_transaction};
/// #[tokio::test]
/// async fn inserts_are_isolated() {
///     let pool = test_pool().await;
///     with_test_transaction(&pool, |mut conn| async move {
///         sqlx::query("INSERT INTO users (email) VALUES ('test@example.com')")
///             .execute(&mut *conn)
///             .await
///             .unwrap();
///         let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
///             .fetch_one(&mut *conn)
///             .await
///             .unwrap();
///         assert!(row.0 > 0);
///     })
///     .await;
///     // The INSERT above is never committed — the next test starts clean.
/// }
/// ```
pub async fn with_test_transaction<F, Fut>(pool: &PgPool, f: F)
where
    F: FnOnce(Transaction<'static, Postgres>) -> Fut,
    Fut: Future<Output = ()>,
{
    let tx = pool
        .begin()
        .await
        .expect("Failed to begin test transaction");

    f(tx).await;
    // Dropping the transaction without calling .commit() triggers an implicit
    // ROLLBACK — SQLx guarantees this on Drop for PgTransaction.
}

/// Truncate a list of tables between tests as an alternative to transaction
/// rollback (useful when the code under test manages its own transactions).
///
/// Tables are truncated in the given order with `TRUNCATE … RESTART IDENTITY
/// CASCADE` so foreign-key constraints are satisfied.
pub async fn truncate_tables(pool: &PgPool, tables: &[&str]) {
    for table in tables {
        sqlx::query(&format!(
            "TRUNCATE TABLE {table} RESTART IDENTITY CASCADE"
        ))
        .execute(pool)
        .await
        .unwrap_or_else(|e| panic!("Failed to truncate {table}: {e}"));
    }
}
