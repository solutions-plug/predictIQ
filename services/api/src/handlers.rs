use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures::future::join_all;
use serde::{Deserialize, Serialize};

use crate::{cache::keys, AppState};

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

pub async fn warm_critical_caches(state: Arc<AppState>) -> anyhow::Result<()> {
    let _ = state.db.statistics_cached().await?;
    let _ = state
        .db
        .featured_markets_cached(state.config.featured_limit)
        .await?;
    let _ = statistics(State(state.clone())).await;
    let _ = featured_markets(State(state)).await;
    Ok(())
}
