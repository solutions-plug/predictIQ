//! Cursor / offset pagination helpers for predictIQ API handlers.
//!
//! ## Limits
//! - Default `limit`: 20 rows
//! - Maximum `limit`: 100 rows (configurable via `MAX_PAGE_LIMIT`)
//! - Requests that exceed the maximum receive `400 Bad Request`.

use axum::{
    extract::Query,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::{Deserialize, Serialize};

// ── Simple query types used by API handlers ───────────────────────────────────

/// Minimal pagination query parameters used directly by API route handlers.
/// Applies soft clamping rather than returning 400, so it composes well
/// with cached endpoints that accept any limit.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

impl PaginationQuery {
    /// Returns the requested limit clamped to [1, MAX_PAGE_LIMIT].
    pub fn limit(&self) -> i64 {
        self.limit
            .unwrap_or(DEFAULT_LIMIT as i64)
            .clamp(1, MAX_PAGE_LIMIT as i64)
    }

    /// Returns the opaque pagination cursor, if provided by the client.
    pub fn cursor(&self) -> Option<String> {
        self.cursor.clone()
    }
}

/// A single page of results with cursor-based navigation metadata.
#[derive(Debug, Clone, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub limit: i64,
    pub has_more: bool,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(
        items: Vec<T>,
        next_cursor: Option<String>,
        limit: i64,
        has_more: bool,
    ) -> Self {
        Self {
            items,
            next_cursor,
            limit,
            has_more,
        }
    }
}

/// Hard cap on the number of rows a client may request in a single page.
pub const MAX_PAGE_LIMIT: u32 = 100;
/// Default rows returned when the client omits `limit`.
pub const DEFAULT_LIMIT: u32 = 20;

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub limit: Option<u32>,
    pub cursor: Option<String>,
    pub offset: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ValidatedPagination {
    pub limit:  u32,
    pub cursor: Option<String>,
    pub offset: u32,
}

#[derive(Debug, Serialize)]
pub struct PaginationError {
    pub error:     &'static str,
    pub message:   String,
    pub max_limit: u32,
}

impl IntoResponse for PaginationError {
    fn into_response(self) -> Response {
        (StatusCode::BAD_REQUEST, Json(self)).into_response()
    }
}

pub fn validate_pagination(params: PaginationParams) -> Result<ValidatedPagination, PaginationError> {
    let limit = params.limit.unwrap_or(DEFAULT_LIMIT);

    if limit == 0 {
        return Err(PaginationError {
            error:     "invalid_limit",
            message:   "limit must be at least 1.".to_string(),
            max_limit: MAX_PAGE_LIMIT,
        });
    }

    if limit > MAX_PAGE_LIMIT {
        return Err(PaginationError {
            error:   "limit_exceeded",
            message: format!(
                "limit {} exceeds the maximum allowed value of {}. \
                 Use cursor-based pagination for large datasets.",
                limit, MAX_PAGE_LIMIT
            ),
            max_limit: MAX_PAGE_LIMIT,
        });
    }

    Ok(ValidatedPagination {
        limit,
        cursor: params.cursor,
        offset: params.offset.unwrap_or(0),
    })
}

pub struct ValidatedPaginationQuery(pub ValidatedPagination);

#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for ValidatedPaginationQuery
where
    S: Send + Sync,
{
    type Rejection = PaginationError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        let Query(params) = Query::<PaginationParams>::from_request_parts(parts, state)
            .await
            .map_err(|_| PaginationError {
                error:     "invalid_query",
                message:   "Failed to parse pagination query parameters.".to_string(),
                max_limit: MAX_PAGE_LIMIT,
            })?;
        validate_pagination(params).map(ValidatedPaginationQuery)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(limit: Option<u32>) -> PaginationParams {
        PaginationParams { limit, cursor: None, offset: None }
    }

    // ── PaginationQuery tests ──────────────────────────────────────────────────

    #[test]
    fn pagination_query_default_limit() {
        let q = PaginationQuery::default();
        assert_eq!(q.limit(), DEFAULT_LIMIT as i64);
    }

    #[test]
    fn pagination_query_clamps_zero_to_one() {
        let q = PaginationQuery { limit: Some(0), cursor: None };
        assert_eq!(q.limit(), 1);
    }

    #[test]
    fn pagination_query_clamps_over_max() {
        let q = PaginationQuery { limit: Some(9999), cursor: None };
        assert_eq!(q.limit(), MAX_PAGE_LIMIT as i64);
    }

    #[test]
    fn pagination_query_returns_cursor() {
        let q = PaginationQuery { limit: None, cursor: Some("abc".to_string()) };
        assert_eq!(q.cursor(), Some("abc".to_string()));
    }

    #[test]
    fn pagination_query_returns_none_cursor_when_absent() {
        let q = PaginationQuery::default();
        assert_eq!(q.cursor(), None);
    }

    // ── PaginatedResponse tests ───────────────────────────────────────────────

    #[test]
    fn paginated_response_stores_all_fields() {
        let resp = PaginatedResponse::new(
            vec![1u32, 2, 3],
            Some("cursor-xyz".to_string()),
            10,
            true,
        );
        assert_eq!(resp.items, vec![1, 2, 3]);
        assert_eq!(resp.next_cursor, Some("cursor-xyz".to_string()));
        assert_eq!(resp.limit, 10);
        assert!(resp.has_more);
    }

    #[test]
    fn paginated_response_empty_last_page() {
        let resp: PaginatedResponse<u32> =
            PaginatedResponse::new(vec![], None, 20, false);
        assert!(resp.items.is_empty());
        assert_eq!(resp.next_cursor, None);
        assert!(!resp.has_more);
    }

    // ── ValidatedPagination tests ─────────────────────────────────────────────

    #[test]
    fn default_limit_applied_when_omitted() {
        let v = validate_pagination(params(None)).unwrap();
        assert_eq!(v.limit, DEFAULT_LIMIT);
    }

    #[test]
    fn exact_max_limit_accepted() {
        let v = validate_pagination(params(Some(MAX_PAGE_LIMIT))).unwrap();
        assert_eq!(v.limit, MAX_PAGE_LIMIT);
    }

    #[test]
    fn limit_exceeding_max_returns_400() {
        let err = validate_pagination(params(Some(MAX_PAGE_LIMIT + 1))).unwrap_err();
        assert_eq!(err.error, "limit_exceeded");
        assert!(err.message.contains(&MAX_PAGE_LIMIT.to_string()));
    }

    #[test]
    fn zero_limit_rejected() {
        let err = validate_pagination(params(Some(0))).unwrap_err();
        assert_eq!(err.error, "invalid_limit");
    }

    #[test]
    fn large_limit_rejected_with_max_in_message() {
        let err = validate_pagination(params(Some(1_000_000))).unwrap_err();
        assert_eq!(err.max_limit, MAX_PAGE_LIMIT);
        assert!(err.message.contains("1000000"));
    }
}
