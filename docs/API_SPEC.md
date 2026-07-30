# PredictIQ API Quick Reference

**Last Updated:** 2026-06-30  
**Spec source:** [`services/api/openapi.yaml`](../services/api/openapi.yaml)

## Public Routes

| Method | Path | Operation | Auth |
|--------|------|-----------|------|
| GET | `/health` | `getHealth` | None |
| GET | `/api/v1/statistics` | `getStatistics` | None |
| GET | `/api/v1/markets/featured` | `getFeaturedMarkets` | None |
| GET | `/api/v1/content` | `getContent` | None |
| GET | `/api/v1/blockchain/health` | `getBlockchainHealth` | None |
| GET | `/api/v1/blockchain/markets/{market_id}` | `getBlockchainMarket` | None |
| GET | `/api/v1/blockchain/stats` | `getBlockchainStats` | None |
| GET | `/api/v1/blockchain/users/{user}/bets` | `getUserBets` | None |
| GET | `/api/v1/blockchain/oracle/{market_id}` | `getOracleResult` | None |
| GET | `/api/v1/blockchain/tx/{tx_hash}` | `getTransactionStatus` | None |

## Newsletter Routes

| Method | Path | Operation | Auth |
|--------|------|-----------|------|
| POST | `/api/v1/newsletter/subscribe` | `newsletterSubscribe` | None |
| GET | `/api/v1/newsletter/confirm` | `newsletterConfirm` | None |
| DELETE | `/api/v1/newsletter/unsubscribe` | `newsletterUnsubscribe` | None |
| GET | `/api/v1/newsletter/gdpr/export` | `newsletterGdprExport` | None |
| DELETE | `/api/v1/newsletter/gdpr/delete` | `newsletterGdprDelete` | None |

## Admin Routes (require `X-API-Key`)

| Method | Path | Operation | Auth |
|--------|------|-----------|------|
| POST | `/api/v1/markets/{market_id}/resolve` | `resolveMarket` | ApiKeyAuth |
| POST | `/api/blockchain/replay` | `blockchainReplay` | ApiKeyAuth |
| GET | `/api/v1/email/preview/{template_name}` | `emailPreview` | ApiKeyAuth |
| POST | `/api/v1/email/test` | `emailSendTest` | ApiKeyAuth |
| GET | `/api/v1/email/analytics` | `getEmailAnalytics` | ApiKeyAuth |
| GET | `/api/v1/email/queue/stats` | `getEmailQueueStats` | ApiKeyAuth |
| GET | `/api/v1/email/queue/dead-letter` | `getEmailDeadLetterList` | ApiKeyAuth |
| POST | `/api/v1/email/queue/dead-letter/{job_id}/requeue` | `requeueEmailDeadLetterJob` | ApiKeyAuth |
| GET | `/api/v1/audit/logs` | `getAuditLogs` | ApiKeyAuth |
| GET | `/api/v1/audit/statistics` | `getAuditStatistics` | ApiKeyAuth |

## Webhook Routes

| Method | Path | Operation | Auth |
|--------|------|-----------|------|
| POST | `/webhooks/sendgrid` | `sendgridWebhook` | SendGrid HMAC signature |

## Metrics

| Method | Path | Operation | Auth |
|--------|------|-----------|------|
| GET | `/metrics` | — | Configurable (public / IP allowlist / ApiKeyAuth) |
