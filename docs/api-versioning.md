# API Versioning

## Current Versions

| Version | Status     | Sunset Date      |
|---------|------------|------------------|
| v1      | Deprecated | 2026-12-31       |
| v2      | Planned    | —                |

## v1 Sunset Timeline

- **Deprecated:** May 2026 — `Deprecation: true` and `Sunset` headers added to all v1 responses.
- **Sunset:** 2026-12-31 — v1 endpoints will be removed.
- **Action required:** Clients must migrate to v2 before 2026-12-31.

The original sunset date was 2026-04-25. It has been extended to 2026-12-31 to allow clients additional migration time.

Clients can detect deprecation by reading the response headers:

```
Deprecation: true
Sunset: Sat, 31 Dec 2026 00:00:00 GMT
Link: </api/v1>; rel="deprecation"; type="text/html"
```

## v2 Roadmap

v2 is under active planning. The following breaking changes are planned:

### Breaking Changes

| Area | v1 Behaviour | v2 Behaviour |
|------|-------------|--------------|
| Error shape | Flat `{code, message}` | Flat `{code, message}` (unchanged) |
| Pagination | `limit`/`offset` query params | Cursor-only (`cursor` param); `offset` removed |
| API key header | `X-API-Key` | `Authorization: Bearer <token>` |
| Versioning header | `API-Version` | `API-Version` (unchanged) |
| Market list | Returns all fields | Returns summary fields; use `/markets/{id}` for full detail |

### Non-Breaking Additions Planned for v2

- Webhook subscription endpoints
- Batch market resolution
- User-level rate limit tiers

## Client Migration Guide

### 1. Detect deprecation headers

Add header inspection to your HTTP client or middleware:

```
if response.headers["Deprecation"] == "true":
    log.warn("v1 API is deprecated, migrate before " + response.headers["Sunset"])
```

### 2. Migrate pagination

Replace `offset`-based pagination with cursor pagination:

```
# v1 (deprecated)
GET /api/v1/markets?limit=20&offset=40

# v2
GET /api/v2/markets?limit=20&cursor=<opaque_cursor_from_previous_response>
```

### 3. Update the API key header

```
# v1 (deprecated)
X-API-Key: your_key

# v2
Authorization: Bearer your_key
```

### 4. Switch the version path prefix

Replace `/api/v1/` with `/api/v2/` in all request URLs once v2 is available.

## Version Negotiation

Clients may send an `API-Version` header to declare the target version. If omitted, the server defaults to the current stable version.

```
API-Version: v1
```

Unsupported versions are ignored and the server falls back to the default version.

## References

- `services/api/src/versioning.rs` — sunset header injection middleware
- `API_SPEC.md` — full endpoint reference
