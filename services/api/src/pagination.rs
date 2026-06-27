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

    Ok(ValidatedPagination {
        limit,
        cursor: params.cursor,
        offset: params.offset.unwrap_or(0),
    })
}

pub struct ValidatedPaginationQuery(pub ValidatedPagination);

/// Lightweight raw pagination query used by handlers that do their own
/// bounds-checking or in-memory slicing.  Axum extracts this directly from
/// the query string; call `.limit()` / `.cursor()` for the clamped values.
#[derive(Debug, Clone, Deserialize, Default, utoipa::IntoParams)]
pub struct PaginationQuery {
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

impl PaginationQuery {
    pub fn limit(&self) -> i64 {
        self.limit.unwrap_or(DEFAULT_LIMIT as i64).max(1).min(MAX_PAGE_LIMIT as i64)
    }

    pub fn cursor(&self) -> Option<String> {
        self.cursor.clone()
    }
}

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
