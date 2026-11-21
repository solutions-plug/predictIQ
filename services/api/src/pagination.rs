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
/// Maximum accepted cursor length (base64url, unpadded).
pub const MAX_CURSOR_LEN: usize = 256;

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

impl ValidatedPagination {
    /// Number of rows to fetch from the database to detect a following page.
    ///
    /// Uses the `limit + 1` sentinel pattern so callers can tell whether more
    /// rows exist without issuing a second `COUNT` query.
    pub fn fetch_limit(&self) -> u32 {
        self.limit.saturating_add(1)
    }

    /// Clamp an arbitrary requested limit into the accepted `[1, MAX_PAGE_LIMIT]`
    /// range, falling back to `DEFAULT_LIMIT` when the request is absent.
    ///
    /// Handlers that cannot surface a `400` (for example because they build the
    /// query themselves) can use this to bound their listing defensively.
    pub fn clamped_limit(requested: Option<u32>) -> u32 {
        requested.unwrap_or(DEFAULT_LIMIT).clamp(1, MAX_PAGE_LIMIT)
    }
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

    // Check for mutually exclusive pagination strategies
    if params.offset.is_some() && params.cursor.is_some() {
        return Err(PaginationError {
            error:     "pagination_conflict",
            message:   "offset and cursor are mutually exclusive: use one pagination strategy at a time.".to_string(),
            max_limit: MAX_PAGE_LIMIT,
        });
    }

    if let Some(cursor) = &params.cursor {
        if cursor.len() > MAX_CURSOR_LEN
            || !cursor.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
        {
            return Err(PaginationError {
                error:     "invalid_cursor",
                message:   "cursor must be base64url (A-Z, a-z, 0-9, '-', '_') and at most 256 characters.".to_string(),
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

    /// Offset derived from the query, defaulting to `0`.
    ///
    /// `PaginationQuery` does not carry an offset field, so this always returns
    /// `0`; it exists so offset-based handlers can share one call site.
    pub fn offset(&self) -> i64 {
        0
    }

    /// Convert into the fully validated form, applying the same rules as
    /// [`validate_pagination`] but clamping instead of rejecting out-of-range
    /// limits.  Useful for handlers that must never fail on pagination input.
    pub fn to_validated(&self) -> ValidatedPagination {
        ValidatedPagination {
            limit:  self.limit(),
            cursor: self.cursor(),
            offset: self.offset() as u32,
        }
    }
}

/// A single page of results returned by paginated endpoints.
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub limit: u32,
    pub has_more: bool,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, next_cursor: Option<String>, limit: u32, has_more: bool) -> Self {
        Self { items, next_cursor, limit, has_more }
    }

    /// Build a page from a slice fetched with `limit + 1` rows (sentinel pattern).
    ///
    /// The extra row, when present, is trimmed and `has_more` is set; the
    /// `next_cursor` is derived from the last kept item via `make_cursor`.
    pub fn from_fetched(
        mut items: Vec<T>,
        limit: u32,
        make_cursor: impl Fn(&T) -> String,
    ) -> Self {
        let has_more = items.len() > limit as usize;
        if has_more {
            items.truncate(limit as usize);
        }
        let next_cursor = if has_more { items.last().map(make_cursor) } else { None };
        Self { items, next_cursor, limit, has_more }
    }

    /// Build a page from a slice fetched with `limit + 1` rows where the cursor
    /// is the item's own string representation (e.g. a UUID).
    pub fn from_fetched_ids(mut items: Vec<T>, limit: u32) -> Self
    where
        T: ToString,
    {
        let has_more = items.len() > limit as usize;
        if has_more {
            items.truncate(limit as usize);
        }
        let next_cursor = if has_more { items.last().map(|i| i.to_string()) } else { None };
        Self { items, next_cursor, limit, has_more }
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
        let cursor = "dGVzdC1jdXJzb3I"; // base64url("test-cursor"), unpadded
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
    fn cursor_rejects_std_base64_unicode_and_overlong() {
        for bad in ["a+b", "a/b", "abc=", "caf\u{e9}", &"a".repeat(MAX_CURSOR_LEN + 1)] {
            let err = validate_pagination(params_with_cursor(None, Some(bad.to_string()))).unwrap_err();
            assert_eq!(err.error, "invalid_cursor");
        }
        assert!(validate_pagination(params_with_cursor(None, Some("a".repeat(MAX_CURSOR_LEN)))).is_ok());
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

    #[test]
    fn offset_and_cursor_together_rejected() {
        let params = PaginationParams {
            limit: None,
            cursor: Some("abc123".to_string()),
            offset: Some(10),
        };
        let err = validate_pagination(params).unwrap_err();
        assert_eq!(err.error, "pagination_conflict");
        assert!(err.message.to_lowercase().contains("mutually exclusive"));
    }

    #[test]
    fn offset_alone_accepted() {
        let params = PaginationParams {
            limit: None,
            cursor: None,
            offset: Some(10),
        };
        let v = validate_pagination(params).unwrap();
        assert_eq!(v.offset, 10);
        assert!(v.cursor.is_none());
    }

    #[test]
    fn cursor_alone_accepted() {
        let params = PaginationParams {
            limit: None,
            cursor: Some("abc123".to_string()),
            offset: None,
        };
        let v = validate_pagination(params).unwrap();
        assert_eq!(v.cursor.as_deref(), Some("abc123"));
        assert_eq!(v.offset, 0);
    }

    #[test]
    fn neither_offset_nor_cursor_uses_defaults() {
        let params = PaginationParams {
            limit: None,
            cursor: None,
            offset: None,
        };
        let v = validate_pagination(params).unwrap();
        assert_eq!(v.limit, DEFAULT_LIMIT);
        assert!(v.cursor.is_none());
        assert_eq!(v.offset, 0);
    }

    // ── Helper surface used by bounded listings ───────────────────────────────

    #[test]
    fn fetch_limit_adds_sentinel_row() {
        let v = validate_pagination(params(Some(10))).unwrap();
        assert_eq!(v.fetch_limit(), 11);
    }

    #[test]
    fn fetch_limit_saturates_at_max() {
        let v = validate_pagination(params(Some(MAX_PAGE_LIMIT))).unwrap();
        assert_eq!(v.fetch_limit(), MAX_PAGE_LIMIT + 1);
        assert_eq!(ValidatedPagination { limit: u32::MAX, cursor: None, offset: 0 }.fetch_limit(), u32::MAX);
    }

    #[test]
    fn clamped_limit_applies_default_and_maximum() {
        assert_eq!(ValidatedPagination::clamped_limit(None), DEFAULT_LIMIT);
        assert_eq!(ValidatedPagination::clamped_limit(Some(0)), 1);
        assert_eq!(ValidatedPagination::clamped_limit(Some(5)), 5);
        assert_eq!(ValidatedPagination::clamped_limit(Some(MAX_PAGE_LIMIT)), MAX_PAGE_LIMIT);
        assert_eq!(ValidatedPagination::clamped_limit(Some(MAX_PAGE_LIMIT + 1)), MAX_PAGE_LIMIT);
        assert_eq!(ValidatedPagination::clamped_limit(Some(u32::MAX)), MAX_PAGE_LIMIT);
    }

    #[test]
    fn pagination_query_clamps_limit_and_defaults_offset() {
        let q = PaginationQuery { limit: None, cursor: None };
        assert_eq!(q.limit(), DEFAULT_LIMIT as i64);
        assert_eq!(q.offset(), 0);

        let q = PaginationQuery { limit: Some(0), cursor: None };
        assert_eq!(q.limit(), 1);

        let q = PaginationQuery { limit: Some(MAX_PAGE_LIMIT as i64 + 50), cursor: None };
        assert_eq!(q.limit(), MAX_PAGE_LIMIT as i64);

        let q = PaginationQuery { limit: Some(7), cursor: Some("abc".to_string()) };
        let v = q.to_validated();
        assert_eq!(v.limit, 7);
        assert_eq!(v.cursor.as_deref(), Some("abc"));
        assert_eq!(v.offset, 0);
    }

    #[test]
    fn paginated_response_from_fetched_trims_sentinel_and_sets_metadata() {
        let items: Vec<u64> = (1..=6).collect();
        let page = PaginatedResponse::from_fetched(items, 5, |n| n.to_string());
        assert_eq!(page.items, vec![1, 2, 3, 4, 5]);
        assert_eq!(page.limit, 5);
        assert!(page.has_more);
        assert_eq!(page.next_cursor.as_deref(), Some("5"));
    }

    #[test]
    fn paginated_response_from_fetched_without_sentinel_has_no_more() {
        let items: Vec<u64> = (1..=5).collect();
        let page = PaginatedResponse::from_fetched(items, 5, |n| n.to_string());
        assert_eq!(page.items.len(), 5);
        assert!(!page.has_more);
        assert!(page.next_cursor.is_none());
    }

    #[test]
    fn paginated_response_from_fetched_ids_uses_item_display() {
        let items: Vec<u64> = (1..=3).collect();
        let page = PaginatedResponse::from_fetched_ids(items, 2);
        assert_eq!(page.items, vec![1, 2]);
        assert!(page.has_more);
        assert_eq!(page.next_cursor.as_deref(), Some("2"));
    }

    #[test]
    fn paginated_response_empty_page_has_no_cursor() {
        let page = PaginatedResponse::<u64>::from_fetched_ids(vec![], DEFAULT_LIMIT);
        assert!(page.items.is_empty());
        assert!(!page.has_more);
        assert!(page.next_cursor.is_none());
        assert_eq!(page.limit, DEFAULT_LIMIT);
    }

    // ── PaginationQuery clamping / malformed cursors ─────────────────────────

    #[test]
    fn query_limit_defaults_when_absent() {
        assert_eq!(PaginationQuery::default().limit(), DEFAULT_LIMIT as i64);
        assert_eq!(DEFAULT_LIMIT, 20);
    }

    #[test]
    fn query_limit_below_minimum_clamps_to_one() {
        for v in [0, -1, i64::MIN] {
            let q = PaginationQuery { limit: Some(v), cursor: None };
            assert_eq!(q.limit(), 1);
        }
    }

    #[test]
    fn query_limit_above_maximum_clamps_to_max() {
        for v in [MAX_PAGE_LIMIT as i64 + 1, 10_000, i64::MAX] {
            let q = PaginationQuery { limit: Some(v), cursor: None };
            assert_eq!(q.limit(), MAX_PAGE_LIMIT as i64);
        }
    }

    #[test]
    fn query_limit_in_range_is_unchanged_and_cursor_round_trips() {
        let q = PaginationQuery { limit: Some(50), cursor: Some("dGVzdA".into()) };
        assert_eq!(q.limit(), 50);
        assert_eq!(q.cursor().as_deref(), Some("dGVzdA"));
    }

    #[test]
    fn padded_or_truncated_cursors_are_rejected_or_accepted_as_opaque() {
        // Padding is not part of the unpadded base64url alphabet → rejected.
        let err = validate_pagination(params_with_cursor(None, Some("dGVzdA==".into()))).unwrap_err();
        assert_eq!(err.error, "invalid_cursor");
        // A truncated-but-alphabet-valid cursor is opaque to this layer and accepted.
        let v = validate_pagination(params_with_cursor(None, Some("dGVz".into()))).unwrap();
        assert_eq!(v.cursor.as_deref(), Some("dGVz"));
    }

    #[test]
    fn page_response_cursor_round_trips_through_validation() {
        let items: Vec<u64> = (1..=(DEFAULT_LIMIT as u64 + 1)).collect();
        let page = PageResponse::from_fetched(items, DEFAULT_LIMIT, |n| format!("c{n}"));
        let cursor = page.next_cursor.clone().unwrap();
        let v = validate_pagination(params_with_cursor(None, Some(cursor.clone()))).unwrap();
        assert_eq!(v.cursor, Some(cursor));
    }
    }
}
