# Service Level Objectives (SLO) Guide

## Overview

This document defines Service Level Objectives (SLOs), error budgets, and reliability practices for PredictIQ.

## What are SLOs?

Service Level Objectives (SLOs) are target values or ranges for service levels measured by Service Level Indicators (SLIs). They represent the reliability goals we commit to achieving.

### Key Concepts

- **SLI (Service Level Indicator)**: A quantitative measure of service level (e.g., request success rate, latency)
- **SLO (Service Level Objective)**: Target value for an SLI (e.g., 99.9% availability)
- **Error Budget**: The allowed amount of unreliability (100% - SLO target)
- **Burn Rate**: How fast we're consuming the error budget

## Defined SLOs

### 1. API Availability
- **Target**: 99.9% (three nines)
- **Measurement Window**: 30 days
- **Error Budget**: 0.1% (43.2 minutes/month)
- **Calculation**: `successful_requests / total_requests * 100`

### 2. API Latency P95
- **Target**: ≤ 200ms
- **Measurement Window**: 30 days
- **Error Budget**: 5% of requests may exceed target
- **Calculation**: `percentage of requests with latency ≤ 200ms`

### 3. API Latency P99
- **Target**: ≤ 500ms
- **Measurement Window**: 30 days
- **Error Budget**: 1% of requests may exceed target
- **Calculation**: `percentage of requests with latency ≤ 500ms`

### 4. Blockchain Sync Latency
- **Target**: ≤ 10 seconds
- **Measurement Window**: 7 days
- **Error Budget**: 5% of sync operations may exceed target
- **Calculation**: `percentage of sync operations completing within 10s`

### 5. Database Query Latency
- **Target**: ≤ 50ms (P95)
- **Measurement Window**: 7 days
- **Error Budget**: 5% of queries may exceed target
- **Calculation**: `percentage of queries with latency ≤ 50ms`

### 6. Cache Availability
- **Target**: 99.95% (three nines five)
- **Measurement Window**: 30 days
- **Error Budget**: 0.05% (21.6 minutes/month)
- **Calculation**: `successful_cache_operations / total_cache_operations * 100`

### 7. Email Delivery Success
- **Target**: 99.0%
- **Measurement Window**: 30 days
- **Error Budget**: 1.0%
- **Calculation**: `delivered_emails / sent_emails * 100`

## Error Budget Policy

The error budget determines how much unreliability is acceptable. When the budget is exhausted, we prioritize reliability over new features.

### Budget Thresholds and Actions

| Budget Remaining | Status | Action |
|-----------------|--------|--------|
| 100% | ✅ Healthy | Normal operations - all features enabled |
| 50% | ⚠️ Warning | Review recent changes and incidents |
| 25% | 🚨 Alert | Freeze non-critical feature deployments |
| 10% | 🔴 Critical | Freeze all deployments, focus on reliability |
| 0% | 💀 Emergency | Rollback recent changes, incident response |

### Budget Reset

Error budgets reset **monthly** at the beginning of each calendar month.

## Burn Rate Alerting

We use multi-window multi-burn-rate alerting to detect when we're consuming error budget too quickly.

### Fast Burn Alert (Critical)
- **Short Window**: 1 hour
- **Long Window**: 6 hours
- **Burn Rate Threshold**: 14.4x
- **Meaning**: At this rate, we'll exhaust the 30-day budget in 2 days
- **Severity**: Critical
- **Action**: Immediate investigation and mitigation

### Slow Burn Alert (Warning)
- **Short Window**: 6 hours
- **Long Window**: 24 hours
- **Burn Rate Threshold**: 6.0x
- **Meaning**: At this rate, we'll exhaust the 30-day budget in 5 days
- **Severity**: Warning
- **Action**: Review and plan mitigation

## Calculating Error Budget

### Formula

```
Error Budget Consumed = (Actual Errors / Error Budget) * 100
Error Budget Remaining = 100 - Error Budget Consumed
Burn Rate = Error Budget Consumed / Days in Window
```

### Example

For API Availability (99.9% target, 30-day window):

- Target: 99.9%
- Error Budget: 0.1%
- Actual Performance: 99.85%
- Actual Errors: 0.15%

```
Error Budget Consumed = (0.15 / 0.1) * 100 = 150%
Error Budget Remaining = 100 - 150 = -50% (exhausted!)
Burn Rate = 150 / 30 = 5% per day
```

## Using the Error Budget Calculator

Run the calculator script:

```bash
cd performance
node scripts/calculate-error-budget.js
```

This generates a report showing:
- Current SLO compliance
- Error budget consumption
- Burn rates
- Status and recommended actions

## Monitoring and Alerting

### Prometheus

SLO metrics are calculated using Prometheus recording rules defined in `performance/config/prometheus-slo-rules.yml`.

Key metrics:
- `slo:api_availability:success_rate`
- `slo:api_availability:error_budget_remaining`
- `slo:api_availability:burn_rate_1h`
- `slo:api_availability:burn_rate_6h`
- `slo:api_availability:burn_rate_24h`

### Grafana

Import the SLO dashboard from `performance/config/grafana-slo-dashboard.json` to visualize:
- SLO compliance
- Error budget remaining
- Burn rates
- Historical trends
- Active alerts

### Alerts

Prometheus alerts are configured for:
- Fast burn (critical)
- Slow burn (warning)
- Error budget exhausted (critical)
- Error budget low (warning)
- Individual SLO violations

### Alert Routing and Subscribers

There are two alert paths. Both are defined in the repo; only the thresholds
below are SLO-related and this section does not change them.

**1. Prometheus rules → Alertmanager.** Recording rules in
`performance/config/prometheus-slo-rules.yml` and alert rules in
`performance/config/alerts.yaml` (each alert carries `severity` and
`runbook_url`; see `docs/runbooks/high-error-rate.md` for the runbook format)
fire in Prometheus. Delivery to people depends on the Alertmanager
`route`/`receivers` configuration (example in `performance/config/README.md`).
Prometheus does not publish to SNS directly.

**2. CloudWatch alarms → SNS.** The monitoring module
(`infrastructure/terraform/modules/monitoring/main.tf`) creates the SNS topic
`predictiq-<env>-alerts`; its CloudWatch alarms (ECS CPU, etc.) use it as
`alarm_actions`. Subscribers come only from two Terraform variables:

| Variable | Protocol | Intended subscriber |
|---|---|---|
| `alert_email_endpoints` | email | on-call engineers / team alias |
| `alert_https_endpoints` | https | PagerDuty and/or Slack webhook |

**Escalation expectations.**
- `severity: critical` (fast burn, budget exhausted): page on-call via the
  HTTPS (PagerDuty) endpoint; acknowledge promptly and follow the linked runbook.
- `severity: warning` (slow burn, budget low): email/Slack, handled in business hours.

**Current state: no subscribers.** Both variables default to `[]`, so unless an
environment sets them the topic has no subscriptions and no alarm notifies
anyone. This was reported in #1775 (unmonitored metrics: #1776). Adding
subscriptions is an infrastructure change tracked there, not in this guide.

## Best Practices

### 1. Set Realistic Targets

- Base SLOs on historical performance
- Consider user expectations
- Account for dependencies
- Leave room for maintenance

### 2. Monitor Continuously

- Track SLO compliance daily
- Review error budget weekly
- Analyze trends monthly
- Adjust targets quarterly

### 3. Use Error Budget Wisely

- Spend budget on innovation
- Don't waste budget on preventable issues
- Balance reliability and velocity
- Communicate budget status to team

### 4. Respond to Violations

- Investigate root causes
- Implement fixes
- Update runbooks
- Conduct blameless postmortems

### 5. Iterate and Improve

- Review SLOs quarterly
- Adjust based on business needs
- Update error budget policy
- Refine alerting thresholds

## Incident Response

### When SLO is Violated

1. **Acknowledge**: Confirm the violation in monitoring
2. **Assess**: Determine impact and severity
3. **Mitigate**: Take immediate action to restore service
4. **Communicate**: Update stakeholders
5. **Resolve**: Fix the root cause
6. **Review**: Conduct postmortem

### When Error Budget is Exhausted

1. **Freeze Deployments**: Stop all non-critical changes
2. **Focus on Reliability**: Prioritize bug fixes and stability
3. **Root Cause Analysis**: Identify what consumed the budget
4. **Implement Fixes**: Address systemic issues
5. **Monitor Closely**: Track recovery
6. **Resume Gradually**: Lift freeze when budget recovers

## Reporting

### Weekly SLO Report

Generated automatically and sent to:
- SRE team
- Engineering leadership
- Product management

Includes:
- SLO compliance percentage
- Error budget remaining
- Burn rate trends
- Incidents and violations
- MTTR (Mean Time To Recovery)

### Monthly SLO Review

Conducted at the beginning of each month:
- Review previous month's performance
- Analyze budget consumption
- Identify improvement opportunities
- Adjust targets if needed
- Reset error budgets

## Tools and Scripts

### Calculate Error Budget
```bash
node performance/scripts/calculate-error-budget.js
```

### Generate SLO Report
```bash
node performance/scripts/generate-report.js
```

### Query Prometheus Metrics
```bash
curl 'http://prometheus:9090/api/v1/query?query=slo:api_availability:success_rate'
```

## References

- [Google SRE Book - SLOs](https://sre.google/sre-book/service-level-objectives/)
- [Google SRE Workbook - SLO Engineering](https://sre.google/workbook/implementing-slos/)
- [Multi-Window Multi-Burn-Rate Alerts](https://sre.google/workbook/alerting-on-slos/)
- [Error Budget Policy](https://sre.google/workbook/error-budget-policy/)

## Support

For questions or issues:
- Slack: #sre-team
- Email: sre-team@predictiq.com
- On-call: PagerDuty rotation
