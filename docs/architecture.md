# PredictIQ — System Architecture

## System Context (C4 Level 1)

Shows PredictIQ and the external actors and systems it interacts with.

```mermaid
C4Context
    title System Context — PredictIQ

    Person(user, "User", "Browses markets, places bets, manages subscriptions via a web browser.")
    Person(admin, "Operator", "Monitors system health, manages deployments, reviews audit logs.")

    System(predictiq, "PredictIQ", "Prediction-markets platform: creates and resolves markets, tracks bets, sends notifications.")

    System_Ext(stellar, "Stellar RPC", "Blockchain network used to record bets and resolve market outcomes on-chain.")
    System_Ext(sendgrid, "SendGrid", "Transactional email delivery: subscription confirmations, notifications, GDPR export links.")

    Rel(user, predictiq, "Uses", "HTTPS")
    Rel(admin, predictiq, "Operates", "HTTPS / AWS Console")
    Rel(predictiq, stellar, "Submits & polls transactions", "HTTPS / JSON-RPC")
    Rel(predictiq, sendgrid, "Sends emails via", "HTTPS / REST API")
```

---

## Container Diagram (C4 Level 2)

Shows the deployable containers that make up PredictIQ and how they communicate. All containers run inside a single AWS VPC; the ALB and frontend are in public subnets while the API, databases, and TTS service live in private subnets.

```mermaid
C4Container
    title Container Diagram — PredictIQ

    Person(user, "User")
    Person(admin, "Operator")

    System_Ext(stellar, "Stellar RPC")
    System_Ext(sendgrid, "SendGrid")

    Boundary(aws, "AWS (us-east-1)") {

        Boundary(pub_subnet, "Public Subnet") {
            Container(alb, "Application Load Balancer", "AWS ALB", "Terminates TLS, routes HTTPS traffic to API tasks.")
            Container(frontend, "Frontend", "Next.js / Node.js", "Server-side-rendered web UI. Fetches market data from the API.")
        }

        Boundary(priv_subnet, "Private Subnet") {
            Container(api, "API Service", "Rust / Axum on ECS Fargate", "REST API: market CRUD, bet placement, newsletter, email management, audit logging.")
            Container(tts, "TTS Service", "Node.js on ECS Fargate", "Text-to-speech micro-service for market narration audio.")
            ContainerDb(postgres, "PostgreSQL", "AWS RDS (PostgreSQL)", "Primary persistent store: markets, bets, users, newsletter subscribers, audit events.")
            ContainerDb(redis, "Redis", "AWS ElastiCache", "Cache (market data, rate-limit counters, idempotency keys), blockchain circuit-breaker state.")
        }
    }

    Rel(user, alb, "HTTPS requests", "443")
    Rel(admin, alb, "HTTPS requests", "443")

    Rel(alb, frontend, "HTTP", "3000")
    Rel(alb, api, "HTTP", "8080")

    Rel(frontend, api, "API calls", "HTTP / JSON")
    Rel(frontend, tts, "Audio requests", "HTTP")

    Rel(api, postgres, "Reads / writes", "PostgreSQL protocol / TLS")
    Rel(api, redis, "Caches / rate-limits", "RESP / TLS")
    Rel(api, stellar, "Submits & polls transactions", "HTTPS / JSON-RPC")
    Rel(api, sendgrid, "Sends transactional email", "HTTPS / REST API")

    Rel(tts, api, "Reads market metadata", "HTTP / JSON")
```

---

## Network Boundaries

| Boundary | Contents | Inbound access |
|---|---|---|
| Public subnets | ALB, (optional) bastion host | Internet (0.0.0.0/0) on port 443 |
| Private subnets | ECS tasks (API, TTS), RDS, ElastiCache | Only from within the VPC |
| AWS Secrets Manager | Credentials (DB password, SendGrid key, etc.) | ECS task IAM role only |

---

## Key Technology Choices

| Component | Technology | Reason |
|---|---|---|
| API runtime | Rust / Axum | Memory-safe, high throughput, low latency |
| TTS service | Node.js | Rapid integration with browser-compatible TTS libraries |
| Frontend | Next.js | SSR for SEO; seamless API integration |
| Primary DB | PostgreSQL (RDS) | ACID guarantees for financial/market data |
| Cache / rate-limit | Redis (ElastiCache) | Sub-millisecond reads, built-in TTL, stream support |
| Blockchain | Stellar (Soroban) | Low-fee, fast-finality smart contracts |
| Email | SendGrid | Reliable delivery, webhook support for tracking |
| Compute | AWS ECS Fargate | Serverless containers; no EC2 management |
| IaC | Terraform | Reproducible, version-controlled infrastructure |

---

## Architecture Review — PR Checklist

When a pull request changes any of the components documented above, update this file as part of the PR. Specifically review if the change affects:

- [ ] A new or removed service / container
- [ ] A new external dependency (third-party API, data store, etc.)
- [ ] A change in network boundary (e.g. moving a service to a public subnet)
- [ ] A change in communication protocol between services
- [ ] A significant change to data-at-rest or data-in-transit trust boundaries
