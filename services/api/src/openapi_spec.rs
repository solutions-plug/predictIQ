use utoipa::OpenApi;

use crate::handlers::{
    ApiError, AuditLogsQuery, AuditStatisticsQuery, EmailAnalyticsQuery, EmailTestRequest,
    FeaturedMarketView, InvalidationResult, NewsletterEmailRequest, NewsletterExportResponse,
    NewsletterResponse, NewsletterSubscribeRequest, ResolveMarketRequest,
    NewsletterConfirmQuery, NewsletterUnsubscribeQuery, NewsletterExportQuery,
};
use crate::pagination::PaginationQuery;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "PredictIQ API",
        version = "1.0.0",
        description = "REST API for the PredictIQ prediction markets platform.\n\
            \n\
            ## API Versioning\n\
            The API uses URL path versioning (`/api/v1/`). The current stable version is **v1**.\n\
            \n\
            ## Deprecation Policy\n\
            When a version is deprecated, responses include a `Deprecation` header. \
            Deprecated versions are supported for a minimum of 12 months.",
    ),
    paths(
        crate::handlers::health,
        crate::handlers::newsletter_subscribe,
        crate::handlers::newsletter_confirm,
        crate::handlers::newsletter_unsubscribe,
        crate::handlers::newsletter_gdpr_export,
        crate::handlers::newsletter_gdpr_delete,
        crate::handlers::statistics,
        crate::handlers::featured_markets,
        crate::handlers::content,
        crate::handlers::resolve_market,
        crate::handlers::blockchain_health,
        crate::handlers::blockchain_market_data,
        crate::handlers::blockchain_platform_stats,
        crate::handlers::blockchain_user_bets,
        crate::handlers::blockchain_oracle_result,
        crate::handlers::blockchain_tx_status,
        crate::handlers::blockchain_replay,
        crate::handlers::email_preview,
        crate::handlers::email_send_test,
        crate::handlers::email_analytics,
        crate::handlers::email_queue_stats,
        crate::handlers::email_dead_letter_list,
        crate::handlers::email_dead_letter_requeue,
        crate::handlers::sendgrid_webhook,
        crate::handlers::audit_logs,
        crate::handlers::audit_statistics,
    ),
    components(
        schemas(
            ApiError,
            FeaturedMarketView,
            InvalidationResult,
            NewsletterSubscribeRequest,
            NewsletterEmailRequest,
            NewsletterResponse,
            NewsletterExportResponse,
            ResolveMarketRequest,
            EmailTestRequest,
        )
    ),
    tags(
        (name = "health", description = "Health check"),
        (name = "newsletter", description = "Newsletter subscription management"),
        (name = "markets", description = "Market data and resolution"),
        (name = "blockchain", description = "Stellar blockchain integration"),
        (name = "email", description = "Email service management (admin)"),
        (name = "webhooks", description = "Incoming provider webhooks"),
        (name = "audit", description = "Audit log access (admin)"),
    ),
    security(
        ("api_key" = [])
    )
)]
pub struct ApiDoc;
