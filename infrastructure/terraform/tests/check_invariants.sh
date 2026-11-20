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
grep -q '"APP_ENV"' main.tf || fail "ECS task definition must provide APP_ENV"
! grep -q '"ENVIRONMENT"' main.tf || fail "ECS task definition must not provide ENVIRONMENT (use APP_ENV)"
echo "OK"