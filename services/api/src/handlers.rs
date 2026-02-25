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

use crate::{
    cache::keys,
    email::webhook::sendgrid_webhook_handler,
    AppState,
};

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

#[derive(Debug, Clone, Deserialize)]
pub struct FeaturedMarketsQuery {
    pub category: Option<String>,
    pub limit: Option<i64>,
    pub page: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturedMarketView {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub category: String,
    pub volume: f64,
    pub participant_count: i32,
    pub ends_at: chrono::DateTime<chrono::Utc>,
    pub outcome_options: serde_json::Value,
    pub current_odds: serde_json::Value,
    pub onchain_volume: String,
    pub resolved_outcome: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeaturedMarketsApiResponse {
    pub markets: Vec<FeaturedMarketView>,
    pub total: i64,
    pub page: i64,
    pub page_size: i64,
    pub last_updated: String,
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

    // Queue confirmation email instead of sending directly
    let confirm_url = format!(
        "{}/api/v1/newsletter/confirm?token={token}",
        state.config.base_url.trim_end_matches('/')
    );
    
    let template_data = serde_json::json!({
        "confirm_url": confirm_url,
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
    Query(query): Query<FeaturedMarketsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let start = Instant::now();
    let endpoint = "featured_markets";

    let category = query.category.as_deref();
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(8).clamp(1, 20);

    let cache_key = keys::api_featured_markets_with_params(category, page, limit);
    let ttl = Duration::from_secs(2 * 60);

    let (payload, hit) = state
        .cache
        .get_or_set_json(&cache_key, ttl, || async {
            let response = state
                .db
                .featured_markets_with_filters(category, page, limit)
                .await?;

            let chain_futures = response
                .markets
                .iter()
                .map(|m| state.blockchain.market_data_cached(m.id));
            let chain_data = join_all(chain_futures).await;

            let mut view = Vec::with_capacity(response.markets.len());
            for (m, chain_result) in response.markets.into_iter().zip(chain_data.into_iter()) {
                let chain = chain_result?;
                view.push(FeaturedMarketView {
                    id: m.id,
                    title: m.title,
                    description: m.description,
                    category: m.category,
                    volume: m.volume,
                    participant_count: m.participant_count,
                    ends_at: m.ends_at,
                    outcome_options: m.outcome_options,
                    current_odds: m.current_odds,
                    onchain_volume: chain.onchain_volume,
                    resolved_outcome: chain.resolved_outcome,
                });
            }

            Ok(FeaturedMarketsApiResponse {
                markets: view,
                total: response.total,
                page: response.page,
                page_size: response.page_size,
                last_updated: response.last_updated.to_rfc3339(),
            })
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

// Email service handlers

#[derive(Debug, Clone, Deserialize)]
pub struct EmailTestRequest {
    pub recipient: String,
    pub template_name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct EmailAnalyticsQuery {
    pub template_name: Option<String>,
    pub days: Option<i32>,
}

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

pub async fn email_queue_stats(
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse, ApiError> {
    let stats = state
        .email_queue
        .get_stats()
        .await
        .map_err(into_api_error)?;

    Ok((StatusCode::OK, Json(stats)))
}

pub async fn sendgrid_webhook(
    State(state): State<Arc<AppState>>,
    Json(events): Json<Vec<crate::email::webhook::SendGridEvent>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    sendgrid_webhook_handler(State(Arc::new(state.webhook_handler.clone())), Json(events)).await
}
