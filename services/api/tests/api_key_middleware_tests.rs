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
}
