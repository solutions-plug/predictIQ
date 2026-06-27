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

## Migration Guide

### v1 → (upcoming v2)

No v2 is available yet. When released, a migration guide will be published here. Subscribe to the changelog to be notified.

## Server-Side Warnings

The API server logs a `WARN`-level message for every request that hits a deprecated version endpoint. Monitor your observability platform for these warnings to track client migration progress.
