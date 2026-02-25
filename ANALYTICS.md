# PredictIQ Analytics Implementation and Event Tracking

> **Issue**: [#87 - Document Analytics Implementation and Event Tracking](https://github.com/solutions-plug/predictIQ/issues/87)

This document provides comprehensive documentation for the analytics implementation, tracked events, and data usage at PredictIQ.

---

## Table of Contents

1. [Analytics Overview](#analytics-overview)
2. [Event Tracking Documentation](#event-tracking-documentation)
3. [Custom Dimensions and Metrics](#custom-dimensions-and-metrics)
4. [Conversion Tracking Setup](#conversion-tracking-setup)
5. [Dashboard Creation Guide](#dashboard-creation-guide)
6. [Report Templates](#report-templates)
7. [Data Analysis Best Practices](#data-analysis-best-practices)
8. [GDPR/Privacy Compliance](#gdprprivacy-compliance)
9. [Debugging Analytics](#debugging-analytics)

---

## Analytics Overview

### Tools Used

| Tool | Purpose | Implementation |
|------|---------|----------------|
| **Custom Analytics** | Primary event tracking | PostgreSQL + Rust API |
| **SendGrid Events** | Email tracking | Webhook integration |
| **Prometheus** | Infrastructure metrics | `/metrics` endpoint |
| **Database Logs** | Query performance | pg_stat statements |

### Data Flow Diagram

```
┌─────────────────────────────────────────────────────────────────────────┐
│                         Analytics Data Flow                              │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                          │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐               │
│  │   Browser   │────►│  API Server │────►│  PostgreSQL │               │
│  │  (Client)   │     │   (Rust)    │     │  Database   │               │
│  └─────────────┘     └──────┬──────┘     └─────────────┘               │
│         │                   │                                            │
│         │            ┌──────▼──────┐                                     │
│         │            │   Redis      │                                     │
│         │            │   Cache      │                                     │
│         │            └─────────────┘                                     │
│         │                                                                   │
│         │            ┌─────────────┐                                     │
│         └──────────►│  SendGrid    │                                     │
│            Email    │  (Webhooks)  │                                     │
│                      └──────┬──────┘                                     │
│                             │                                            │
│                      ┌──────▼──────┐                                     │
│                      │   Email     │                                     │
│                      │  Analytics  │                                     │
│                      └─────────────┘                                     │
│                                                                          │
│  ┌─────────────┐     ┌─────────────┐                                     │
│  │  Grafana    │◄────│ Prometheus  │                                     │
│  │  Dashboard  │     │   Metrics   │                                     │
│  └─────────────┘     └─────────────┘                                     │
│                                                                          │
└─────────────────────────────────────────────────────────────────────────┘
```

### Architecture

#### Database Schema

```sql
-- analytics_events table
CREATE TABLE analytics_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_name VARCHAR(120) NOT NULL,
    event_category VARCHAR(80),
    user_id UUID,
    session_id VARCHAR(120),
    page_url TEXT,
    referrer TEXT,
    properties JSONB NOT NULL DEFAULT '{}'::JSONB,
    ip_address INET,
    user_agent TEXT,
    occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    content_id UUID REFERENCES content_management(id)
);
```

#### API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/v1/analytics/events` | POST | Track an event |
| `/api/v1/analytics/query` | POST | Query events |
| `/api/v1/email/analytics` | GET | Get email analytics |
| `/api/v1/email/queue/stats` | GET | Get queue stats |
| `/metrics` | GET | Prometheus metrics |

### Privacy Compliance

| Aspect | Implementation |
|--------|----------------|
| Data Minimization | Only collect necessary fields |
| Consent | Cookie consent banner |
| Anonymization | IP anonymization option |
| Retention | 13 months raw, then aggregated |
| Deletion | GDPR delete endpoint available |

---

## Event Tracking Documentation

### All Tracked Events

#### User Engagement Events

| Event Name | Category | Description | Fired When |
|------------|----------|-------------|------------|
| `page_view` | engagement | Page was viewed | User visits a page |
| `scroll_depth` | engagement | User scrolls | Scrolling past 25%, 50%, 75%, 100% |
| `time_on_page` | engagement | Time spent | Page is unloaded |
| `click` | engagement | Link/button clicked | User clicks interactive element |
| `form_start` | engagement | Form interaction begins | User focuses on form field |
| `form_submit` | engagement | Form submitted | Form submission attempt |

#### Conversion Events

| Event Name | Category | Description | Fired When |
|------------|----------|-------------|------------|
| `newsletter_signup` | conversion | Newsletter subscription | User subscribes |
| `waitlist_join` | conversion | Waitlist signup | User joins waitlist |
| `contact_form_submit` | conversion | Contact form submitted | Form submission success |
| `account_created` | conversion | New account | Account registration |
| `first_bet` | conversion | First bet placed | User places first bet |

#### Transaction Events

| Event Name | Category | Description | Fired When |
|------------|----------|-------------|------------|
| `market_created` | transaction | Market created | Creator publishes market |
| `bet_placed` | transaction | Bet placed | Bet transaction confirmed |
| `bet_cancelled` | transaction | Bet cancelled | Cancellation confirmed |
| `winnings_claimed` | transaction | Payout claimed | Claim transaction confirmed |

#### Email Events (SendGrid)

| Event Name | Category | Description | Fired When |
|------------|----------|-------------|------------|
| `email_sent` | email | Email dispatched | SendGrid accepts |
| `email_delivered` | email | Email delivered | Recipient server accepts |
| `email_opened` | email | Email opened | Recipient opens email |
| `email_clicked` | email | Link clicked | Recipient clicks link |
| `email_bounced` | email | Email bounced | Delivery failed |
| `email_complained` | email | Spam complaint | Recipient marks as spam |
| `email_unsubscribed` | email | Unsubscribed | User clicks unsubscribe |

#### Security Events

| Event Name | Category | Description | Fired When |
|------------|----------|-------------|------------|
| `login_success` | security | Login successful | Authentication succeeds |
| `login_failed` | security | Login failed | Authentication fails |
| `rate_limit_exceeded` | security | Rate limit hit | Request blocked |
| `invalid_input` | security | Bad input detected | Validation fails |

---

### Event Properties

#### Standard Properties (All Events)

| Property | Type | Description |
|----------|------|-------------|
| `session_id` | string | Unique session identifier |
| `user_id` | uuid | Logged-in user ID (if applicable) |
| `timestamp` | datetime | Event occurrence time |
| `url` | string | Page URL |
| `referrer` | string | Referring URL |
| `ip_address` | inet | Client IP (anonymized) |
| `user_agent` | string | Browser/client info |

#### Event-Specific Properties

**page_view**
```json
{
  "event": "page_view",
  "properties": {
    "page_title": "Home",
    "page_path": "/",
    "viewport_width": 1920,
    "viewport_height": 1080
  }
}
```

**newsletter_signup**
```json
{
  "event": "newsletter_signup",
  "properties": {
    "source": "homepage",
    "campaign": "spring-promo",
    "location": "footer"
  }
}
```

**bet_placed**
```json
{
  "event": "bet_placed",
  "properties": {
    "market_id": "0x1234",
    "market_title": "Will ETH hit $5000?",
    "outcome_id": 1,
    "outcome_name": "Yes",
    "amount_xlm": 100,
    "odds": 2.5,
    "potential_payout_xlm": 250
  }
}
```

**email_opened**
```json
{
  "event": "email_opened",
  "properties": {
    "message_id": "msg-abc123",
    "template_name": "newsletter_confirmation",
    "device_type": "desktop",
    "client": "Gmail"
  }
}
```

---

### When Events Fire

#### Client-Side (Browser)

```javascript
// In predictiq-frontend or embedded script

// Page view - fires on route change
trackPageView('/markets/eth-5000', 'Will ETH hit $5000?');

// Click tracking
document.querySelectorAll('[data-track]').forEach(el => {
  el.addEventListener('click', (e) => {
    trackEvent('click', {
      element: e.target.dataset.track,
      text: e.target.textContent
    });
  });
});

// Newsletter signup
async function subscribeNewsletter(email, source) {
  await fetch('/api/v1/newsletter/subscribe', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ email })
  });
  
  // Client-side tracking
  trackEvent('newsletter_signup', { source });
}
```

#### Server-Side (Rust)

```rust
// From services/api/src/handlers.rs

pub async fn track_event(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<AnalyticsEvent>,
) -> Result<Json<()>, AppError> {
    let event = AnalyticsEvent {
        id: Uuid::new_v4(),
        event_name: payload.event_name,
        event_category: payload.event_category,
        user_id: payload.user_id,
        session_id: payload.session_id,
        page_url: payload.page_url,
        referrer: payload.referrer,
        properties: payload.properties,
        ip_address: None, // Extracted from request
        user_agent: None,  // Extracted from request
        occurred_at: Utc::now(),
        content_id: payload.content_id,
    };
    
    // Insert into database
    state.db.insert_analytics_event(event).await?;
    
    Ok(Json(()))
}
```

---

### Code Examples

#### Frontend JavaScript SDK

```javascript
// analytics.js - Client-side tracking library

class Analytics {
  constructor(config) {
    this.endpoint = config.endpoint || '/api/v1/analytics';
    this.sessionId = this.getOrCreateSessionId();
    this.userId = config.userId || null;
  }

  getOrCreateSessionId() {
    let sessionId = sessionStorage.getItem('predictiq_session');
    if (!sessionId) {
      sessionId = 'sess-' + Math.random().toString(36).substr(2, 9);
      sessionStorage.setItem('predictiq_session', sessionId);
    }
    return sessionId;
  }

  async track(eventName, properties = {}) {
    const payload = {
      event_name: eventName,
      event_category: this.categorizeEvent(eventName),
      session_id: this.sessionId,
      user_id: this.userId,
      page_url: window.location.href,
      referrer: document.referrer,
      properties: this.sanitizeProperties(properties)
    };

    try {
      await fetch(this.endpoint + '/events', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify(payload)
      });
    } catch (error) {
      console.warn('Analytics tracking failed:', error);
    }
  }

  categorizeEvent(eventName) {
    const categories = {
      'page_view': 'engagement',
      'newsletter_signup': 'conversion',
      'waitlist_join': 'conversion',
      'bet_placed': 'transaction',
      'email_opened': 'email',
      'login_failed': 'security'
    };
    return categories[eventName] || 'unknown';
  }

  sanitizeProperties(props) {
    // Remove any PII from properties
    const sanitized = { ...props };
    delete sanitized.email;
    delete sanitized.name;
    return sanitized;
  }
}

// Usage
const analytics = new Analytics({ userId: userIdFromAuth });

// Track page views
analytics.track('page_view', { 
  page_title: document.title 
});

// Track conversions
analytics.track('newsletter_signup', { 
  source: 'footer' 
});
```

#### Rust Backend

```rust
// services/api/src/analytics.rs

use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    pub id: Uuid,
    pub event_name: String,
    pub event_category: Option<String>,
    pub user_id: Option<Uuid>,
    pub session_id: String,
    pub page_url: Option<String>,
    pub referrer: Option<String>,
    pub properties: serde_json::Value,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub occurred_at: DateTime<Utc>,
    pub content_id: Option<Uuid>,
}

pub async fn track_event(
    pool: &sqlx::PgPool,
    event: AnalyticsEvent,
) -> Result<(), AppError> {
    sqlx::query(
        r#"
        INSERT INTO analytics_events 
            (id, event_name, event_category, user_id, session_id, 
             page_url, referrer, properties, ip_address, user_agent, 
             occurred_at, content_id)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
        "#
    )
    .bind(event.id)
    .bind(event.event_name)
    .bind(event.event_category)
    .bind(event.user_id)
    .bind(event.session_id)
    .bind(event.page_url)
    .bind(event.referrer)
    .bind(event.properties)
    .bind(event.ip_address)
    .bind(event.user_agent)
    .bind(event.occurred_at)
    .bind(event.content_id)
    .execute(pool)
    .await?;
    
    Ok(())
}
```

---

## Custom Dimensions and Metrics

### Custom Dimensions

| Dimension | Scope | Description |
|-----------|-------|-------------|
| `session_source` | Session | Traffic source (organic, social, direct) |
| `campaign_name` | Session | Marketing campaign |
| `device_type` | Session | Desktop, mobile, tablet |
| `user_tier` | User | free, pro, premium |
| `market_category` | Event | Sports, crypto, politics, etc. |
| `outcome_odds` | Event | Odds at time of bet |

### Custom Metrics

| Metric | Description | Calculation |
|--------|-------------|--------------|
| `bet_conversion_rate` | Bets per visitor | bets / unique visitors |
| `avg_time_to_first_bet` | Days to first bet | AVG(first_bet_date - signup_date) |
| `email_open_rate` | Opens per delivery | delivered / opened |
| `email_click_rate` | Clicks per open | opened / clicked |
| `market_resolution_rate` | Markets resolved | resolved / created |

### Implementation

```sql
-- Query using custom dimensions
SELECT 
    campaign_name,
    COUNT(*) as total_visits,
    COUNT(DISTINCT user_id) as unique_users,
    COUNT(*) FILTER (WHERE event_name = 'bet_placed') as total_bets,
    ROUND(
        COUNT(*) FILTER (WHERE event_name = 'bet_placed') * 100.0 / 
        COUNT(DISTINCT session_id), 2
    ) as conversion_rate
FROM analytics_events
WHERE occurred_at >= NOW() - INTERVAL '30 days'
GROUP BY campaign_name
ORDER BY total_visits DESC;
```

---

## Conversion Tracking Setup

### Conversion Funnel

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Conversion Funnel                            │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐          │
│  │   Visit     │────►│   Signup    │────►│  First Bet   │          │
│  │   100%      │     │    15%      │     │     5%       │          │
│  └─────────────┘     └─────────────┘     └─────────────┘          │
│        │                   │                   │                      │
│        ▼                   ▼                   ▼                      │
│    10,000              1,500               500                       │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

### Tracking Conversions

#### Define Conversion Goals

| Goal | Event | Target |
|------|-------|--------|
| Newsletter Signup | `newsletter_signup` | 100/week |
| Waitlist Join | `waitlist_join` | 500/week |
| Account Created | `account_created` | 50/week |
| First Bet | `first_bet` | 25/week |

#### Attribution Models

```sql
-- First-touch attribution
WITH first_touch AS (
    SELECT 
        user_id,
        MIN(occurred_at) as first_touch,
        referrer as source
    FROM analytics_events
    WHERE event_name IN ('page_view', 'newsletter_signup')
    GROUP BY user_id, referrer
)
SELECT 
    source,
    COUNT(*) as conversions
FROM first_touch
GROUP BY source;

-- Multi-touch attribution (weighted)
WITH touchpoints AS (
    SELECT 
        user_id,
        event_name,
        occurred_at,
        referrer,
        ROW_NUMBER() OVER (PARTITION BY user_id ORDER BY occurred_at) as touch_num,
        COUNT(*) OVER (PARTITION BY user_id) as total_touches
    FROM analytics_events
    WHERE event_name = 'bet_placed'
)
SELECT 
    referrer,
    COUNT(*) as weight
FROM touchpoints
WHERE event_name = 'bet_placed'
GROUP BY referrer;
```

---

## Dashboard Creation Guide

### Grafana Dashboard Setup

#### Data Source Configuration

```yaml
# Grafana provisioning/datasources/dashboard.yaml
apiVersion: 1

datasources:
  - name: PostgreSQL PredictIQ
    type: postgres
    url: postgres.predictiq.internal:5432
    database: predictiq
    user: readonly_user
    secureJsonData:
      password: ${POSTGRES_PASSWORD}
```

#### Recommended Dashboards

##### 1. Real-Time Traffic

```json
{
  "title": "PredictIQ - Real-Time Traffic",
  "panels": [
    {
      "title": "Active Users (Last 5 min)",
      "type": "stat",
      "targets": [{
        "sql": "SELECT COUNT(DISTINCT session_id) FROM analytics_events WHERE occurred_at > NOW() - INTERVAL '5 minutes'"
      }]
    },
    {
      "title": "Page Views (Last Hour)",
      "type": "graph",
      "targets": [{
        "sql": "SELECT date_trunc('minute', occurred_at), COUNT(*) FROM analytics_events WHERE event_name = 'page_view' AND occurred_at > NOW() - INTERVAL '1 hour' GROUP BY 1"
      }]
    },
    {
      "title": "Top Pages",
      "type": "table",
      "targets": [{
        "sql": "SELECT page_url, COUNT(*) as views FROM analytics_events WHERE occurred_at > NOW() - INTERVAL '24 hours' AND event_name = 'page_view' GROUP BY page_url ORDER BY views DESC LIMIT 10"
      }]
    }
  ]
}
```

##### 2. Conversion Funnel

```json
{
  "title": "Conversion Funnel",
  "panels": [
    {
      "title": "Signup to Bet Conversion",
      "type": "bargauge",
      "targets": [{
        "sql": "SELECT ROUND(100.0 * (SELECT COUNT(DISTINCT user_id) FROM analytics_events WHERE event_name = 'bet_placed') / NULLIF((SELECT COUNT(DISTINCT user_id) FROM analytics_events WHERE event_name = 'account_created'), 0), 2)"
      }]
    }
  ]
}
```

### Metrics to Track

| Category | Metrics |
|----------|---------|
| Traffic | Sessions, page views, unique visitors, bounce rate |
| Engagement | Avg session duration, pages per session, scroll depth |
| Conversions | Signups, bets placed, winnings claimed |
| Revenue | Total bets, total volume, fees collected |
| Email | Open rate, click rate, delivery rate, unsubscribes |

---

## Report Templates

### Weekly Performance Report

```markdown
# PredictIQ Weekly Performance Report
## Week of [DATE]

### Traffic Summary
- Total Sessions: [X]
- Unique Visitors: [X]
- Page Views: [X]
- Bounce Rate: [X]%

### Conversion Metrics
- Newsletter Signups: [X] (+X% WoW)
- Waitlist Joins: [X] (+X% WoW)
- Accounts Created: [X] (+X% WoW)
- First Bets: [X] (+X% WoW)

### Top Performing Content
1. [Page Title] - [X] views
2. [Page Title] - [X] views
3. [Page Title] - [X] views

### Email Performance
- Emails Sent: [X]
- Open Rate: [X]%
- Click Rate: [X]%
- Unsubscribes: [X]

### Issues & Notes
- [Issue 1]
- [Issue 2]
```

### Monthly Analytics Report

```markdown
# PredictIQ Monthly Analytics Report
## [MONTH] [YEAR]

### Executive Summary
[Brief overview of key metrics and trends]

### Traffic Analysis
| Metric | This Month | Last Month | Change |
|--------|-----------|------------|--------|
| Sessions | 50,000 | 45,000 | +11% |
| Users | 12,000 | 10,500 | +14% |
| Page Views | 200,000 | 180,000 | +11% |

### Funnel Analysis
| Stage | Users | Drop-off |
|-------|-------|----------|
| Visit | 12,000 | - |
| Signup | 2,000 | 83% |
| First Bet | 500 | 75% |
| Repeat Bet | 200 | 60% |

### Revenue Metrics
- Total Bet Volume: XLM [X]
- Platform Fees: XLM [X]
- Average Bet Size: XLM [X]

### Recommendations
1. [Recommendation 1]
2. [Recommendation 2]
```

---

## Data Analysis Best Practices

### Query Guidelines

```sql
-- ✅ Good: Use proper indexing
SELECT * FROM analytics_events 
WHERE occurred_at >= NOW() - INTERVAL '30 days'
AND event_name = 'page_view';

-- ❌ Bad: Full table scan
SELECT * FROM analytics_events 
WHERE event_name = 'page_view';

-- ✅ Good: Aggregate before joining
WITH daily_stats AS (
    SELECT 
        DATE_TRUNC('day', occurred_at) as day,
        COUNT(*) as events
    FROM analytics_events
    WHERE occurred_at >= NOW() - INTERVAL '30 days'
    GROUP BY 1
)
SELECT * FROM daily_stats;
```

### Common Queries

```sql
-- User journey analysis
SELECT 
    user_id,
    array_agg(event_name ORDER BY occurred_at) as journey
FROM analytics_events
WHERE user_id = 'specific-user-uuid'
GROUP BY user_id;

-- Cohort analysis
SELECT 
    DATE_TRUNC('day', created_at) as cohort_date,
    COUNT(*) as users,
    COUNT(*) FILTER (WHERE EXISTS (
        SELECT 1 FROM analytics_events e 
        WHERE e.user_id = u.id 
        AND e.event_name = 'first_bet'
    )) as converted
FROM users u
WHERE u.created_at >= NOW() - INTERVAL '30 days'
GROUP BY 1;

-- Attribution analysis
SELECT 
    first.referrer as source,
    COUNT(DISTINCT first.user_id) as users,
    COUNT(*) FILTER (WHERE last.event_name = 'bet_placed') as conversions
FROM analytics_events first
JOIN analytics_events last ON first.user_id = last.user_id
WHERE first.event_name = 'page_view'
AND first.occurred_at = (
    SELECT MIN(occurred_at) 
    FROM analytics_events 
    WHERE user_id = first.user_id
)
GROUP BY first.referrer;
```

### Analysis Tools

| Tool | Use Case |
|------|----------|
| SQL (psql) | Ad-hoc queries |
| Grafana | Dashboards |
| Metabase | Self-service analytics |
| Python/pandas | Advanced analysis |
| Jupyter Notebooks | Research/prototyping |

---

## GDPR/Privacy Compliance

### Cookie Consent

#### Consent Banner

```html
<div id="cookie-consent" style="display: none;">
  <p>We use cookies to analyze traffic and improve your experience.</p>
  <button id="accept-cookies">Accept</button>
  <button id="reject-cookies">Reject</button>
  <a href="/privacy">Privacy Policy</a>
</div>

<script>
document.addEventListener('DOMContentLoaded', () => {
  if (!localStorage.getItem('cookie_consent')) {
    document.getElementById('cookie-consent').style.display = 'block';
  }
  
  document.getElementById('accept-cookies').addEventListener('click', () => {
    localStorage.setItem('cookie_consent', 'accepted');
    enableAnalytics();
    document.getElementById('cookie-consent').style.display = 'none';
  });
  
  document.getElementById('reject-cookies').addEventListener('click', () => {
    localStorage.setItem('cookie_consent', 'rejected');
    disableAnalytics();
    document.getElementById('cookie-consent').style.display = 'none';
  });
});
</script>
```

#### Consent Levels

| Level | Cookies | Purpose |
|-------|---------|---------|
| `necessary` | session_id, csrf_token | Essential functionality |
| `analytics` | _ga, _gid | Traffic analysis |
| `marketing` | (none currently) | Future targeting |

### Data Retention

| Data Type | Retention Period | Auto-Delete |
|-----------|------------------|-------------|
| Raw analytics events | 13 months | ✅ Cron job |
| Aggregated analytics | 7 years | ✅ Archive |
| Session data | 24 hours | ✅ Automatic |
| User consent records | Until withdrawn | ✅ Manual |

### User Data Deletion

#### GDPR Delete Endpoint

```bash
# User can request data deletion
DELETE /api/v1/analytics/gdpr/delete?user_id=USER_UUID
```

```sql
-- Deletion procedure
DELETE FROM analytics_events WHERE user_id = 'USER_UUID';
DELETE FROM consent_records WHERE user_id = 'USER_UUID';
```

#### Anonymization

For analytics that need to be retained:

```sql
-- Anonymize instead of delete (for aggregate data)
UPDATE analytics_events 
SET 
    user_id = NULL,
    ip_address = NULL,
    user_agent = NULL,
    session_id = 'anonymized_' || substr(session_id, 1, 8)
WHERE user_id = 'USER_UUID';
```

---

## Debugging Analytics

### Testing Events Locally

#### Using curl

```bash
# Test event tracking locally
curl -X POST http://localhost:8080/api/v1/analytics/events \
  -H "Content-Type: application/json" \
  -d '{
    "event_name": "page_view",
    "event_category": "engagement",
    "session_id": "test-session-123",
    "page_url": "http://localhost:3000/",
    "properties": {}
  }'
```

#### Using the test script

```bash
# Run analytics tests
cd services/api
./test_analytics.sh

# Output:
# ✓ Page view tracked
# ✓ Conversion tracked
# ✓ Event query returns data
# ✓ Retention policy applied
```

### Validation Tools

#### Check Event Schema

```javascript
// validate-event.js
const Ajv = require('ajv');
const eventSchema = require('./schemas/event.json');

const ajv = new Ajv();
const validate = ajv.compile(eventSchema);

function validateEvent(event) {
  const valid = validate(event);
  if (!valid) {
    console.error('Invalid event:', validate.errors);
    return false;
  }
  return true;
}

// Usage
validateEvent({
  event_name: 'page_view',
  properties: {}
});
```

#### Database Validation Queries

```sql
-- Check for events with missing required fields
SELECT id, event_name, occurred_at
FROM analytics_events
WHERE event_name IS NULL 
   OR properties IS NULL 
   OR session_id IS NULL;

-- Check for duplicate events
SELECT event_name, session_id, occurred_at, COUNT(*)
FROM analytics_events
WHERE occurred_at > NOW() - INTERVAL '1 hour'
GROUP BY 1, 2, 3
HAVING COUNT(*) > 1;

-- Check event distribution by hour
SELECT 
    EXTRACT(HOUR FROM occurred_at) as hour,
    event_name,
    COUNT(*)
FROM analytics_events
WHERE occurred_at > NOW() - INTERVAL '24 hours'
GROUP BY 1, 2
ORDER BY 1, 3 DESC;
```

### Common Issues

| Issue | Symptom | Solution |
|-------|---------|----------|
| Events not firing | No data in dashboard | Check browser console for errors |
| Missing session_id | Can't track user journey | Implement session ID generation |
| High bounce rate | Users leaving immediately | Check page load performance |
| Duplicate events | Inflated counts | Add deduplication logic |
| Data latency | Old data showing | Check worker/scheduler status |

### Debug Mode

```javascript
// Enable debug logging
const analytics = new Analytics({ 
  debug: true,
  endpoint: '/api/v1/analytics'
});

// Console output:
// [ANALYTICS] Track: page_view { page_url: '/', session_id: 'sess-abc123' }
// [ANALYTICS] Sent: 200 OK
```

---

## Appendix

### Database Indexes

```sql
-- Performance indexes
CREATE INDEX idx_analytics_events_event_name ON analytics_events(event_name);
CREATE INDEX idx_analytics_events_occurred_at ON analytics_events(occurred_at DESC);
CREATE INDEX idx_analytics_events_session_id ON analytics_events(session_id);
CREATE INDEX idx_analytics_events_properties_gin ON analytics_events USING GIN(properties);
```

### Environment Variables

```bash
# Analytics configuration
ANALYTICS_ENABLED=true
ANALYTICS_SAMPLE_RATE=100  # Percentage of events to track
ANALYTICS_DEBUG=false

# Database
DATABASE_URL=postgres://user:pass@localhost/predictiq

# Redis (for real-time analytics)
REDIS_URL=redis://localhost:6379
```

---

*Last Updated: February 2024*
*Document Owner: Analytics Team*
*Next Review: August 2024*
