use std::{sync::Arc, time::Duration};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::security::{extract_client_ip, RateLimitConfig, RateLimiter, TrustProxy};

/// Newsletter endpoint rate limiting (5 req/hour per IP)
pub async fn newsletter_rate_limit_middleware(
    State((limiter, TrustProxy(trust_proxy))): State<(Arc<RateLimiter>, TrustProxy)>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&headers, connect_info.as_ref(), trust_proxy);
    let config = RateLimitConfig::new(5, Duration::from_secs(3600));

    if !limiter.check(&format!("newsletter:{}", ip), &config).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Contact endpoint rate limiting (3 req/hour per IP)
pub async fn contact_rate_limit_middleware(
    State((limiter, TrustProxy(trust_proxy))): State<(Arc<RateLimiter>, TrustProxy)>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&headers, connect_info.as_ref(), trust_proxy);
    let config = RateLimitConfig::new(3, Duration::from_secs(3600));

    if !limiter.check(&format!("contact:{}", ip), &config).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Analytics endpoint rate limiting (1000 req/min per session)
pub async fn analytics_rate_limit_middleware(
    State((limiter, TrustProxy(trust_proxy))): State<(Arc<RateLimiter>, TrustProxy)>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Use session ID if available, otherwise fall back to IP
    let ip;
    let session_key = match headers.get("x-session-id").and_then(|h| h.to_str().ok()) {
        Some(id) => id.to_owned(),
        None => {
            ip = extract_client_ip(&headers, connect_info.as_ref());
            ip
        }
    };

    let config = RateLimitConfig::new(1000, Duration::from_secs(60));

    if !limiter
        .check(&format!("analytics:{}", session_key), &config)
        .await
    {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}

/// Admin endpoint rate limiting (stricter limits)
pub async fn admin_rate_limit_middleware(
    State((limiter, TrustProxy(trust_proxy))): State<(Arc<RateLimiter>, TrustProxy)>,
    headers: HeaderMap,
    connect_info: Option<ConnectInfo<std::net::SocketAddr>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let ip = extract_client_ip(&headers, connect_info.as_ref(), trust_proxy);
    let config = RateLimitConfig::new(30, Duration::from_secs(60));

    if !limiter.check(&format!("admin:{}", ip), &config).await {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(request).await)
}
