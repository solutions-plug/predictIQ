mod common;

use common::db_fixture::test_pool;

#[tokio::test]
async fn audit_filter_query_plans_use_indexes() {
    let pool = test_pool().await;
    let mut transaction = pool.begin().await.unwrap();

    sqlx::query("SET LOCAL enable_seqscan = off")
        .execute(&mut *transaction)
        .await
        .unwrap();

    let cases = [
        (
            "actor and time range",
            "SELECT id FROM audit_log WHERE actor = 'plan-test' AND timestamp >= TIMESTAMPTZ '2026-01-01 00:00:00+00' AND timestamp <= TIMESTAMPTZ '2026-02-01 00:00:00+00' ORDER BY timestamp DESC LIMIT 100",
            "idx_audit_log_actor_time",
        ),
        (
            "action and time range",
            "SELECT id FROM audit_log WHERE action = 'plan-test' AND timestamp >= TIMESTAMPTZ '2026-01-01 00:00:00+00' AND timestamp <= TIMESTAMPTZ '2026-02-01 00:00:00+00' ORDER BY timestamp DESC LIMIT 100",
            "idx_audit_log_action_time",
        ),
        (
            "resource type and time range",
            "SELECT id FROM audit_log WHERE resource_type = 'plan-test' AND timestamp >= TIMESTAMPTZ '2026-01-01 00:00:00+00' AND timestamp <= TIMESTAMPTZ '2026-02-01 00:00:00+00' ORDER BY timestamp DESC LIMIT 100",
            "idx_audit_log_resource_type_time",
        ),
        (
            "statistics time range",
            "SELECT COUNT(*) FILTER (WHERE status = 'success'), COUNT(*) FILTER (WHERE status = 'failure') FROM audit_log WHERE timestamp >= TIMESTAMPTZ '2026-01-01 00:00:00+00' AND timestamp <= TIMESTAMPTZ '2026-02-01 00:00:00+00'",
            "idx_audit_log_timestamp",
        ),
    ];

    for (description, query, expected_index) in cases {
        let plan = sqlx::query_scalar::<_, String>(&format!("EXPLAIN (COSTS OFF) {query}"))
            .fetch_all(&mut *transaction)
            .await
            .unwrap()
            .join("\n");

        assert!(
            !plan.contains("Seq Scan"),
            "{description} unexpectedly uses a sequential scan:\n{plan}"
        );
        assert!(
            plan.contains(expected_index),
            "{description} did not use {expected_index}:\n{plan}"
        );
    }
}