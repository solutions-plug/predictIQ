//! CSRF protection middleware for the PredictIQ API.
//!
//! # Why CSRF matters here (and where it does not)
//!
//! Cross-Site Request Forgery (CSRF) exploits the browser's automatic inclusion of
//! cookies in cross-origin requests.  It is only relevant when **both** of the
//! following are true:
//!
//! 1. The endpoint mutates state (POST / PUT / PATCH / DELETE).
//! 2. Authentication is carried by a **cookie** that the browser will include
//!    automatically for any origin.
//!
//! ## Admin routes — out of scope
//!
//! Admin routes use `X-Api-Key` header authentication, not cookies.  A browser
//! cannot be tricked into attaching a custom `X-Api-Key` header via a forged
//! cross-site form submission or `<img>` tag, so CSRF does not apply to those
//! routes.  The middleware short-circuits immediately when `X-Api-Key` is
//! present.
//!
//! ## Newsletter subscribe — primary in-scope endpoint
//!
//! `POST /api/v1/newsletter/subscribe` requires `Content-Type: application/json`.
//! HTML `<form>` elements can only submit `application/x-www-form-urlencoded` or
//! `multipart/form-data`, so a plain cross-site form attack is already blocked by
//! the `content_type_validation_middleware`.  This middleware adds a second layer
//! of defense-in-depth by rejecting requests whose `Origin` header does not match
//! the list of allowed origins.
//!
//! ## Unsubscribe / GDPR endpoints — out of scope
//!
//! The unsubscribe endpoint is a `GET` that takes an opaque URL token
//! (`?token=...`).  GET requests do not mutate state in the traditional sense
//! (the action is triggered by following a link), and the token acts as a
//! per-user secret that cannot be guessed by a cross-site attacker.  This
//! middleware only inspects state-changing methods.
//!
//! ## Defense-in-depth summary
//!
//! | Layer | Mechanism |
//! |-------|-----------|
//! | 1 | JSON `Content-Type` requirement blocks HTML-form CSRF |
//! | 2 | `Origin` / `Referer` validation (this middleware) rejects cross-origin browser requests |
//! | 3 | No cookie-based session auth means there is no credential for CSRF to abuse |

use std::sync::Arc;

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

/// Configuration for the CSRF protection middleware.
///
/// Construct this from the application's CORS allowed-origins list so that
/// the two lists stay in sync.
#[derive(Clone)]
pub struct CsrfConfig {
    /// Origins that are permitted to make state-changing requests, e.g.
    /// `["https://app.predictiq.com", "https://staging.predictiq.com"]`.
    ///
    /// Values are matched case-insensitively against the `Origin` header.
    pub allowed_origins: Vec<String>,
}

/// CSRF protection middleware.
///
/// ## Logic
///
/// 1. **API-key requests** — pass through immediately.  `X-Api-Key`
///    authentication is not susceptible to CSRF because browsers cannot attach
///    custom headers via forged cross-site requests.
///
/// 2. **Safe methods** (GET, HEAD, OPTIONS, TRACE) — pass through.  These
///    should not perform state-changing operations; mutations must use an
///    appropriate HTTP method.
///
/// 3. **State-changing methods** (POST, PUT, PATCH, DELETE):
///    - If an `Origin` header is present and does **not** match any configured
///      allowed origin → **403 Forbidden**.  This covers the most common CSRF
///      vector: a browser-originated cross-site request that includes the
///      `Origin` header automatically.
///    - If the request carries a `Cookie` header (indicating a browser context)
///      but has **no** `Origin` header, the `Referer` header is checked as a
///      fallback.  If `Referer` is present and does not start with any allowed
///      origin → **403 Forbidden**.
///    - Requests with neither `Origin` nor `Cookie` are non-browser / API
///      clients and are allowed through.
pub async fn csrf_protection_middleware(
    State(config): State<Arc<CsrfConfig>>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // ── 1. API-key requests are not cookie-based — CSRF does not apply ────────
    if headers.contains_key("x-api-key") {
        return Ok(next.run(request).await);
    }

    // ── 2. Safe (read-only) methods — no state change, skip check ─────────────
    let method = request.method().clone();
    let is_state_changing = matches!(
        method.as_str(),
        "POST" | "PUT" | "PATCH" | "DELETE"
    );
    if !is_state_changing {
        return Ok(next.run(request).await);
    }

    // ── 3. State-changing methods: validate Origin / Referer ──────────────────

    // Helper: check whether `value` starts with (or equals) any allowed origin.
    let is_allowed = |value: &str| -> bool {
        let value_lc = value.to_lowercase();
        config.allowed_origins.iter().any(|allowed| {
            let allowed_lc = allowed.to_lowercase();
            // Exact match or the value starts with the allowed origin followed
            // by '/' (to handle paths appended to the origin).
            value_lc == allowed_lc
                || value_lc.starts_with(&format!("{}/", allowed_lc))
        })
    };

    // Check the `Origin` header first — browsers always send this on
    // cross-origin requests and on same-origin requests with CORS preflight.
    if let Some(origin_val) = headers.get("origin").and_then(|v| v.to_str().ok()) {
        if !is_allowed(origin_val) {
            tracing::warn!(
                origin = %origin_val,
                method = %method,
                "CSRF: rejected cross-origin state-changing request"
            );
            return Err(StatusCode::FORBIDDEN);
        }
        // Origin present and allowed — pass through.
        return Ok(next.run(request).await);
    }

    // No `Origin` header.  If a `Cookie` header is present the request comes
    // from a browser context; fall back to `Referer` validation.
    if headers.contains_key("cookie") {
        if let Some(referer_val) = headers.get("referer").and_then(|v| v.to_str().ok()) {
            if !is_allowed(referer_val) {
                tracing::warn!(
                    referer = %referer_val,
                    method = %method,
                    "CSRF: rejected request with non-matching Referer and Cookie present"
                );
                return Err(StatusCode::FORBIDDEN);
            }
        }
        // Referer absent or allowed — pass through.
    }

    // Non-browser / API client (no Origin, no Cookie) — pass through.
    Ok(next.run(request).await)
}
