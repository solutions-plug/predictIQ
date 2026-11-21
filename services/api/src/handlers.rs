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

use crate::{blockchain::HealthStatus, cache::{keys, InvalidationTag}, db::DbError, email::webhook::sendgrid_webhook_handler, pagination::{PaginatedResponse, PaginationQuery}, body_redact, AppState, batch_53_implementations::RequeueResult, validation};

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
        // Log the full error chain for debugging, then record it on the active OTel span
        // so traces carry the root cause even though the HTTP response is sanitised.
        tracing::error!(error = %err, "internal server error");
        {
            use tracing_opentelemetry::OpenTelemetrySpanExt;
            tracing::Span::current()
                .set_status(opentelemetry::trace::Status::error(format!("{err:#}")));
        }
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
                tracing::error!(db_constraint = %msg, "database constraint violation");
                return ApiError::conflict("A record with this value already exists.");
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
    /// Exact decimal string, matching the on-chain `onchain_volume` convention.
    pub volume: String,
    pub ends_at: chrono::DateTime<chrono::Utc>,
    pub onchain_volume: String,
    pub resolved_outcome: Option<u32>,
}

/// Legacy `/health` endpoint — retained for backward compatibility.
/// Returns 200 when healthy and 503 when any dependency is down.
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

    const HEALTH_TIMEOUT: Duration = Duration::from_secs(10);

    let inner = async {
        let request_id = headers
            .get(REQUEST_ID_HEADER)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("-");

        let (cb_state, cb_state_val) = match state.cache.circuit_state() {
            CircuitState::Closed => ("closed", 0),
            CircuitState::Open => ("open", 1),
            CircuitState::HalfOpen => ("half_open", 2),
        };
        let pool = state.cache.pool_status();
        state
            .metrics
            .set_circuit_breaker_state(cb_state_val);

        let mut health_status = serde_json::json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "request_id": request_id,
            "redis": {
                "circuit_breaker": cb_state,
                "pool_size": pool.size,
                "pool_available": pool.available,
                "status": "ok",
            },
            "db": {
                "status": "ok",
            },
            "workers": {
                "blockchain_sync": "running",
                "blockchain_monitor": "running",
                "email_queue": "running",
            }
        });

        let mut degraded = false;

        let (cache_result, db_result, processing_count_result, sendgrid_result) = tokio::join!(
            state.cache.ping(),
            state.db.ping(),
            state.email_queue.get_processing_count(),
            state.email_service.probe_sendgrid(),
        );

        if cache_result.is_err() {
            degraded = true;
            health_status["status"] = "degraded".into();
            health_status["redis"]["status"] = "unhealthy".into();
        }

    // Avoid live external probes on the legacy `/health` endpoint to prevent
    // unauthenticated amplification against third-party APIs (e.g. SendGrid).
    // The detailed dependency endpoints still perform live checks.
    health_status["sendgrid"] = serde_json::json!({ "status": "unknown" });

        if let Ok(processing_count) = processing_count_result {
            health_status["workers"]["email_queue_processing"] = processing_count.into();
        }

        let sendgrid_status = match sendgrid_result {
            Ok(()) => "ok",
            Err(e) => {
                tracing::warn!(error = %e, "SendGrid connectivity probe failed");
                degraded = true;
                health_status["status"] = "degraded".into();
                "degraded"
            }
        };
        health_status["sendgrid"] = serde_json::json!({ "status": sendgrid_status });

        let status_code = if degraded {
            StatusCode::SERVICE_UNAVAILABLE
        } else {
            StatusCode::OK
        };
        (status_code, Json(health_status))
    };

    match tokio::time::timeout(HEALTH_TIMEOUT, inner).await {
        Ok(response) => response,
        Err(_) => {
            tracing::warn!("health check timed out after {HEALTH_TIMEOUT:?}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(serde_json::json!({
                    "status": "degraded",
                    "reason": "health check timed out",
                })),
            )
        }
    }
}

/// Liveness probe: just confirms the process is alive and serving requests.
/// Never returns 503 — if this endpoint is reachable the process is up.
pub async fn health_live() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(serde_json::json!({
            "status": "ok",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        })),
    )
}

/// Readiness probe: verifies all critical dependencies are reachable.
///
/// Returns 503 if the database or Redis is unavailable.  Blockchain RPC
/// degradation is surfaced in the body but does not affect the status code
/// because the API can continue to serve cached data without it.
pub async fn health_ready(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> impl IntoResponse {
    use crate::correlation::REQUEST_ID_HEADER;

    let request_id = headers
        .get(REQUEST_ID_HEADER)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("-");

    // ── Database ──────────────────────────────────────────────────────────────
    let db_ok = state.db.ping().await.is_ok();

    // ── Redis: direct PING bypassing circuit breaker, with latency ───────────
    let redis_result = state.cache.ping_direct_ms().await;
    let redis_latency_ms = redis_result.as_ref().ok().copied().unwrap_or(0);
    let redis_ok = redis_result.is_ok();
    let redis_degraded = redis_ok && redis_latency_ms > 100;

    // ── Blockchain RPC (cached; does not block readiness) ─────────────────────
    let rpc_health = state.blockchain.health_check_cached().await.ok();
    let rpc_status = match &rpc_health {
        Some(h) if h.is_healthy => "ok",
        Some(_) => "degraded",
        None => "unknown",
    };

    let all_critical_ok = db_ok && redis_ok;
    let overall_status = if !all_critical_ok {
        "unavailable"
    } else if redis_degraded {
        "degraded"
    } else {
        "ok"
    };

    let status_code = if all_critical_ok {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    (
        status_code,
        Json(serde_json::json!({
            "status": overall_status,
            "request_id": request_id,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "dependencies": {
                "database": {
                    "status": if db_ok { "ok" } else { "unavailable" }
                },
                "redis": {
                    "status": if !redis_ok { "unavailable" } else if redis_degraded { "degraded" } else { "ok" },
                    "latency_ms": redis_latency_ms,
                },
                "blockchain_rpc": {
                    "status": rpc_status,
                }
            }
        })),
    )
}

/// Dependency details endpoint: structured per-dependency health with latency.
///
/// Returns the same data as `/health/ready` but always returns 200 so
/// Prometheus scrape targets and monitoring dashboards can always collect
/// the data even when dependencies are degraded.
pub async fn health_dependencies(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let db_start = Instant::now();
    let db_ok = state.db.ping().await.is_ok();
    let db_latency_ms = db_start.elapsed().as_millis();

    let redis_result = state.cache.ping_direct_ms().await;
    let redis_latency_ms = redis_result.as_ref().ok().copied().unwrap_or(0);
    let redis_ok = redis_result.is_ok();
    let redis_status = if !redis_ok {
        "unavailable"
    } else if redis_latency_ms > 100 {
        "degraded"
    } else {
        "ok"
    };

    let rpc_health = state.blockchain.health_check_cached().await.ok();
    let rpc_status = match &rpc_health {
        Some(h) if h.is_healthy => "ok",
        Some(_) => "degraded",
        None => "unknown",
    };
    let rpc_latest_ledger = rpc_health.as_ref().map(|h| h.latest_ledger).unwrap_or(0);

    (
        StatusCode::OK,
        Json(serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "dependencies": [
                {
                    "name": "database",
                    "status": if db_ok { "ok" } else { "unavailable" },
                    "latency_ms": db_latency_ms,
                },
                {
                    "name": "redis",
                    "status": redis_status,
                    "latency_ms": redis_latency_ms,
                },
                {
                    "name": "blockchain_rpc",
                    "status": rpc_status,
                    "latest_ledger": rpc_latest_ledger,
                },
            ]
        })),
    )
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct NewsletterSubscribeRequest {
    pub email: String,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct NewsletterEmailRequest {
    pub email: String,
    /// Signed verification token obtained from `/gdpr/request-token`.
    pub token: String,
}

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct NewsletterGdprTokenRequest {
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

#[derive(Debug, Clone, Deserialize, utoipa::ToSchema)]
pub struct NewsletterExportBody {
    pub email: String,
    /// Signed verification token obtained from `/gdpr/request-token`.
    pub token: String,
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

/// Verify a GDPR verification token for `email`.
///
/// Returns `Some(response)` when the caller should be short-circuited
/// (missing signing config, or an invalid/expired/mismatched token) and
/// `None` when the token checks out and the request may proceed.
fn check_gdpr_token(state: &AppState, email: &str, token: &str) -> Option<Response> {
    let Some(secret) = state.config.unsubscribe_signing_secret.as_deref() else {
        return Some(
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(NewsletterResponse {
                    success: false,
                    message: "GDPR verification not configured.".to_string(),
                }),
            )
                .into_response(),
        );
    };

    if !crate::newsletter::verify_gdpr_verification_token(token, email, secret) {
        return Some(
            (
                StatusCode::UNAUTHORIZED,
                Json(NewsletterResponse {
                    success: false,
                    message: "Invalid or expired verification token.".to_string(),
                }),
            )
                .into_response(),
        );
    }

    None
}

/// Built-in disposable domains; extend at runtime via the comma-separated
/// `DISPOSABLE_EMAIL_DOMAINS` env var. Subdomains of a listed domain match.
const DISPOSABLE_DOMAINS: &[&str] = &[
    "mailinator.com",
    "tempmail.com",
    "guerrillamail.com",
    "10minutemail.com",
    "yopmail.com",
    "trashmail.com",
    "throwawaymail.com",
    "sharklasers.com",
    "getnada.com",
    "dispostable.com",
];

fn is_disposable_email(email: &str) -> bool {
    let Some((_, domain)) = email.rsplit_once('@') else {
        return false;
    };
    let domain = domain.trim_end_matches('.').to_ascii_lowercase();
    let extra = std::env::var("DISPOSABLE_EMAIL_DOMAINS").unwrap_or_default();
    DISPOSABLE_DOMAINS
        .iter()
        .copied()
        .chain(extra.split(',').map(str::trim).filter(|d| !d.is_empty()))
        .any(|d| {
            let d = d.to_ascii_lowercase();
            domain == d || domain.ends_with(&format!(".{d}"))
        })
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

    // Disposable addresses get the same uniform 202 as everyone else (no
    // enumeration of the blocklist); nothing is stored or sent.
    let disposable = is_disposable_email(&email);
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

    // Per-recipient cooldown: at most one confirmation email per address per hour,
    // regardless of how many client IPs are used. Fails open if Redis is down.
    let throttled = if already_active || disposable {
        false
    } else {
        use sha2::{Digest, Sha256};
        let key = format!(
            "newsletter:confirm_cooldown:{}",
            hex::encode(Sha256::digest(email.as_bytes()))
        );
        state
            .cache
            .incr_with_ttl(&key, std::time::Duration::from_secs(3600))
            .await
            .map(|n| n > 1)
            .unwrap_or(false)
    };

    if !already_active && !disposable && !throttled {
        let token = crate::newsletter::generate_opaque_unsubscribe_token().0;
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
    let redacted_email = crate::body_redact::redact_email(&email);
    tracing::info!(request_id, email = %redacted_email, source = %source, ip = %ip, "newsletter subscription attempt");

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

    let redacted_email = crate::body_redact::redact_email(&email);
    tracing::info!("[newsletter] unsubscribed email={redacted_email}");

    Ok((
        StatusCode::OK,
        Json(NewsletterResponse {
            success: true,
            message: "Successfully unsubscribed.".to_string(),
        }),
    ))
}

#[utoipa::path(
    post,
    path = "/api/v1/newsletter/gdpr/request-token",
    tag = "newsletter",
    request_body = NewsletterGdprTokenRequest,
    responses(
        (status = 200, description = "Verification email sent if the address is subscribed", body = NewsletterResponse),
        (status = 400, description = "Invalid email", body = NewsletterResponse),
        (status = 429, description = "Rate limited", body = NewsletterResponse),
    )
)]
pub async fn newsletter_gdpr_request_token(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    connect_info: Option<axum::extract::ConnectInfo<std::net::SocketAddr>>,
    Json(payload): Json<NewsletterGdprTokenRequest>,
) -> Result<Response, ApiError> {
    use crate::security::extract_client_ip_cidrs;
    let ip = extract_client_ip_cidrs(
        &headers,
        connect_info.as_ref(),
        state.config.trust_proxy,
        &state.config.trusted_proxy_cidrs,
    );
    let allowed = state
        .newsletter_rate_limiter
        .allow(
            &format!("gdpr_token:ip:{ip}"),
            state.config.gdpr_export_rate_limit as usize,
            std::time::Duration::from_secs(state.config.gdpr_export_rate_window_secs),
        )
        .await;
    if !allowed {
        return Ok((
            StatusCode::TOO_MANY_REQUESTS,
            Json(NewsletterResponse {
                success: false,
                message: "Too many requests, please try again later.".to_string(),
            }),
        )
            .into_response());
    }

    let Some(email) = normalized_email(&payload.email) else {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(NewsletterResponse {
                success: false,
                message: "Invalid email address.".to_string(),
            }),
        )
            .into_response());
    };

    // Never disclose whether the address exists: only send an email when a
    // record is found, but return the same generic response either way.
    match state.db.newsletter_get_by_email(&email).await {
        Ok(Some(_)) => {
            if let Some(secret) = state.config.unsubscribe_signing_secret.as_deref() {
                let token = crate::newsletter::generate_gdpr_verification_token(&email, secret);
                if let Err(e) =
                    crate::newsletter::send_gdpr_verification_email(&state.config, &email, &token)
                        .await
                {
                    tracing::warn!(error = %e, "[newsletter] failed to send GDPR verification email");
                }
            } else {
                tracing::warn!("[newsletter] GDPR verification requested but no signing secret configured");
            }
        }
        Ok(None) => {}
        Err(e) => tracing::warn!(error = %e, "[newsletter] gdpr token lookup failed"),
    }

    Ok((
        StatusCode::OK,
        Json(NewsletterResponse {
            success: true,
            message: "If this email is subscribed, a verification code has been sent."
                .to_string(),
        }),
    )
        .into_response())
}

#[utoipa::path(
    post,
    path = "/api/v1/newsletter/gdpr/export",
    tag = "newsletter",
    request_body = NewsletterExportBody,
    responses(
        (status = 200, description = "GDPR data export", body = NewsletterExportResponse),
        (status = 400, description = "Invalid email", body = NewsletterResponse),
        (status = 401, description = "Invalid or expired verification token", body = NewsletterResponse),
        (status = 404, description = "No record found", body = NewsletterResponse),
        (status = 429, description = "Rate limited", body = NewsletterResponse),
    )
)]
pub async fn newsletter_gdpr_export(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    connect_info: Option<axum::extract::ConnectInfo<std::net::SocketAddr>>,
    Json(body): Json<NewsletterExportBody>,
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

    let Some(email) = normalized_email(&body.email) else {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(NewsletterResponse {
                success: false,
                message: "Invalid email address.".to_string(),
            }),
        )
            .into_response());
    };

    if let Some(resp) = check_gdpr_token(&state, &email, &body.token) {
        return Ok(resp);
    }

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
        (status = 401, description = "Invalid or expired verification token", body = NewsletterResponse),
    )
)]
pub async fn newsletter_gdpr_delete(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<NewsletterEmailRequest>,
) -> Result<Response, ApiError> {
    let Some(email) = normalized_email(&payload.email) else {
        return Ok((
            StatusCode::BAD_REQUEST,
            Json(NewsletterResponse {
                success: false,
                message: "Invalid email address.".to_string(),
            }),
        )
            .into_response());
    };

    if let Some(resp) = check_gdpr_token(&state, &email, &payload.token) {
        return Ok(resp);
    }

    let _ = state
        .db
        .newsletter_gdpr_delete(&email)
        .await
        .map_err(into_api_error)?;

    let redacted_email = crate::body_redact::redact_email(&email);
    tracing::info!("[newsletter] gdpr delete email={redacted_email}");

    Ok((
        StatusCode::OK,
        Json(NewsletterResponse {
            success: true,
            message: "Data deleted.".to_string(),
        }),
    )
        .into_response())
}

/// Revoke an API key by id.
///
/// Sets `revoked_at` on the target key so that subsequent requests
/// authenticated with it are rejected. Revoking an already-revoked key is
/// idempotent and returns success.
#[utoipa::path(
    post,
    path = "/api/v1/admin/api-keys/{id}/revoke",
    tag = "admin",
    params(
        ("id" = Uuid, Path, description = "API key id to revoke"),
    ),
    responses(
        (status = 200, description = "API key revoked"),
        (status = 404, description = "API key not found", body = ApiError),
    )
)]
pub async fn revoke_api_key(
    State(state): State<Arc<AppState>>,
    Path(id): Path<Uuid>,
) -> Result<impl IntoResponse, ApiError> {
    let revoked = state
        .db
        .revoke_api_key(id)
        .await
        .map_err(into_api_error)?;

    if !revoked {
        return Err(ApiError::not_found("API key not found."));
    }

    tracing::info!(api_key_id = %id, "api key revoked");

    Ok((
        StatusCode::OK,
        Json(serde_json::json!({
            "success": true,
            "id": id,
            "revoked": true,
        })),
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
/// Validate a user address path parameter before it is used in cache keys or RPC calls.
fn validate_user_address_param(address: &str) -> Result<(), ApiError> {
    validation::validate_user_address(address)
        .map_err(|e| ApiError::bad_request(e.to_string()))
}

/// Validate a market id path parameter before it is used in cache keys or RPC calls.
fn validate_market_id_param(market_id: &str) -> Result<i64, ApiError> {
    validation::validate_market_id_str(market_id)
        .map_err(|e| ApiError::bad_request(e.to_string()))
}

#[utoipa::path(
    get,
    path = "/api/v1/users/{address}/positions",
    tag = "users",
    params(
        ("address" = String, Path, description = "User Stellar address"),
    ),
    responses(
        (status = 200, description = "User positions"),
        (status = 400, description = "Invalid user address", body = ApiError),
    )
)]
pub async fn user_positions(
    State(state): State<Arc<AppState>>,
    Path(address): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Reject malformed addresses before any cache lookup or RPC call.
    validate_user_address_param(&address)?;

    let start = Instant::now();
    let cache_key = keys::api_user_positions(&address);
    let ttl = Duration::from_secs(60);
    let endpoint = "user_positions";

    let (payload, hit) = state
        .cache
        .get_or_set_json(&cache_key, ttl, || async {
            let positions = state.db.user_positions_cached(&address).await?;
            Ok(positions)
        })
        .await
        .map_err(into_api_error)?;

    if hit {
        state.metrics.observe_hit("api", endpoint);
    } else {
        state.metrics.observe_miss("api", endpoint);
    }
    state.metrics.observe_request(endpoint, 200, start.elapsed().as_secs_f64());

    Ok((StatusCode::OK, Json(payload)))
}

#[utoipa::path(
    get,
    path = "/api/v1/markets/{market_id}",
    tag = "markets",
    params(
        ("market_id" = String, Path, description = "Market id"),
    ),
    responses(
        (status = 200, description = "Market details"),
        (status = 400, description = "Invalid market id", body = ApiError),
        (status = 404, description = "Market not found", body = ApiError),
    )
)]
pub async fn market_detail(
    State(state): State<Arc<AppState>>,
    Path(market_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    // Reject malformed market ids before any cache lookup or RPC call.
    let market_id = validate_market_id_param(&market_id)?;

    let start = Instant::now();
    let cache_key = keys::api_market_detail(market_id);
    let ttl = Duration::from_secs(60);
    let endpoint = "market_detail";

    let (payload, hit) = state
        .cache
        .get_or_set_json(&cache_key, ttl, || async {
            let market = state
                .db
                .market_by_id_cached(market_id)
                .await?
                .ok_or_else(|| anyhow::anyhow!("market not found"))?;
            Ok(market)
        })
        .await
        .map_err(into_api_error)?;

    if hit {
        state.metrics.observe_hit("api", endpoint);
    } else {
        state.metrics.observe_miss("api", endpoint);
    }
    state.metrics.observe_request(endpoint, 200, start.elapsed().as_secs_f64());

    Ok((StatusCode::OK, Json(payload)))
}

#[utoipa::path(
    get,
    path = "/api/v1/admin/dead-letter-jobs",
    tag = "admin",
    params(PaginationQuery),
    responses(
        (status = 200, description = "Paginated list of dead-letter jobs"),
        (status = 400, description = "Invalid pagination parameters", body = ApiError),
    )
)]
pub async fn dead_letter_jobs(
    State(state): State<Arc<AppState>>,
    Query(query): Query<PaginationQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let start = Instant::now();
    let limit = query.limit();
    let offset = query.offset();
    let endpoint = "dead_letter_jobs";

    let (jobs, total) = state
        .db
        .dead_letter_jobs_paginated(limit, offset)
        .await
        .map_err(into_api_error)?;

    let response = PaginatedResponse::new(jobs, total, limit, offset);
    state.metrics.observe_request(endpoint, 200, start.elapsed().as_secs_f64());

    Ok((StatusCode::OK, Json(response)))
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
    state.metrics.observe_request(endpoint, 200, start.elapsed().as_secs_f64());

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
    // Validate pagination inputs before they reach cache keys or RPC calls.
    validation::validate_market_id(limit as i64)
        .map_err(|e| ApiError::bad_request(e.to_string()))?;
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
        .map_err
