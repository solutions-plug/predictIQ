#[cfg(test)]
mod resolve_market_tests {
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::post,
        Router,
    };
    use serde_json::json;
    use std::sync::Arc;
    use tower::ServiceExt;

    use crate::handlers::{resolve_market, InvalidationResult};

    // ---------------------------------------------------------------------------
    // Helpers
    // ---------------------------------------------------------------------------

    /// Build a minimal router wired to `resolve_market` for handler-level tests.
    fn app(state: Arc<crate::AppState>) -> Router {
        Router::new()
            .route("/admin/markets/:market_id/resolve", post(resolve_market))
            .with_state(state)
    }

    async fn post_resolve(
        router: Router,
        market_id: i64,
        outcome_index: u32,
    ) -> axum::response::Response {
        let body = serde_json::to_vec(&json!({ "outcome_index": outcome_index })).unwrap();
        router
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/admin/markets/{market_id}/resolve"))
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap()
    }

    // ---------------------------------------------------------------------------
    // Unit tests — no real DB/Redis required
    // ---------------------------------------------------------------------------

    /// Verifies that a successful DB write returns 200 and a non-zero
    /// `invalidated_keys` count.
    #[tokio::test]
    #[ignore] // Requires PostgreSQL + Redis
    async fn test_resolve_market_success_returns_200_and_invalidation_count() {
        let state = build_test_state().await;
        // Insert a test market first so the UPDATE finds a row.
        sqlx::query(
            "INSERT INTO markets (id, title, status, total_volume, ends_at) \
             VALUES (9001, 'Test Market', 'active', 0, NOW() + INTERVAL '1 day')",
        )
        .execute(state.db.pool())
        .await
        .unwrap();

        let response = post_resolve(app(Arc::clone(&state)), 9001, 0).await;
        assert_eq!(response.status(), StatusCode::OK);

        let body: InvalidationResult =
            serde_json::from_slice(&axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert!(body.invalidated_keys > 0);

        // Cleanup
        sqlx::query("DELETE FROM markets WHERE id = 9001")
            .execute(state.db.pool())
            .await
            .unwrap();
    }

    /// Verifies that resolving a non-existent market returns 500.
    #[tokio::test]
    #[ignore] // Requires PostgreSQL + Redis
    async fn test_resolve_market_not_found_returns_500() {
        let state = build_test_state().await;
        let response = post_resolve(app(Arc::clone(&state)), 999_999_999, 0).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    /// Verifies that resolving an already-resolved market returns 500.
    #[tokio::test]
    #[ignore] // Requires PostgreSQL + Redis
    async fn test_resolve_market_already_resolved_returns_500() {
        let state = build_test_state().await;
        sqlx::query(
            "INSERT INTO markets (id, title, status, outcome_index, total_volume, ends_at, resolved_at) \
             VALUES (9002, 'Resolved Market', 'resolved', 1, 0, NOW() - INTERVAL '1 day', NOW())",
        )
        .execute(state.db.pool())
        .await
        .unwrap();

        let response = post_resolve(app(Arc::clone(&state)), 9002, 0).await;
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

        // Cleanup
        sqlx::query("DELETE FROM markets WHERE id = 9002")
            .execute(state.db.pool())
            .await
            .unwrap();
    }

    // ---------------------------------------------------------------------------
    // Pure-logic unit tests (no I/O)
    // ---------------------------------------------------------------------------

    /// Verifies that `ResolveMarketRequest` deserialises correctly.
    #[test]
    fn test_resolve_market_request_deserialises() {
        let json = r#"{"outcome_index": 2}"#;
        let req: crate::handlers::ResolveMarketRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.outcome_index, 2);
    }

    /// Verifies that `InvalidationResult` serialises correctly.
    #[test]
    fn test_invalidation_result_serialises() {
        let result = InvalidationResult { invalidated_keys: 6 };
        let json = serde_json::to_value(&result).unwrap();
        assert_eq!(json["invalidated_keys"], 6);
    }

    // ---------------------------------------------------------------------------
    // Helper — builds AppState from env (used by #[ignore] integration tests)
    // ---------------------------------------------------------------------------

    #[cfg(test)]
    async fn build_test_state() -> Arc<crate::AppState> {
        use crate::{
            audit::AuditLogger,
            blockchain::BlockchainClient,
            cache::RedisCache,
            config::Config,
            db::Database,
            email::{queue::EmailQueue, service::EmailService, webhook::WebhookHandler},
            metrics::Metrics,
            newsletter::IpRateLimiter,
        };

        let config = Config::from_env();
        let metrics = Metrics::new().expect("metrics");
        let cache = RedisCache::new(&config.redis_url).await.expect("redis");
        let db = Database::new(&config.database_url, cache.clone(), metrics.clone(), &config.db_pool)
            .await
            .expect("db");
        let blockchain = BlockchainClient::new(&config, cache.clone(), metrics.clone())
            .expect("blockchain");
        let email_service = EmailService::new(config.clone()).expect("email_service");
        let email_queue = EmailQueue::new(cache.clone(), db.clone());
        let webhook_handler = WebhookHandler::new(db.clone());
        let audit_logger = AuditLogger::new(db.pool());

        Arc::new(crate::AppState {
            config,
            cache: cache.clone(),
            db,
            blockchain,
            metrics,
            newsletter_rate_limiter: IpRateLimiter::new(cache),
            email_service,
            email_queue,
            webhook_handler,
            audit_logger,
        })
    }
}
