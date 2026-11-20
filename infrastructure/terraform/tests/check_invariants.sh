#!/usr/bin/env bash
# Static regression checks for redis TLS, ALB dimensions, alarms and autoscaling.
set -euo pipefail
cd "$(dirname "$0")/.."
fail() { echo "FAIL: $1"; exit 1; }
grep -q 'rediss://' modules/redis/main.tf || fail "redis_url must use rediss://"
! grep -q '"redis://' modules/redis/main.tf || fail "plaintext redis:// URL present"
grep -q 'tokio-rustls-comp' ../../services/api/Cargo.toml || fail "redis crate TLS feature missing"
! grep -q 'alb/\*' modules/monitoring/main.tf || fail "wildcard LoadBalancer dimension"
grep -q 'var.alb_arn_suffix' modules/monitoring/main.tf || fail "alarms must use alb_arn_suffix"
for m in FreeStorageSpace DatabaseMemoryUsagePercentage Evictions ReplicationLag UnHealthyHostCount TargetResponseTime RunningTaskCount; do
  grep -q "$m" modules/monitoring/main.tf || fail "missing alarm for $m"
done
grep -q aws_appautoscaling_target modules/ecs/main.tf || fail "autoscaling target missing"
grep -q ALBRequestCountPerTarget modules/ecs/main.tf || fail "request-count scaling missing"

# DB_HOST must be the bare RDS hostname (no port suffix) and DB_PORT must be set.
db_host_line=$(grep -E 'name[[:space:]]*=[[:space:]]*"DB_HOST"' main.tf || true)
[ -n "$db_host_line" ] || fail "DB_HOST not set in task definition"
if echo "$db_host_line" | grep -qE ':[0-9]+'; then
  fail "DB_HOST contains a port suffix"
fi
if echo "$db_host_line" | grep -q 'address'; then
  :
elif echo "$db_host_line" | grep -q 'endpoint'; then
  fail "DB_HOST must use the RDS address attribute, not endpoint"
fi
grep -qE 'name[[:space:]]*=[[:space:]]*"DB_PORT"' main.tf || fail "DB_PORT not set in task definition"
grep -qE 'DB_PORT[^\n]*aws_db_instance[^\n]*\.port|aws_db_instance[^\n]*\.port[^\n]*DB_PORT' main.tf \
  || fail "DB_PORT must match the RDS instance port"

echo "OK"