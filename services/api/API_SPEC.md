# API Versioning & Deprecation Schedule

## Current Version

| Version | Status    | Sunset Date              |
|---------|-----------|--------------------------|
| v1      | Deprecated | Sat, 25 Apr 2026 00:00 UTC |

## Deprecation Policy

Deprecated API versions include two response headers per [RFC 8594](https://www.rfc-editor.org/rfc/rfc8594):

- **`Deprecation: true`** — signals that the endpoint is deprecated.
- **`Sunset: <HTTP-date>`** — the date after which the version will be removed.
- **`Link: </api/v1>; rel="deprecation"; type="text/html"`** — points to migration docs.

Clients must migrate before the sunset date. After sunset, requests to removed versions will receive `410 Gone`.

## Version Selection

Send the `API-Version` header to select a specific version:

```
API-Version: v1
```

Omitting the header defaults to the current version.

## Rate Limits

Requests are limited to **100 requests per minute per IP address**. The limit is keyed on the client IP address, not on the API key — multiple API keys used from the same IP share a single budget.

The enforced value is defined by the `RATE_LIMIT_PER_MINUTE` constant in [`src/rate_limit.rs`](src/rate_limit.rs). The behaviour is pinned by [`tests/rate_limiting_tests.rs`](tests/rate_limiting_tests.rs); if the constant and this document ever disagree, the tests are authoritative.

## Migration Guide

### v1 → (upcoming v2)

No v2 is available yet. When released, a migration guide will be published here. Subscribe to the changelog to be notified.

## Server-Side Warnings

The API server logs a `WARN`-level message for every request that hits a deprecated version endpoint. Monitor your observability platform for these warnings to track client migration progress.
