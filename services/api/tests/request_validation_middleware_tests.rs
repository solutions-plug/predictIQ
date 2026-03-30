#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::{get, post},
        Router,
    };
    use predictiq_api::{
        security::{api_key_middleware, ApiKeyAuth},
        validation::{
            content_type_validation_middleware, parse_request_body_max_bytes,
            request_size_validation_middleware, request_validation_middleware,
        },
    };
    use tower::ServiceExt;

    fn app() -> Router {
        let auth = Arc::new(ApiKeyAuth::new(vec!["admin-valid-key".to_string()]));

        let admin_routes = Router::new()
            .route("/admin/mutate", post(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(auth, api_key_middleware));

        Router::new()
            .route("/safe", get(|| async { "ok" }))
            .route("/safe-post", post(|| async { "ok" }))
            .merge(admin_routes)
            .layer(middleware::from_fn(request_validation_middleware))
            .layer(middleware::from_fn(content_type_validation_middleware))
            .layer(middleware::from_fn(request_size_validation_middleware))
    }

    #[tokio::test]
    async fn suspicious_query_is_rejected() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/safe?q=' OR 1=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn safe_query_is_allowed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/safe?q=normal-search")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn suspicious_path_is_rejected() {
        let response = app()
            .oneshot(
                Request::builder()
                    .uri("/safe//nested")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn mutating_request_without_content_type_is_rejected() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn mutating_request_with_unsupported_content_type_is_rejected() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "text/plain")
                    .body(Body::from("plain"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn json_with_charset_is_allowed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json; charset=utf-8")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn request_larger_than_default_limit_returns_413() {
        let over_limit = vec![b'a'; 1_048_577];
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json")
                    .body(Body::from(over_limit))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn request_at_default_limit_is_allowed() {
        let at_limit = vec![b'a'; 1_048_576];
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json")
                    .body(Body::from(at_limit))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[test]
    fn request_size_limit_parsing_is_configurable() {
        assert_eq!(parse_request_body_max_bytes(Some("512")), 512);
        assert_eq!(parse_request_body_max_bytes(Some(" 2048 ")), 2048);
        assert_eq!(parse_request_body_max_bytes(Some("0")), 1_048_576);
        assert_eq!(parse_request_body_max_bytes(Some("invalid")), 1_048_576);
        assert_eq!(parse_request_body_max_bytes(None), 1_048_576);
    }

    #[tokio::test]
    async fn admin_route_requires_api_key() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/mutate")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_route_rejects_invalid_api_key() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/mutate")
                    .header("content-type", "application/json")
                    .header("x-api-key", "wrong-key")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_route_allows_valid_api_key() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/admin/mutate")
                    .header("content-type", "application/json")
                    .header("x-api-key", "admin-valid-key")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
