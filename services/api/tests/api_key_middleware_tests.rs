/// Integration tests for `api_key_middleware`.
///
/// Covers every auth-failure permutation and verifies:
/// - status code is always 401
/// - `content-type` is `application/json`
/// - `www-authenticate` header is present
/// - response body contains an `error` field
/// - valid key passes through to the handler (200)
#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use predictiq_api::security::{api_key_middleware, ApiKeyAuth};
    use tower::ServiceExt;

    fn app() -> Router {
        let auth = Arc::new(ApiKeyAuth::new(vec!["valid-key".to_string()]));
        Router::new()
            .route("/protected", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(auth, api_key_middleware))
    }

    /// Builds a router whose auth state contains a single key that has been
    /// marked revoked (`revoked_at` set). Requests using this key must be
    /// rejected exactly like an unknown key.
    fn revoked_app() -> Router {
        let auth = Arc::new(ApiKeyAuth::new_with_revoked(
            vec!["valid-key".to_string()],
            vec!["revoked-key".to_string()],
        ));
        Router::new()
            .route("/protected", get(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(auth, api_key_middleware))
    }

    async fn body_json(resp: axum::response::Response) -> serde_json::Value {
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    // ── 401 permutations ─────────────────────────────────────────────────────

    #[tokio::test]
    async fn missing_header_returns_401() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert!(resp.headers()["content-type"]
            .to_str()
            .unwrap()
            .contains("application/json"));
        assert!(resp.headers().contains_key("www-authenticate"));
        let body = body_json(resp).await;
        assert!(body.get("error").is_some());
    }

    #[tokio::test]
    async fn empty_key_returns_401() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("x-api-key", "")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert!(resp.headers().contains_key("www-authenticate"));
        let body = body_json(resp).await;
        assert!(body.get("error").is_some());
    }

    #[tokio::test]
    async fn wrong_key_returns_401() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("x-api-key", "wrong-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert!(resp.headers().contains_key("www-authenticate"));
        let body = body_json(resp).await;
        assert!(body.get("error").is_some());
    }

    #[tokio::test]
    async fn partial_key_returns_401() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("x-api-key", "valid-ke") // one char short
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn key_with_extra_chars_returns_401() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("x-api-key", "valid-key-extra")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
    }

    // ── Happy path ───────────────────────────────────────────────────────────

    #[tokio::test]
    async fn valid_key_passes_through() {
        let resp = app()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("x-api-key", "valid-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    // ── Revocation ───────────────────────────────────────────────────────────

    /// A key whose `revoked_at` is set must be rejected with 401, must carry
    /// the `www-authenticate` header, and must return a JSON `error` body.
    /// This test fails if the `revoked_at` check in the middleware is removed.
    #[tokio::test]
    async fn revoked_key_returns_401() {
        let resp = revoked_app()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("x-api-key", "revoked-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        assert!(resp.headers().contains_key("www-authenticate"));
        assert!(resp.headers()["content-type"]
            .to_str()
            .unwrap()
            .contains("application/json"));
        let body = body_json(resp).await;
        assert!(body.get("error").is_some());
    }

    /// A key that is present but not revoked must still authenticate (200).
    #[tokio::test]
    async fn non_revoked_key_still_authenticates() {
        let resp = revoked_app()
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("x-api-key", "valid-key")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(resp.status(), StatusCode::OK);
    }

    /// If the middleware distinguishes revoked keys from unknown keys in its
    /// rejection reason, assert the revoked-key response carries the revoked
    /// reason rather than the generic unknown-key reason.
    #[tokio::test]
    async fn revoked_key_rejection_reason_distinguishes_from_unknown() {
        let revoked_body = body_json(
            revoked_app()
                .oneshot(
                    Request::builder()
                        .uri("/protected")
                        .header("x-api-key", "revoked-key")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;

        let unknown_body = body_json(
            revoked_app()
                .oneshot(
                    Request::builder()
                        .uri("/protected")
                        .header("x-api-key", "unknown-key")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap(),
        )
        .await;

        let revoked_reason = revoked_body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let unknown_reason = unknown_body
            .get("error")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        // The revoked key must be rejected with a reason that is not the
        // generic unknown-key reason, so operators can tell the two apart.
        assert_ne!(revoked_reason, unknown_reason);
        assert!(revoked_reason.to_lowercase().contains("revok"));
    }
}
