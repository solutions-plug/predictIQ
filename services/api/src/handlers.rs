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

use crate::{cache::keys, newsletter::send_confirmation_email, AppState};

#[derive(Debug, Serialize)]
pub struct ApiError {
    pub message: String,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (StatusCode::INTERNAL_SERVER_ERROR, Json(self)).into_response()
    }
}

fn into_api_error(err: anyhow::Error) -> ApiError {
    ApiError {
        message: err.to_string(),
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContentQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturedMarketView {
    pub id: i64,
    pub title: String,
    pub volume: f64,
    pub ends_at: chrono::DateTime<chrono::Utc>,
    pub onchain_volume: String,
    pub resolved_outcome: Option<u32>,
}

pub async fn health() -> impl IntoResponse {
    (StatusCode::OK, "ok")
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewsletterSubscribeRequest {
    pub email: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewsletterEmailRequest {
    pub email: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewsletterConfirmQuery {
    pub token: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewsletterExportQuery {
    pub email: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewsletterResponse {
    pub success: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct NewsletterExportResponse {
    pub success: bool,
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

fn client_ip(headers: &HeaderMap) -> String {
    if let Some(forwarded_for) = headers.get("x-forwarded-for").and_then(|h| h.to_str().ok()) {
        if let Some(ip) = forwarded_for.split(',').next() {
            let ip = ip.trim();
            if !ip.is_empty() {
                return ip.to_string();
            }
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip").and_then(|h| h.to_str().ok()) {
        let ip = real_ip.trim();
        if !ip.is_empty() {
            return ip.to_string();
        }
    }

    "unknown".to_string()
}

pub async fn newsletter_subscribe(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<NewsletterSubscribeRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let ip = client_ip(&headers);
    let allowed = state
        .newsletter_rate_limiter
        .allow(&ip, 5, Duration::from_secs(15 * 60))
        .await;

    if !allowed {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            Json(NewsletterResponse {
                success: false,
                message: "Too many requests, please try again later.".to_string(),
            }),
        ));
    }

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

    if let Some(existing) = state
        .db
        .newsletter_get_by_email(&email)
        .await
        .map_err(into_api_error)?
    {
        if existing.confirmed && existing.unsubscribed_at.is_none() {
            return Ok((
                StatusCode::CONFLICT,
                Json(NewsletterResponse {
                    success: false,
                    message: "Email already subscribed.".to_string(),
                }),
            ));
        }
    }

    let token = Uuid::new_v4().to_string();
    state
        .db
        .newsletter_upsert_pending(&email, &source, &token)
        .await
        .map_err(into_api_error)?;

    send_confirmation_email(&state.config, &email, &token)
        .await
        .map_err(into_api_error)?;

    tracing::info!("[newsletter] subscription attempt email={email} source={source} ip={ip}");

    Ok((
        StatusCode::OK,
        Json(NewsletterResponse {
            success: true,
            message: "Please check your email to confirm your subscription.".to_string(),
        }),
    ))
}

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
        .newsletter_confirm_by_token(query.token.trim())
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

pub async fn newsletter_unsubscribe(
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

pub async fn newsletter_gdpr_export(
    State(state): State<Arc<AppState>>,
    Query(query): Query<NewsletterExportQuery>,
) -> Result<Response, ApiError> {
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

#[derive(Debug, Clone, Deserialize)]
pub struct PagingQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

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

pub async fn featured_markets(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let start = Instant::now();
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

    if hit {
        state.metrics.observe_hit("api", endpoint);
    } else {
        state.metrics.observe_miss("api", endpoint);
    }
    state.metrics.observe_request(endpoint, start.elapsed());

    Ok((StatusCode::OK, Json(payload)))
}

pub async fn content(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ContentQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let start = Instant::now();
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query
        .page_size
        .unwrap_or(state.config.content_default_page_size)
        .clamp(1, 100);
    let endpoint = "content";

    let cache_key = keys::api_content(page, page_size);
    let ttl = Duration::from_secs(60 * 60);

    let (payload, hit) = state
        .cache
        .get_or_set_json(&cache_key, ttl, || async {
            let data = state.db.content_cached(page, page_size).await?;
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

#[derive(Debug, Serialize)]
pub struct InvalidationResult {
    pub invalidated_keys: usize,
}

pub async fn resolve_market(
    State(state): State<Arc<AppState>>,
    Path(market_id): Path<i64>,
) -> Result<impl IntoResponse, ApiError> {
    // This is a placeholder mutation endpoint for invalidation. Hook your real write flow here.
    let mut invalidated = 0usize;

    state
        .cache
        .del(&keys::chain_market(market_id))
        .await
        .map_err(into_api_error)?;
    invalidated += 1;

    let patterns = [
        keys::api_statistics(),
        keys::api_featured_markets(),
        format!("{}:*", keys::DBQ_PREFIX),
        format!("{}:*", keys::API_PREFIX),
    ];

    for p in patterns {
        let n = state
            .cache
            .del_by_pattern(&p)
            .await
            .map_err(into_api_error)?;
        invalidated += n;
    }

    state
        .metrics
        .observe_invalidation("market_write", invalidated);

    Ok((
        StatusCode::OK,
        Json(InvalidationResult {
            invalidated_keys: invalidated,
        }),
    ))
}

pub async fn metrics(State(state): State<Arc<AppState>>) -> Result<impl IntoResponse, ApiError> {
    let body = state.metrics.render().map_err(into_api_error)?;
    Ok((StatusCode::OK, body))
}

pub async fn blockchain_health(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let data = state
        .blockchain
        .health_check_cached()
        .await
        .map_err(into_api_error)?;
    Ok((StatusCode::OK, Json(data)))
}

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

pub async fn blockchain_user_bets(
    State(state): State<Arc<AppState>>,
    Path(user): Path<String>,
    Query(query): Query<PagingQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let page = query.page.unwrap_or(1).max(1);
    let page_size = query.page_size.unwrap_or(20).clamp(1, 100);
    let data = state
        .blockchain
        .user_bets_cached(&user, page, page_size)
        .await
        .map_err(into_api_error)?;
    Ok((StatusCode::OK, Json(data)))
}

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

pub async fn warm_critical_caches(state: Arc<AppState>) -> anyhow::Result<()> {
    let _ = state.db.statistics_cached().await?;
    let _ = state
        .db
        .featured_markets_cached(state.config.featured_limit)
        .await?;
    let _ = state.blockchain.health_check_cached().await?;
    let _ = state.blockchain.platform_statistics_cached().await?;
    let _ = statistics(State(state.clone())).await;
    let _ = featured_markets(State(state)).await;
    Ok(())
}
