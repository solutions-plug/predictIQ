# PredictIQ API - API Specification

**Version:** 1.0.0

REST API for the PredictIQ prediction markets platform.

## API Versioning

The API uses URL path versioning (`/api/v1/`). The current stable version is **v1**.

Clients may also send an `API-Version` header (e.g. `API-Version: v1`) to explicitly
declare the version they target. If omitted, the server defaults to the current version.

## Deprecation Policy

When a version is deprecated:
- Responses will include a `Deprecation` header set to `true`.
- A `Sunset` header will indicate the date after which the version will be removed.
- A `Link` header will point to migration documentation.

Clients should monitor these headers and migrate before the sunset date.

Deprecated versions are supported for a minimum of **12 months** after the deprecation
announcement before being removed.

## Table of Contents

- [Overview](#overview)
- [Authentication](#authentication)
- [Endpoints](#endpoints)
- [Error Handling](#error-handling)
- [Contract Error Codes](#contract-error-codes)
- [Rate Limiting](#rate-limiting)

## Overview

### Base URL

```
http://0.0.0.0:8080
```

### API Versioning

The API uses URL path versioning (`/api/v1/`). The current stable version is **v1**.

Clients may also send an `API-Version` header (e.g. `API-Version: v1`) to explicitly
declare the version they target. If omitted, the server defaults to the current version.

### Deprecation Policy

When a version is deprecated:
- Responses will include a `Deprecation` header set to `true`.
- A `Sunset` header will indicate the date after which the version will be removed.
- A `Link` header will point to migration documentation.

Clients should monitor these headers and migrate before the sunset date.

Deprecated versions are supported for a minimum of **12 months** after the deprecation
announcement before being removed.

## Authentication

The API uses Bearer token authentication. Include your API key in the `Authorization` header:

```
Authorization: Bearer YOUR_API_KEY
```

## Endpoints

## Error Handling

All errors are returned as JSON with the following structure:

```json
{
  "error": {
    "code": "ERROR_CODE",
    "message": "Human-readable error message",
    "details": {}
  }
}
```

### Common Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| INVALID_REQUEST | 400 | Request validation failed |
| UNAUTHORIZED | 401 | Authentication required or failed |
| FORBIDDEN | 403 | Insufficient permissions |
| NOT_FOUND | 404 | Resource not found |
| CONFLICT | 409 | Resource conflict (e.g., duplicate) |
| RATE_LIMITED | 429 | Rate limit exceeded |
| INTERNAL_ERROR | 500 | Internal server error |

## Contract Error Codes

When a blockchain endpoint proxies a Soroban contract call that fails, the API wraps the
contract error in the standard error envelope with `code` set to `CONTRACT_ERROR` and a
`details.contract_code` field containing the numeric error code.

```json
{
  "error": {
    "code": "CONTRACT_ERROR",
    "message": "The market has been closed and no longer accepts bets or updates.",
    "details": {
      "contract_code": 103,
      "variant": "MarketClosed"
    }
  }
}
```

For the full list of contract error codes and their descriptions see
[`docs/CONTRACT_ERRORS.md`](docs/CONTRACT_ERRORS.md).

Quick reference for the most common codes:

| Code | Variant | Description |
|------|---------|-------------|
| 101 | `NotAuthorized` | Caller lacks required authorization. |
| 102 | `MarketNotFound` | No market exists with the given ID. |
| 103 | `MarketClosed` | Market is closed; no bets or updates accepted. |
| 107 | `InsufficientBalance` | Caller's token balance is too low. |
| 115 | `MarketNotActive` | Market is not in an active state. |
| 121 | `ContractPaused` | Contract is paused; all writes are disabled. |
| 142 | `BetNotFound` | No bet found for the given ID or caller. |
| 147 | `MarketNotResolved` | Market has not been resolved yet. |

## Rate Limiting

The API implements rate limiting to ensure fair usage:

- **Rate Limit:** 1000 requests per minute per API key
- **Headers:**
  - `X-RateLimit-Limit`: Maximum requests per window
  - `X-RateLimit-Remaining`: Requests remaining in current window
  - `X-RateLimit-Reset`: Unix timestamp when limit resets

When rate limited (HTTP 429), the response includes a `Retry-After` header indicating
how many seconds to wait before retrying.

---

**Generated from:** `services/api/openapi.yaml`  
**Last Updated:** 2026-05-28T13:37:38.653Z  
**Note:** This file is auto-generated. Do not edit directly. Update `services/api/openapi.yaml` instead.

## Pagination

All list endpoints support pagination via query parameters.

| Parameter | Type   | Default | Maximum | Description                              |
|-----------|--------|---------|---------|------------------------------------------|
| `limit`   | uint32 | 20      | **100** | Number of rows to return per page        |
| `offset`  | uint32 | 0       | —       | Zero-based row offset (offset pagination)|
| `cursor`  | string | —       | —       | Opaque cursor (cursor pagination)        |

Requests with `limit > 100` receive **400 Bad Request**:

```json
{
  "error": "limit_exceeded",
  "message": "limit 500 exceeds the maximum allowed value of 100.",
  "max_limit": 100
}
```

