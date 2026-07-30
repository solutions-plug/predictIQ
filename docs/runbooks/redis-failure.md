# Redis Failure Runbook

## Alert

**Name:** `RedisFailure`
**Severity:** critical
**Detection:** `redis_up == 0` for 1 minute, or API error rate attributable to
cache errors (`cache_errors_total` rate spike).
**Dashboard:** Grafana → *PredictIQ Services* → *Cache Health*

## Impact

- API response times degrade significantly (all cached queries hit the database).
- Rate-limiting and session data are unavailable.
- Idempotency key checks for email and bet placement are bypassed, risking
  duplicate processing.

## Immediate Mitigation (< 5 minutes)

1. Test connectivity:
   ```bash
   redis-cli -u $REDIS_URL ping
   # Expected: PONG
   ```
2. Check ElastiCache cluster status in AWS console:
   ```
   ElastiCache → Redis clusters → predictiq-cache → Events
   ```
3. If the primary node has failed and a replica is available, trigger a
   manual failover:
   ```bash
   aws elasticache test-failover \
     --replication-group-id predictiq-cache \
     --node-group-id 0001
   ```
4. If no replica is available, restart the cluster node from the AWS console
   (ElastiCache → Nodes → Reboot).

## Investigation Steps

1. **Check ElastiCache metrics** (AWS CloudWatch):
   - `CurrConnections` — unusual spike or drop to 0
   - `FreeableMemory` — near 0 indicates memory pressure causing evictions
   - `EngineCPUUtilization` — sustained > 90%
2. **Check the API for cache-related errors:**
   ```bash
   aws logs tail /ecs/predictiq-api --follow --since 5m | grep -i "redis\|cache\|ECONNREFUSED"
   ```
3. **Review recent memory growth** — if `FreeableMemory` trended down, a
   missing key expiry or a large value was cached without a TTL.

## Escalation

- **< 5 min:** On-call engineer attempts failover.
- **5–15 min:** Page the infrastructure team (PagerDuty: `predictiq-infra`).
- **> 15 min:** Declare incident; consider switching the API to cache-bypass
  mode (set `REDIS_BYPASS=true` env var and redeploy).

## Post-Incident Steps

1. Capture the root cause (memory pressure, network partition, node failure).
2. Verify replica count is ≥ 1 in production.
3. Add missing TTLs to any key that contributed to memory exhaustion.
4. Update this runbook with new findings.
