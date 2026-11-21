#[cfg(test)]
mod tests {
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };

    use axum::{
        body::Body,
        extract::Path,
        http::{Request, StatusCode},
        middleware,
        routing::{get, patch, post, put},
        Router,
    };
    use predictiq_api::{
        security::{api_key_middleware, ApiKeyAuth},
        validation::{
            content_type_validation_middleware, parse_request_body_max_bytes,
            request_size_validation_middleware, request_validation_middleware,
            validate_market_id, validate_user_address, DEFAULT_REQUEST_BODY_MAX_BYTES,
        },
    };
    use tower::ServiceExt;

    /// A well-formed Stellar strkey (56 chars, starts with `G`).
    const VALID_STELLAR_ADDRESS: &str =
        "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAWHF";

    /// Spy that records whether the downstream handler was reached. Used to
    /// prove that path-parameter validation rejects requests *before* any
    /// cache or RPC call could be attempted.
    #[derive(Clone, Default)]
    struct HandlerSpy {
        invoked: Arc<AtomicBool>,
    }

    impl HandlerSpy {
        fn new() -> Self {
            Self::default()
        }

        fn was_invoked(&self) -> bool {
            self.invoked.load(Ordering::SeqCst)
        }

        fn mark_invoked(&self) {
            self.invoked.store(true, Ordering::SeqCst);
        }
    }

    fn app() -> Router {
        app_with_spy(HandlerSpy::new())
    }

    fn app_with_spy(spy: HandlerSpy) -> Router {
        let auth = Arc::new(ApiKeyAuth::new(vec!["admin-valid-key".to_string()]));

        let admin_routes = Router::new()
            .route("/admin/mutate", post(|| async { "ok" }))
            .layer(middleware::from_fn_with_state(auth, api_key_middleware));

        // Routes mirroring the real path parameter names from
        // `services/api/src/validation.rs`. The handlers record invocation so
        // tests can assert validation short-circuits before the handler (and
        // therefore before any cache or RPC call) runs.
        let user_spy = spy.clone();
        let user_routes = Router::new().route(
            "/users/:address",
            get(move |Path(_address): Path<String>| {
                let spy = user_spy.clone();
                async move {
                    spy.mark_invoked();
                    "ok"
                }
            }),
        );

        let market_spy = spy.clone();
        let market_routes = Router::new().route(
            "/markets/:market_id",
            get(move |Path(_market_id): Path<String>| {
                let spy = market_spy.clone();
                async move {
                    spy.mark_invoked();
                    "ok"
                }
            }),
        );

        Router::new()
            .route("/safe", get(|| async { "ok" }))
            .route("/safe-post", post(|| async { "ok" }))
            .route("/safe-put", put(|| async { "ok" }))
            .route("/safe-patch", patch(|| async { "ok" }))
            .merge(admin_routes)
            .merge(user_routes)
            .merge(market_routes)
            .layer(middleware::from_fn(request_validation_middleware))
            .layer(middleware::from_fn(content_type_validation_middleware))
            .layer(middleware::from_fn(request_size_validation_middleware))
    }

    // ── Query / path validation ───────────────────────────────────────────────

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

    // ── User address path param validation ────────────────────────────────────

    #[test]
    fn valid_user_addresses_pass_through_unchanged() {
        let valid = [
            "0x0000000000000000000000000000000000000000",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "0xABCDEFABCDEFABCDEFABCDEFABCDEFABCDEFABCD",
            "0x1234567890abcdef1234567890abcdef12345678",
        ];

        for address in valid {
            let result = validate_user_address(address);
            assert!(result.is_ok(), "expected {address} to be valid");
            assert_eq!(result.unwrap(), address);
        }
    }

    #[test]
    fn malformed_user_addresses_are_rejected() {
        let invalid = [
            "",
            "0x",
            "0x123",
            "0xZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ",
            "abcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcde",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabc",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd0",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcz",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefab d",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefab-d",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefab'c",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefab;c",
            " 0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd ",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd\n",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd/../",
        ];

        for address in invalid {
            assert!(
                validate_user_address(address).is_err(),
                "expected {address:?} to be rejected"
            );
        }
    }

    // ── Market id path param validation ───────────────────────────────────────

    #[test]
    fn valid_market_ids_pass_through_unchanged() {
        let valid = [
            "1",
            "42",
            "0",
            "1234567890",
            "market-1",
            "market_1",
            "MARKET-1",
            "abc123",
            "a",
            "0xabcdefabcdefabcdefabcdefabcdefabcdefabcd",
        ];

        for market_id in valid {
            let result = validate_market_id(market_id);
            assert!(result.is_ok(), "expected {market_id} to be valid");
            assert_eq!(result.unwrap(), market_id);
        }
    }

    #[test]
    fn malformed_market_ids_are_rejected() {
        let invalid = [
            "",
            " ",
            "market 1",
            "market/1",
            "market\\1",
            "market'1",
            "market;1",
            "market--1",
            "-market",
            "market-",
            "_market",
            "market_",
            "1 OR 1=1",
            "1' OR '1'='1",
            "<script>",
            "market?1",
            "market#1",
            "market%201",
            "market\u{0000}1",
            "market\n1",
        ];

        for market_id in invalid {
            assert!(
                validate_market_id(market_id).is_err(),
                "expected {market_id:?} to be rejected"
            );
        }
    }

    // ── Path parameter validation ─────────────────────────────────────────────

    #[tokio::test]
    async fn malformed_user_address_is_rejected_before_handler() {
        let spy = HandlerSpy::new();
        let response = app_with_spy(spy.clone())
            .oneshot(
                Request::builder()
                    .uri("/users/not-an-address")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            !spy.was_invoked(),
            "handler must not run for a malformed user address"
        );
    }

    #[tokio::test]
    async fn malformed_market_id_is_rejected_before_handler() {
        let spy = HandlerSpy::new();
        let response = app_with_spy(spy.clone())
            .oneshot(
                Request::builder()
                    .uri("/markets/abc")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            !spy.was_invoked(),
            "handler must not run for a non-numeric market id"
        );
    }

    #[tokio::test]
    async fn well_formed_user_address_passes_validation() {
        let spy = HandlerSpy::new();
        let response = app_with_spy(spy.clone())
            .oneshot(
                Request::builder()
                    .uri(format!("/users/{VALID_STELLAR_ADDRESS}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            spy.was_invoked(),
            "handler should run for a well-formed user address"
        );
    }

    #[tokio::test]
    async fn numeric_market_id_passes_validation() {
        let spy = HandlerSpy::new();
        let response = app_with_spy(spy.clone())
            .oneshot(
                Request::builder()
                    .uri("/markets/42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert!(
            spy.was_invoked(),
            "handler should run for a numeric market id"
        );
    }

    // ── Content-Type: missing header → 415 ───────────────────────────────────

    #[tokio::test]
    async fn post_without_content_type_returns_415() {
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

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn put_without_content_type_returns_415() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/safe-put")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn patch_without_content_type_returns_415() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/safe-patch")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    // ── Content-Type: unsupported types → 415 ────────────────────────────────

    #[tokio::test]
    async fn post_with_text_plain_returns_415() {
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
    async fn put_with_text_plain_returns_415() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/safe-put")
                    .header("content-type", "text/plain")
                    .body(Body::from("plain"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn patch_with_text_plain_returns_415() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/safe-patch")
                    .header("content-type", "text/plain")
                    .body(Body::from("plain"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn post_with_form_urlencoded_returns_415() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/x-www-form-urlencoded")
                    .body(Body::from("key=value"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    #[tokio::test]
    async fn post_with_multipart_returns_415() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "multipart/form-data; boundary=----boundary")
                    .body(Body::from("data"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNSUPPORTED_MEDIA_TYPE);
    }

    // ── Content-Type: valid JSON variants → 200 ───────────────────────────────

    #[tokio::test]
    async fn post_with_application_json_is_allowed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn post_with_json_charset_utf8_is_allowed() {
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
    async fn post_with_json_charset_uppercase_is_allowed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json; charset=UTF-8")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn put_with_application_json_is_allowed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/safe-put")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn patch_with_application_json_is_allowed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("PATCH")
                    .uri("/safe-patch")
                    .header("content-type", "application/json")
                    .body(Body::from("{}"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // ── GET is not affected by content-type validation ────────────────────────

    #[tokio::test]
    async fn get_without_content_type_is_allowed() {
        let response = app()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/safe")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    // ── Request size limits ───────────────────────────────────────────────────

    #[tokio::test]
    async fn request_larger_than_default_limit_returns_413() {
        // Sends Content-Length header — fast-path rejection.
        let over_limit = vec![b'a'; DEFAULT_REQUEST_BODY_MAX_BYTES + 1];
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
        let at_limit = vec![b'a'; DEFAULT_REQUEST_BODY_MAX_BYTES];
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

    #[tokio::test]
    async fn request_one_byte_under_limit_is_allowed() {
        let under_limit = vec![b'a'; DEFAULT_REQUEST_BODY_MAX_BYTES - 1];
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json")
                    .body(Body::from(under_limit))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn chunked_body_over_limit_returns_413() {
        // No Content-Length header — exercises the stream-buffering guard,
        // not the fast-path Content-Length check.
        use axum::body::Body;
        use futures::stream;
        use std::io;

        let chunk_size = 65_536usize; // 64 KiB chunks
        let total = DEFAULT_REQUEST_BODY_MAX_BYTES + chunk_size;
        let chunk = vec![b'x'; chunk_size];

        // Build a stream of identical chunks that totals more than the limit.
        let chunks: Vec<Result<Vec<u8>, io::Error>> =
            std::iter::repeat_with(|| Ok(chunk.clone()))
                .take(total / chunk_size + 1)
                .collect();

        let body = Body::from_stream(stream::iter(chunks));

        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json")
                    // Deliberately omit Content-Length to force stream path.
                    .body(body)
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    #[tokio::test]
    async fn chunked_body_at_limit_is_allowed() {
        use axum::body::Body;
        use futures::stream;
        use std::io;

        // Exactly DEFAULT_REQUEST_BODY_MAX_BYTES bytes, no Content-Length.
        let data = vec![b'x'; DEFAULT_REQUEST_BODY_MAX_BYTES];
        let body = Body::from_stream(stream::iter(vec![
            Ok::<Vec<u8>, io::Error>(data),
        ]));

        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json")
                    .body(body)
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
        assert_eq!(parse_request_body_max_bytes(Some("0")), DEFAULT_REQUEST_BODY_MAX_BYTES);
        assert_eq!(parse_request_body_max_bytes(Some("invalid")), DEFAULT_REQUEST_BODY_MAX_BYTES);
        assert_eq!(parse_request_body_max_bytes(None), DEFAULT_REQUEST_BODY_MAX_BYTES);
    }

    #[test]
    fn default_limit_is_one_mib() {
        assert_eq!(DEFAULT_REQUEST_BODY_MAX_BYTES, 1_048_576);
    }

    #[tokio::test]
    async fn two_mb_body_returns_413() {
        let two_mb = vec![b'a'; 2 * 1_048_576];
        let response = app()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/safe-post")
                    .header("content-type", "application/json")
                    .body(Body::from(two_mb))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
    }

    // ── Admin route auth ──────────────────────────────────────────────────────

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
