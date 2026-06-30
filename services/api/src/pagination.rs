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

    if let Some(cursor) = &params.cursor {
        if !cursor.chars().all(|c| c.is_alphanumeric() || matches!(c, '-' | '_' | '=' | '+' | '/')) {
            return Err(PaginationError {
                error:     "invalid_cursor",
                message:   "cursor contains invalid characters.".to_string(),
                max_limit: MAX_PAGE_LIMIT,
            });
        }
        if cursor.is_empty() {
            return Err(PaginationError {
                error:     "invalid_cursor",
                message:   "cursor must not be empty.".to_string(),
                max_limit: MAX_PAGE_LIMIT,
            });
        }
    }

    Ok(ValidatedPagination {
        limit,
        cursor: params.cursor,
        offset: params.offset.unwrap_or(0),
    })
}

/// Build a cursor-paginated response from a slice fetched with `limit + 1` rows.
///
/// If the fetched slice exceeds `limit`, there is a next page; the extra item is
/// trimmed and `next_cursor` is set. Callers must pass a function that derives the
/// opaque (base64-safe) cursor token from the last item kept.
#[derive(Debug, Serialize)]
pub struct PageResponse<T: Serialize> {
    pub items: Vec<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl<T: Serialize> PageResponse<T> {
    pub fn from_fetched(mut items: Vec<T>, limit: u32, make_cursor: impl Fn(&T) -> String) -> Self {
        let has_next = items.len() > limit as usize;
        if has_next {
            items.truncate(limit as usize);
        }
        let next_cursor = if has_next { items.last().map(make_cursor) } else { None };
        Self { items, next_cursor }
    }
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

    fn params_with_cursor(limit: Option<u32>, cursor: Option<String>) -> PaginationParams {
        PaginationParams { limit, cursor, offset: None }
    }

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

    // ── Cursor-based pagination edge cases ────────────────────────────────────

    #[test]
    fn empty_result_set_produces_no_next_cursor() {
        let page = PageResponse::<String>::from_fetched(vec![], DEFAULT_LIMIT, |s| s.clone());
        assert!(page.items.is_empty());
        assert!(page.next_cursor.is_none(), "last page must have no next_cursor");
    }

    #[test]
    fn single_item_result_has_no_next_cursor() {
        let page = PageResponse::from_fetched(vec!["item-1".to_string()], DEFAULT_LIMIT, |s| s.clone());
        assert_eq!(page.items.len(), 1);
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn exactly_one_full_page_has_no_next_cursor() {
        // Simulate fetching limit rows (no +1 sentinel) → no next page.
        let items: Vec<u64> = (1..=DEFAULT_LIMIT as u64).collect();
        let page = PageResponse::from_fetched(items, DEFAULT_LIMIT, |n| n.to_string());
        assert_eq!(page.items.len(), DEFAULT_LIMIT as usize);
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn results_at_limit_boundary_signals_next_page() {
        // Fetch limit+1 rows (the sentinel pattern); next_cursor should be set.
        let items: Vec<u64> = (1..=(DEFAULT_LIMIT as u64 + 1)).collect();
        let page = PageResponse::from_fetched(items, DEFAULT_LIMIT, |n| n.to_string());
        assert_eq!(page.items.len(), DEFAULT_LIMIT as usize, "extra sentinel row must be trimmed");
        assert!(page.next_cursor.is_some(), "sentinel row means there is a next page");
    }

    #[test]
    fn valid_looking_cursor_accepted() {
        // A cursor that looks like a deleted-item cursor (valid base64-safe chars) must
        // pass validation; the DB will simply return an empty result set.
        let cursor = "dGVzdC1jdXJzb3I="; // base64("test-cursor")
        let v = validate_pagination(params_with_cursor(None, Some(cursor.to_string()))).unwrap();
        assert_eq!(v.cursor.as_deref(), Some(cursor));
    }

    #[test]
    fn tampered_cursor_with_invalid_chars_returns_400() {
        // Angle brackets, quotes, or script injection characters must be rejected.
        let tampered = "<script>alert(1)</script>";
        let err = validate_pagination(params_with_cursor(None, Some(tampered.to_string()))).unwrap_err();
        assert_eq!(err.error, "invalid_cursor");
    }

    #[test]
    fn tampered_cursor_with_null_byte_returns_400() {
        let tampered = "valid-prefix\x00injected";
        let err = validate_pagination(params_with_cursor(None, Some(tampered.to_string()))).unwrap_err();
        assert_eq!(err.error, "invalid_cursor");
    }

    #[test]
    fn empty_cursor_string_returns_400() {
        let err = validate_pagination(params_with_cursor(None, Some(String::new()))).unwrap_err();
        assert_eq!(err.error, "invalid_cursor");
    }

    #[test]
    fn tampered_cursor_response_code_is_bad_request() {
        use axum::response::IntoResponse;
        use axum::http::StatusCode;

        let err = PaginationError {
            error:     "invalid_cursor",
            message:   "cursor contains invalid characters.".to_string(),
            max_limit: MAX_PAGE_LIMIT,
        };
        let resp = err.into_response();
        assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    }
}
