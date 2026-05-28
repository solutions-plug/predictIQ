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
