# Security Policy

## Supported Versions

Only the latest release of PredictIQ receives security fixes.

| Version | Supported |
|---------|-----------|
| Latest  | ✅ Yes    |
| Older   | ❌ No     |

## Reporting a Vulnerability

**Please do not open a public GitHub issue for security vulnerabilities.**

Use one of the following channels:

1. **GitHub Private Vulnerability Reporting** (preferred) — click the
   [Report a vulnerability](../../security/advisories/new) button on the
   Security tab of this repository.
2. **Email** — send details to `security@predictiq.io` with the subject line
   `[SECURITY] <brief description>`.

### What to include

- A clear description of the vulnerability and its potential impact
- Steps to reproduce or a proof-of-concept (if available)
- Affected component(s) and version(s)
- Any suggested mitigations

## Response Timeline

| Milestone | Target |
|-----------|--------|
| Acknowledgement | Within **2 business days** |
| Initial assessment | Within **5 business days** |
| Fix or mitigation | Within **30 days** for critical/high; **90 days** for medium/low |
| Public disclosure | After a fix is available and affected users have had time to update |

We follow [coordinated disclosure](https://en.wikipedia.org/wiki/Coordinated_vulnerability_disclosure). We will notify you before any public disclosure and credit you in the release notes unless you prefer to remain anonymous.

## Disclosure Policy

- Vulnerabilities are kept confidential until a fix is released.
- We will publish a security advisory on GitHub after the fix is deployed.
- We ask reporters to refrain from public disclosure until we have released a fix or the agreed embargo period has passed.

## Scope

The following are **in scope**:

- `services/api` — Rust API backend
- `services/tts` — TTS microservice
- `frontend` — Next.js frontend
- `contracts/predict-iq` — Soroban smart contracts
- CI/CD pipelines and infrastructure-as-code in this repository

The following are **out of scope**:

- Third-party services (SendGrid, Stellar network, Pyth Network)
- Denial-of-service attacks without a demonstrated security impact
- Issues already reported or known

## Security Best Practices for Contributors

- Never commit secrets, API keys, or credentials — use environment variables.
- Follow the principle of least privilege when adding new permissions.
- Validate and sanitise all external input.
- Keep dependencies up to date; dependency-scan CI runs on every PR.

## CSRF Protection

Cross-Site Request Forgery (CSRF) exploits the browser's automatic inclusion of
cookies in cross-origin requests.  It is only relevant when an endpoint mutates
state **and** authentication is carried by a cookie that the browser attaches
automatically.

### Assessment

The PredictIQ API uses **stateless authentication** — API keys (`X-Api-Key`
header) and URL-embedded tokens — not cookies.  This eliminates the primary CSRF
attack vector for all current endpoints.

| Endpoint | Auth method | CSRF risk | Mitigation |
|---|---|---|---|
| `POST /api/v1/newsletter/subscribe` | None (public) | Low | JSON Content-Type blocks HTML-form CSRF; Origin validation middleware |
| `GET /api/v1/newsletter/unsubscribe` | URL token (`?token=`) | None | Tokens are per-user secrets; GET does not mutate via form attack |
| `GET /api/v1/newsletter/confirm` | URL token | None | Same as above |
| `GET /api/v1/newsletter/gdpr/export` | URL token | None | Same as above |
| `DELETE /api/v1/newsletter/gdpr/delete` | URL token | None | Same as above |
| `POST /api/v1/markets/*/resolve` | `X-Api-Key` header | None | Custom headers cannot be forged by cross-site forms |
| All other admin routes | `X-Api-Key` header | None | Custom headers cannot be forged by cross-site forms |

### Defense-in-depth layers

Even though no cookie auth exists, the following defenses are active:

1. **JSON Content-Type requirement** — `content_type_validation_middleware`
   enforces `Content-Type: application/json` on all state-changing endpoints.
   HTML `<form>` elements can only submit `application/x-www-form-urlencoded`
   or `multipart/form-data`, so a forged form POST is rejected before reaching
   any handler.

2. **Origin / Referer validation** — `csrf_protection_middleware` (in
   `src/csrf.rs`) is applied to the newsletter route group.  For any
   state-changing request (POST / PUT / PATCH / DELETE) that carries an
   `Origin` header, the middleware rejects the request with **403 Forbidden**
   if the origin is not in the `CORS_ALLOWED_ORIGINS` list.  When a `Cookie`
   header is present but `Origin` is absent, the `Referer` header is checked
   as a fallback.  Non-browser clients (no `Origin`, no `Cookie`) are passed
   through unchanged.

3. **API-key exclusion** — requests carrying an `X-Api-Key` header skip the
   Origin/Referer check entirely, as API-key auth is inherently CSRF-safe.

### Future considerations

If cookie-based session authentication is introduced, a
**Synchronizer Token** or **Double-Submit Cookie** pattern must be added for
all state-changing endpoints accessible via browser sessions.
