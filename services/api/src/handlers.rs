use crate::content_type::require_json_content_type;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::ValidateEmail;

use crate::{blockchain::HealthStatus, cache::{keys, InvalidationTag}, db::DbError, email::webhook::sendgrid_webhook_handler, pagination::{PaginatedResponse, PaginationQuery}, AppState};

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiError {
    pub code: &'static str,
    pub message: String,
    #[serde(skip)]
    #[schema(ignore)]
    pub status: StatusCode,
}

impl ApiError {
    pub fn internal(err: anyhow::Error) -> Self {
        tracing::error!(error = %err, "internal server error");
        Self {
            code: "INTERNAL_ERROR",
            message: "An internal error occurred.".to_string(),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self {
            code: "BAD_REQUEST",
            message: message.into(),
            status: StatusCode::BAD_REQUEST,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: "NOT_FOUND",
            message: message.into(),
            status: StatusCode::NOT_FOUND,
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: "CONFLICT",
            message: message.into(),
            status: StatusCode::CONFLICT,
        }
    }

    pub fn rate_limited() -> Self {
        Self {
            code: "RATE_LIMITED",
            message: "Too many requests, please try again later.".to_string(),
            status: StatusCode::TOO_MANY_REQUESTS,
        }
    }

    pub fn service_unavailable(message: impl Into<String>) -> Self {
        Self {
            code: "SERVICE_UNAVAILABLE",
            message: message.into(),
            status: StatusCode::SERVICE_UNAVAILABLE,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}

fn into_api_error(err: anyhow::Error) -> ApiError {
    if let Some(db_err) = err.downcast_ref::<DbError>() {
        match db_err {
            DbError::Timeout => {
                return ApiError::service_unavailable("database query timed out");
            }
            DbError::PoolExhausted => {
                return ApiError::service_unavailable("database connection pool exhausted");
            }
            DbError::ConstraintViolation(msg) => {
                return ApiError::conflict(msg.clone());
            }
            DbError::Other(_) => {}
        }
    }
    ApiError::internal(err)
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct FeaturedMarketView {
    pub id: i64,
    pub title: String,
    pub volume: f64,
    pub ends_at: chrono::DateTime<chrono::Utc>,
    pub onchain_volume: String,
    pub resolved_outcome: Option<u32>,
}

#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy or degraded"),
    )
)]
pub async fn health(State(state): State<Arc<AppState>>, headers: HeaderMap) -> impl IntoResponse {
    use crate::cache::CircuitState;
    use crate::correlation::REQUEST_ID_HEADER;

    let request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    let cb_state = match state.cache.circuit_state() {
        CircuitState::Closed => "closed",
        CircuitState::Open => "open",
        CircuitState::HalfOpen => "half_open",
    };
    let pool = state.cache.pool_status();

    let mut health_status = serde_json::json!({
        "status": "ok",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "request_id": request_id,
        "redis": {
            "circuit_breaker": cb_state,
            "pool_size": pool.size,
            "pool_available": pool.available,
        },
        "db": {
            "status": "ok",
        },
        "workers": {
            "blockchain_sync": "running",
            "blockchain_monitor": "running",
            "email_queue": "running",
            "rate_limiter_cleanup": "running"
        }
    });

    if state.cache.ping().await.is_err() {
        health_status["status"] = "degraded".into();
        health_status["redis"]["status"] = "unhealthy".into();
    }

    if state.db.ping().await.is_err() {
        health_status["status"] = "degraded".into();
        health_status["db"]["status"] = "unhealthy".into();
    }

    if let Ok(processing_count) = state.email_queue.get_processing_count().await {
        health_status["workers"]["email_queue_processing"] = processing_count.into();
    }

    (StatusCode::OK, Json(health_status))
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct NewsletterSubscribeRequest {
    pub email: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct NewsletterEmailRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct NewsletterConfirmQuery {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct NewsletterUnsubscribeQuery {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct NewsletterExportQuery {
    pub email: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct NewsletterResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, utoipa::ToSchema)]
pub struct NewsletterExportResponse {
    pub success: bool,
    #[schema(value_type = Object)]
    pub data: crate::db::NewsletterSubscriber,
}

fn normalized_email(raw: &str) -> Option<String> {
    let candidate = raw.trim().to_lowercase();
    if candidate.validate_email() {
        Some(candidate)
    } else {
        None
    }
}

fn is_disposable_email(email: &str) -> bool {
    const DISPOSABLE_DOMAINS: &[&str] = &["mailinator.com", "tempmail.com", "guerrillamail.com"];

    email
        .rsplit_once('@')
        .map(|(_, domain)| DISPOSABLE_DOMAINS.contains(&domain))
        .unwrap_or(false)
}

use crate::security::extract_client_ip_cidrs;

#[utoipa::path(
    post,
    path = "/api/v1/newsletter/subscribe",
    tag = "newsletter",
    request_body = NewsletterSubscribeRequest,
    responses(
        (status = 202, description = "Subscription request accepted", body = NewsletterResponse),
        (status = 400, description = "Invalid email address", body = NewsletterResponse),
    )
)]
pub async fn newsletter_subscribe(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    connect_info: Option<axum::extract::ConnectInfo<std::net::SocketAddr>>,
    Json(payload): Json<NewsletterSubscribeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let ip = extract_client_ip_cidrs(
        &headers,
        connect_info.as_ref(),
        state.config.trust_proxy,
        &state.config.trusted_proxy_cidrs,
    );

    let email = match normalized_email(&payload.email) {
        Some(value) => value,
        None => {
            return Ok((
                StatusCode::BAD_REQUEST,
                Json(NewsletterResponse {
                    success: false,
                    message: "Invalid email address.".to_string(),
                }),
            ));
        }
    };

    if is_disposable_email(&email) {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(NewsletterResponse {
                success: false,
                message: "Disposable emails are not allowed.".to_string(),
            }),
        ));
    }
    let source = payload
        .source
        .unwrap_or_else(|| "direct".to_string())
        .trim()
        .chars()
        .take(64)
        .collect::<String>();
    let source = if source.is_empty() {
        "direct".to_string()
    } else {
        source
    };

    // Always upsert and send confirmation — uniform response prevents enumeration.
    // For already-confirmed active subscribers we skip the DB write but still
    // return the same success body and status so response time/body are identical.
    let existing = state
        .db
        .newsletter_get_by_email(&email)
        .await
        .map_err(into_api_error)?;

    let already_active = existing
        .as_ref()
        .map(|s| s.confirmed && s.unsubscribed_at.is_none())
        .unwrap_or(false);

    if !already_active {
        let token = Uuid::new_v4().to_string();
        state
            .db
            .newsletter_upsert_pending(&email, &source, &token)
            .await
            .map_err(into_api_error)?;

        let confirm_url = format!(
            "{}/api/v1/newsletter/confirm?token={token}",
            state.config.base_url.trim_end_matches('/')
        );
        let unsubscribe_url = state
            .config
            .unsubscribe_signing_secret
            .as_deref()
            .and_then(|secret| crate::newsletter::generate_unsubscribe_token(&email, secret).ok())
            .map(|tok| format!(
                "{}/api/v1/newsletter/unsubscribe?token={tok}",
                state.config.base_url.trim_end_matches('/')
            ))
            .unwrap_or_default();
        let template_data = serde_json::json!({
            "confirm_url": confirm_url,
            "unsubscribe_url": unsubscribe_url,
            "email": email
        });
        state
            .email_queue
            .enqueue(
                crate::email::types::EmailJobType::NewsletterConfirmation,
                &email,
                "newsletter_confirmation",
                template_data,
                0,
            )
            .await
            .map_err(into_api_error)?;
    }

    let request_id = headers
        .get(crate::correlation::REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");
    tracing::info!(request_id, email = %email, source = %source, ip = %ip, "newsletter subscription attempt");

    Ok((
        StatusCode::ACCEPTED,
        Json(NewsletterResponse {
            success: true,
            message: "Please check your email to confirm your subscription.".to_string(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/newsletter/confirm",
    tag = "newsletter",
    params(NewsletterConfirmQuery),
    responses(
        (status = 200, description = "Subscription confirmed", body = NewsletterResponse),
        (status = 400, description = "Missing or invalid token", body = NewsletterResponse),
        (status = 404, description = "Token not found or expired", body = NewsletterResponse),
    )
)]
pub async fn newsletter_confirm(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NewsletterConfirmQuery>,
) -> Result<impl IntoResponse, ApiError> {
    if query.token.trim().is_empty() {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(NewsletterResponse {
                success: false,
                message: "Missing confirmation token.".to_string(),
            }),
        ));
    }

    let updated = state
        .db
        .newsletter_confirm_by_token(query.token.trim(), state.config.newsletter_token_ttl_secs)
        .await
        .map_err(into_api_error)?;

    if !updated {
        return Ok((
            StatusCode::NOT_FOUND,
            Json(NewsletterResponse {
                success: false,
                message: "Invalid or expired confirmation token.".to_string(),
            }),
        ));
    }

    Ok((
        StatusCode::OK,
        Json(NewsletterResponse {
            success: true,
            message: "Subscription confirmed.".to_string(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/newsletter/unsubscribe",
    tag = "newsletter",
    params(NewsletterUnsubscribeQuery),
    responses(
        (status = 200, description = "Successfully unsubscribed", body = NewsletterResponse),
        (status = 401, description = "Invalid unsubscribe token", body = NewsletterResponse),
    )
)]
pub async fn newsletter_unsubscribe(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NewsletterUnsubscribeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let secret = match state.config.unsubscribe_signing_secret.as_deref() {
        Some(s) => s.to_string(),
        None => {
            return Ok((
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(NewsletterResponse {
                    success: false,
                    message: "Unsubscribe not configured.".to_string(),
                }),
            ));
        }
    };

    let email = match crate::newsletter::validate_unsubscribe_token(&query.token, &secret) {
        Some(e) => e,
        None => {
            return Ok((
                StatusCode::UNAUTHORIZED,
                Json(NewsletterResponse {
                    success: false,
                    message: "Invalid unsubscribe token.".to_string(),
                }),
            ));
        }
    };

    let _ = state
        .db
        .newsletter_unsubscribe(&email)
        .await
        .map_err(into_api_error)?;

    tracing::info!("[newsletter] unsubscribed email={email}");

    Ok((
        StatusCode::OK,
        Json(NewsletterResponse {
            success: true,
            message: "Successfully unsubscribed.".to_string(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/newsletter/gdpr/export",
    tag = "newsletter",
    params(NewsletterExportQuery),
    responses(
        (status = 200, description = "GDPR data export", body = NewsletterExportResponse),
        (status = 400, description = "Invalid email", body = NewsletterResponse),
        (status = 404, description = "No record found", body = NewsletterResponse),
        (status = 429, description = "Rate limited", body = NewsletterResponse),
    )
)]
pub async fn newsletter_gdpr_export(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    connect_info: Option<axum::extract::ConnectInfo<std::net::SocketAddr>>,
    Query(query): Query<NewsletterExportQuery>,
) -> Result<Response, ApiError> {
    use crate::security::extract_client_ip_cidrs;
    let ip = extract_client_ip_cidrs(
        &headers,
        connect_info.as_ref(),
        state.config.trust_proxy,
        &state.config.trusted_proxy_cidrs,
    );
    let allowed_ip = state
        .newsletter_rate_limiter
        .allow(
            &format!("gdpr_export:ip:{ip}"),
            state.config.gdpr_export_rate_limit as usize,
            std::time::Duration::from_secs(state.config.gdpr_export_rate_window_secs),
        )
        .await;
    if !allowed_ip {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            Json(NewsletterResponse {
                success: false,
                message: "Too many requests, please try again later.".to_string(),
            }),
        )
            .into_response());
    }

    let Some(email) = normalized_email(&query.email) else {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(NewsletterResponse {
                success: false,
                message: "Invalid email address.".to_string(),
            }),
        )
            .into_response());
    };

    let data = state
        .db
        .newsletter_get_by_email(&email)
        .await
        .map_err(into_api_error)?;

    // Per-email rate limit (separate from IP limit)
    let allowed_email = state
        .newsletter_rate_limiter
        .allow(
            &format!("gdpr_export:email:{email}"),
            state.config.gdpr_export_rate_limit as usize,
            std::time::Duration::from_secs(state.config.gdpr_export_rate_window_secs),
        )
        .await;
    if !allowed_email {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            Json(NewsletterResponse {
                success: false,
                message: "Too many requests, please try again later.".to_string(),
            }),
        )
            .into_response());
    }

    let Some(data) = data else {
        return Ok((
            StatusCode::NOT_FOUND,
            Json(NewsletterResponse {
                success: false,
                message: "No newsletter record found.".to_string(),
            }),
        )
            .into_response());
    };

    Ok((
        StatusCode::OK,
        Json(NewsletterExportResponse {
            success: true,
            data,
        }),
    )
        .into_response())
}

#[utoipa::path(
    delete,
    path = "/api/v1/newsletter/gdpr/delete",
    tag = "newsletter",
    request_body = NewsletterEmailRequest,
    responses(
        (status = 200, description = "Data deleted", body = NewsletterResponse),
        (status = 400, description = "Invalid email", body = NewsletterResponse),
    )
)]
pub async fn newsletter_gdpr_delete(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewsletterEmailRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let Some(email) = normalized_email(&payload.email) else {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(NewsletterResponse {
                success: false,
                message: "Invalid email address.".to_string(),
            }),
        ));
    };

    let _ = state
        .db
        .newsletter_gdpr_delete(&email)
        .await
        .map_err(into_api_error)?;

    tracing::info!("[newsletter] gdpr delete email={email}");

    Ok((
        StatusCode::OK,
        Json(NewsletterResponse {
            success: true,
            message: "Data deleted.".to_string(),
        }),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/statistics",
    tag = "markets",
    responses(
        (status = 200, description = "Platform statistics"),
    )
)]
pub async fn statistics(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, ApiError> {
    let start = Instant::now();
    let cache_key = keys::api_statistics();
    let ttl = Duration::from_secs(5 * 60);
    let endpoint = "statistics";

    let (payload, hit) = state
        .cache
        .get_or_set_json(&cache_key, ttl, || async {
            let data = state.db.statistics_cached().await?;
            Ok(data)
        })
        .await
        .map_err(into_api_error)?;

    if hit {
        state.metrics.observe_hit("api", endpoint);
    } else {
        state.metrics.observe_miss("api", endpoint);
    }
    state.metrics.observe_request(endpoint, start.elapsed());

    Ok((StatusCode::OK, Json(payload)))
}

#[utoipa::path(
    get,
    path = "/api/v1/markets/featured",
    tag = "markets",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Paginated list of featured markets"),
    )
)]
pub async fn featured_markets(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let start = Instant::now();
    let limit = query.limit();
    let cursor = query.cursor();
    let cache_key = keys::api_featured_markets();
    let ttl = Duration::from_secs(2 * 60);
    let endpoint = "featured_markets";

    let featured_limit = state.config.featured_limit;

    let (payload, hit) = state
        .cache
        .get_or_set_json(&cache_key, ttl, || async {
            let markets = state.db.featured_markets_cached(featured_limit).await?;
            let chain_futures = markets
                .iter()
                .map(|m| state.blockchain.market_data_cached(m.id));
            let chain_data = join_all(chain_futures).await;

            let mut view = Vec::with_capacity(markets.len());
            for (m, chain_result) in markets.into_iter().zip(chain_data.into_iter()) {
                let chain = chain_result?;
                view.push(FeaturedMarketView {
                    id: m.id,
                    title: m.title,
                    volume: m.volume,
                    ends_at: m.ends_at,
                    onchain_volume: chain.onchain_volume,
                    resolved_outcome: chain.resolved_outcome,
                });
            }
            Ok(view)
        })
        .await
        .map_err(into_api_error)?;

    let start_idx = cursor
        .as_ref()
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(0);
    let end_idx = (start_idx + limit as usize).min(payload.len());
    let has_more = end_idx < payload.len();
    let next_cursor = if has_more {
        Some(end_idx.to_string())
    } else {
        None
    };

    let paginated = PaginatedResponse::new(
        payload[start_idx..end_idx].to_vec(),
        next_cursor,
        limit,
        has_more,
    );

    if hit {
        state.metrics.observe_hit("api", endpoint);
    } else {
        state.metrics.observe_miss("api", endpoint);
    }
    state.metrics.observe_request(endpoint, start.elapsed());

    Ok((StatusCode::OK, Json(paginated)))
}

#[utoipa::path(
    get,
    path = "/api/v1/content",
    tag = "markets",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Paginated content items"),
    )
)]
pub async fn content(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let start = Instant::now();
    let limit = query.limit();
    let cursor = query.cursor();
    let endpoint = "content";

    let cache_key = keys::api_content(limit);
    let ttl = Duration::from_secs(60 * 60);

    let (payload, hit) = state
        .cache
        .get_or_set_json(&cache_key, ttl, || async {
            let data = state.db.content_cached(limit).await?;
            Ok(data)
        })
        .await
        .map_err(into_api_error)?;

    let start_idx = cursor
        .as_ref()
        .and_then(|c| c.parse::<usize>().ok())
        .unwrap_or(0);
    let end_idx = (start_idx + limit as usize).min(payload.len());
    let has_more = end_idx < payload.len();
    let next_cursor = if has_more {
        Some(end_idx.to_string())
    } else {
        None
    };

    let paginated = PaginatedResponse::new(
        payload[start_idx..end_idx].to_vec(),
        next_cursor,
        limit,
        has_more,
    );

    if hit {
        state.metrics.observe_hit("api", endpoint);
    } else {
        state.metrics.observe_miss("api", endpoint);
    }
    state.metrics.observe_request(endpoint, start.elapsed());

    Ok((StatusCode::OK, Json(paginated)))
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct InvalidationResult {
    pub invalidated_keys: usize,
}

/// Resolve a market by its ID.
///
/// Workflow:
/// 1. Fetch the current market state from the blockchain.
/// 2. Persist the resolved outcome to the database.
/// 3. Invalidate only the cache keys that are directly affected by this market's
///    resolution (specific market key, oracle result, statistics aggregates, and
///    featured-markets list). Content pages and per-user bet lists are left intact
///    because they are not affected by a single market resolution.
/// 4. Cache invalidation only runs after a successful write — a failed DB update
///    leaves the cache untouched.
#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct ResolveMarketRequest {
    /// The winning outcome index (0-based).
    pub outcome_index: u32,
}

#[utoipa::path(
    post,
    path = "/api/v1/markets/{market_id}/resolve",
    tag = "markets",
    params(
        ("market_id" = i64, Path, description = "Market database ID"),
    ),
    request_body = ResolveMarketRequest,
    responses(
        (status = 200, description = "Market resolved and cache invalidated", body = InvalidationResult),
        (status = 400, description = "Bad request", body = ApiError),
        (status = 500, description = "Internal error", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn resolve_market(
    State(state): State<Arc<AppState>>,
    Path(market_id): Path<i64>,
    Json(payload): Json<ResolveMarketRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // 1. Persist the resolution to the database.
    state
        .db
        .resolve_market(market_id, payload.outcome_index)
        .await
        .map_err(into_api_error)?;

    // 2. Invalidate only the keys affected by this market's resolution via tag.
    let tag = InvalidationTag::MarketResolved {
        market_id,
        network: state.config.network_name().to_owned(),
        featured_limit: state.config.featured_limit,
    };
    let invalidated = state.cache.invalidate_tag(&tag).await.map_err(into_api_error)?;

    state
        .metrics
        .observe_invalidation("market_resolve", invalidated);

    tracing::info!(market_id, invalidated, "market resolved and cache invalidated");

    Ok((
        StatusCode::OK,
        Json(InvalidationResult {
            invalidated_keys: invalidated,
        }),
    ))
}

pub async fn metrics(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, ApiError> {
    state.db.record_pool_metrics();
    let body = state.metrics.render().map_err(into_api_error)?;
    Ok((
        StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/blockchain/health",
    tag = "blockchain",
    responses(
        (status = 200, description = "Blockchain node is healthy"),
        (status = 503, description = "Blockchain node is degraded or unreachable"),
    )
)]
pub async fn blockchain_health(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let data = state
        .blockchain
        .health_check_cached()
        .await
        .map_err(into_api_error)?;
    let status_code = match data.status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded | HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };
    Ok((status_code, Json(data)))
}

#[utoipa::path(
    get,
    path = "/api/v1/blockchain/markets/{market_id}",
    tag = "blockchain",
    params(
        ("market_id" = i64, Path, description = "Market database ID"),
    ),
    responses(
        (status = 200, description = "On-chain market data"),
        (status = 500, description = "Blockchain query failed", body = ApiError),
    )
)]
pub async fn blockchain_market_data(
    State(state): State<Arc<AppState>>,
    Path(market_id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let data = state
        .blockchain
        .market_data_cached(market_id)
        .await
        .map_err(into_api_error)?;
    Ok((StatusCode::OK, Json(data)))
}

#[utoipa::path(
    get,
    path = "/api/v1/blockchain/stats",
    tag = "blockchain",
    responses(
        (status = 200, description = "Platform-wide blockchain statistics"),
        (status = 500, description = "Blockchain query failed", body = ApiError),
    )
)]
pub async fn blockchain_platform_stats(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let data = state
        .blockchain
        .platform_statistics_cached()
        .await
        .map_err(into_api_error)?;
    Ok((StatusCode::OK, Json(data)))
}

#[utoipa::path(
    get,
    path = "/api/v1/blockchain/users/{user}/bets",
    tag = "blockchain",
    params(
        ("user" = String, Path, description = "Stellar account address"),
        PaginationQuery,
    ),
    responses(
        (status = 200, description = "Paginated list of user bets"),
        (status = 500, description = "Blockchain query failed", body = ApiError),
    )
)]
pub async fn blockchain_user_bets(
    State(state): State<Arc<AppState>>,
    Path(user): Path<String>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page_size = query.limit();
    // cursor encodes the page number (0-based)
    let page = query
        .cursor()
        .as_deref()
        .and_then(|c| c.parse::<i64>().ok())
        .unwrap_or(0)
        .max(0);

    let page_data = state
        .blockchain
        .user_bets_page(&user, page, page_size)
        .await
        .map_err(into_api_error)?;

    let has_more = (page + 1) * page_size < page_data.total;
    let next_cursor = if has_more {
        Some((page + 1).to_string())
    } else {
        None
    };

    let paginated = PaginatedResponse::new(
        page_data.items,
        next_cursor,
        page_size,
        has_more,
    );

    Ok((StatusCode::OK, Json(paginated)))
}

#[utoipa::path(
    get,
    path = "/api/v1/blockchain/oracle/{market_id}",
    tag = "blockchain",
    params(
        ("market_id" = i64, Path, description = "Market database ID"),
    ),
    responses(
        (status = 200, description = "Oracle resolution result for the market"),
        (status = 500, description = "Blockchain query failed", body = ApiError),
    )
)]
pub async fn blockchain_oracle_result(
    State(state): State<Arc<AppState>>,
    Path(market_id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    let data = state
        .blockchain
        .oracle_result_cached(market_id)
        .await
        .map_err(into_api_error)?;
    Ok((StatusCode::OK, Json(data)))
}

#[utoipa::path(
    get,
    path = "/api/v1/blockchain/tx/{tx_hash}",
    tag = "blockchain",
    params(
        ("tx_hash" = String, Path, description = "Stellar transaction hash"),
    ),
    responses(
        (status = 200, description = "Transaction status"),
        (status = 500, description = "Blockchain query failed", body = ApiError),
    )
)]
pub async fn blockchain_tx_status(
    State(state): State<Arc<AppState>>,
    Path(tx_hash): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    state.blockchain.watch_transaction(&tx_hash).await;
    let data = state
        .blockchain
        .transaction_status_cached(&tx_hash)
        .await
        .map_err(into_api_error)?;
    Ok((StatusCode::OK, Json(data)))
}

#[utoipa::path(
    post,
    path = "/api/blockchain/replay",
    tag = "blockchain",
    responses(
        (status = 200, description = "Replay progress"),
        (status = 500, description = "Replay failed", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn blockchain_replay(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<crate::blockchain::ReplayRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let progress = state
        .blockchain
        .replay_events(payload.from_ledger)
        .await
        .map_err(into_api_error)?;
    Ok((StatusCode::OK, Json(progress)))
}

pub async fn warm_critical_caches(state: Arc<AppState>) -> anyhow::Result<()> {
    macro_rules! warm {
        ($name:expr, $fut:expr, $ok:ident, $fail:ident) => {
            if let Err(e) = $fut.await {
                $fail += 1;
                tracing::warn!(endpoint = $name, error = %e, "cache warming failed for endpoint");
            } else {
                $ok += 1;
            }
        };
    }

    let (mut succeeded, mut failed) = (0usize, 0usize);

    warm!("db.statistics",             state.db.statistics_cached().map(|r| r.map(|_| ())),                                                                                      succeeded, failed);
    warm!("db.featured_markets",       state.db.featured_markets_cached(state.config.featured_limit).map(|r| r.map(|_| ())),                                                     succeeded, failed);
    warm!("blockchain.health",         state.blockchain.health_check_cached().map(|r| r.map(|_| ())),                                                                             succeeded, failed);
    warm!("blockchain.platform_stats", state.blockchain.platform_statistics_cached().map(|r| r.map(|_| ())),                                                                     succeeded, failed);
    warm!("api.statistics",            statistics(State(state.clone())).map(|r| r.map(|_| ()).map_err(|e| anyhow::anyhow!("{e:?}"))),                                             succeeded, failed);
    warm!("api.featured_markets",      featured_markets(State(state.clone()), Query(PaginationQuery::default())).map(|r| r.map(|_| ()).map_err(|e| anyhow::anyhow!("{e:?}"))),   succeeded, failed);
    warm!("api.content",               content(State(state.clone()), Query(PaginationQuery::default())).map(|r| r.map(|_| ()).map_err(|e| anyhow::anyhow!("{e:?}"))),             succeeded, failed);

    tracing::info!(succeeded, failed, total = succeeded + failed, "cache warming complete");
    Ok(())
}

// Email service handlers

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct EmailTestRequest {
    pub recipient: String,
    pub template_name: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::IntoParams)]
pub struct EmailAnalyticsQuery {
    pub template_name: Option<String>,
    pub days: Option<i32>,
}

#[utoipa::path(
    get,
    path = "/api/v1/email/preview/{template_name}",
    tag = "email",
    params(
        ("template_name" = String, Path, description = "Email template name"),
    ),
    responses(
        (status = 200, description = "Rendered email HTML preview"),
        (status = 500, description = "Template render error", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn email_preview(
    State(state): State<Arc<AppState>>,
    Path(template_name): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let test_data = match template_name.as_str() {
        "newsletter_confirmation" => serde_json::json!({
            "confirm_url": format!("{}/api/v1/newsletter/confirm?token=test-token-123", state.config.base_url),
            "email": "test@example.com"
        }),
        "waitlist_confirmation" => serde_json::json!({
            "email": "test@example.com"
        }),
        "contact_form_auto_response" => serde_json::json!({
            "name": "Test User",
            "subject": "Test Subject",
            "message": "This is a test message."
        }),
        "welcome_email" => serde_json::json!({
            "name": "Test User",
            "dashboard_url": format!("{}/dashboard", state.config.base_url),
            "help_url": format!("{}/help", state.config.base_url),
            "unsubscribe_url": format!("{}/api/v1/newsletter/unsubscribe", state.config.base_url)
        }),
        _ => serde_json::json!({}),
    };

    let preview = state
        .email_service
        .preview_email(&template_name, &test_data)
        .map_err(into_api_error)?;

    Ok((StatusCode::OK, Json(preview)))
}

#[utoipa::path(
    post,
    path = "/api/v1/email/test",
    tag = "email",
    request_body = EmailTestRequest,
    responses(
        (status = 200, description = "Test email sent"),
        (status = 500, description = "Send failed", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn email_send_test(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<EmailTestRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let message_id = state
        .email_service
        .send_test_email(&payload.recipient, &payload.template_name)
        .await
        .map_err(into_api_error)?;

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "message": "Test email sent successfully",
            "message_id": message_id
        })),
    ))
}

#[utoipa::path(
    get,
    path = "/api/v1/email/analytics",
    tag = "email",
    params(EmailAnalyticsQuery),
    responses(
        (status = 200, description = "Email delivery analytics"),
        (status = 500, description = "Query failed", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn email_analytics(
    State(state): State<Arc<AppState>>,
    Query(query): Query<EmailAnalyticsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let days = query.days.unwrap_or(30).clamp(1, 365);
    let analytics = state
        .db
        .email_get_analytics(query.template_name.as_deref(), days)
        .await
        .map_err(into_api_error)?;

    Ok((StatusCode::OK, Json(analytics)))
}

#[utoipa::path(
    get,
    path = "/api/v1/email/queue/stats",
    tag = "email",
    responses(
        (status = 200, description = "Email queue statistics"),
        (status = 500, description = "Query failed", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn email_queue_stats(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state
        .email_queue
        .get_stats()
        .await
        .map_err(into_api_error)?;

    state.metrics.set_dlq_size(stats.dead_letter as i64);

    Ok((StatusCode::OK, Json(stats)))
}

#[utoipa::path(
    get,
    path = "/api/v1/email/queue/dead-letter",
    tag = "email",
    responses(
        (status = 200, description = "List of dead-letter email job IDs"),
        (status = 500, description = "Query failed", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn email_dead_letter_list(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let ids = state
        .email_queue
        .list_dead_letter()
        .await
        .map_err(into_api_error)?;

    Ok((StatusCode::OK, Json(serde_json::json!({ "jobs": ids, "count": ids.len() }))))
}

#[utoipa::path(
    post,
    path = "/api/v1/email/queue/dead-letter/{job_id}/requeue",
    tag = "email",
    params(
        ("job_id" = String, Path, description = "Dead-letter job UUID"),
    ),
    responses(
        (status = 200, description = "Job requeued"),
        (status = 404, description = "Job not found in dead-letter set", body = ApiError),
        (status = 500, description = "Requeue failed", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn email_dead_letter_requeue(
    State(state): State<Arc<AppState>>,
    Path(job_id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let requeued = state
        .email_queue
        .requeue_dead_letter(job_id)
        .await
        .map_err(into_api_error)?;

    if requeued {
        Ok((StatusCode::OK, Json(serde_json::json!({ "requeued": true, "job_id": job_id }))))
    } else {
        Err(ApiError::not_found(format!("Job {job_id} not found in dead-letter set")))
    }
}

#[utoipa::path(
    post,
    path = "/webhooks/sendgrid",
    tag = "webhooks",
    responses(
        (status = 200, description = "Events processed"),
        (status = 400, description = "Invalid signature or payload", body = ApiError),
    )
)]
pub async fn sendgrid_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(events): Json<Vec<crate::email::webhook::SendGridEvent>>,
) -> Result<impl IntoResponse, ApiError> {
    sendgrid_webhook_handler(State(Arc::new(state.webhook_handler.clone())), headers, Json(events))
        .await
        .map_err(|(status, msg)| ApiError {
            code: "WEBHOOK_ERROR",
            message: msg,
            status,
        })
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/logs",
    tag = "audit",
    params(AuditLogsQuery),
    responses(
        (status = 200, description = "Audit log entries"),
        (status = 500, description = "Query failed", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn audit_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuditLogsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    use chrono::{DateTime, Utc};
    
    let from = params.from.and_then(|s| s.parse::<DateTime<Utc>>().ok());
    let to = params.to.and_then(|s| s.parse::<DateTime<Utc>>().ok());
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);
    
    let logs = state
        .audit_logger
        .query(
            params.actor.as_deref(),
            params.action.as_deref(),
            params.resource_type.as_deref(),
            from,
            to,
            limit,
            offset,
        )
        .await
        .map_err(into_api_error)?;
    
    Ok((StatusCode::OK, Json(logs)))
}

#[utoipa::path(
    get,
    path = "/api/v1/audit/statistics",
    tag = "audit",
    params(AuditStatisticsQuery),
    responses(
        (status = 200, description = "Audit log statistics for the requested period"),
        (status = 500, description = "Query failed", body = ApiError),
    ),
    security(("api_key" = []))
)]
pub async fn audit_statistics(
    State(state): State<Arc<AppState>>,
    Query(params): Query<AuditStatisticsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    use chrono::{DateTime, Duration, Utc};
    
    let to = params
        .to
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(Utc::now);
    
    let from = params
        .from
        .and_then(|s| s.parse::<DateTime<Utc>>().ok())
        .unwrap_or_else(|| to - Duration::days(30));
    
    let stats = state
        .audit_logger
        .statistics(from, to)
        .await
        .map_err(into_api_error)?;
    
    Ok((StatusCode::OK, Json(stats)))
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct AuditLogsQuery {
    pub actor: Option<String>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, serde::Deserialize, utoipa::IntoParams)]
pub struct AuditStatisticsQuery {
    pub from: Option<String>,
    pub to: Option<String>,
}
