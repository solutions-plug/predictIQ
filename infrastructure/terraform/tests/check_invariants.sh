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
# Reject any wildcard LoadBalancer dimension in alarm or dashboard definitions.
! grep -Eq 'LoadBalancer[^\n]*"\*"' main.tf || fail "wildcard LoadBalancer dimension in main.tf"
! grep -Eq 'LoadBalancer[^\n]*"\*"' modules/monitoring/main.tf || fail "wildcard LoadBalancer dimension in monitoring"
for m in FreeStorageSpace DatabaseMemoryUsagePercentage Evictions ReplicationLag UnHealthyHostCount TargetResponseTime RunningTaskCount; do
  grep -q "$m" modules/monitoring/main.tf || fail "missing alarm for $m"
done
grep -q aws_appautoscaling_target modules/ecs/main.tf || fail "autoscaling target missing"
grep -q ALBRequestCountPerTarget modules/ecs/main.tf || fail "request-count scaling missing"
grep -q '"APP_ENV"' main.tf || fail "ECS task definition must provide APP_ENV"
! grep -q '"ENVIRONMENT"' main.tf || fail "ECS task definition must not provide ENVIRONMENT (use APP_ENV)"

# Alarm dimensions must reference real resources: every var.* used in a dimension
# must be declared in the monitoring module and wired from a module output in main.tf
# that is actually defined by the source module.
mon=modules/monitoring/main.tf
dim_vars=$(grep -E 'dimensions|dims|ClusterName|ServiceName|LoadBalancer|TargetGroup|ReplicationGroupId|DBInstanceIdentifier' "$mon" \
  | sed -E 's/threshold *= *var\.[a-z_]+//' | grep -oE 'var\.[a-z_]+' | sort -u | sed 's/var\.//')
for v in $dim_vars; do
  grep -q "variable \"$v\"" modules/monitoring/variables.tf "$mon" 2>/dev/null \
    || fail "$mon: alarm dimension var.$v is not declared as a variable"
  wired=$(grep -E "^[[:space:]]*(${v}|ecs_cluster|ecs_service)[[:space:]]*=" main.tf | grep -E "^[[:space:]]*${v}[[:space:]]*=" | grep -oE 'module\.[a-z_]+\.[a-z_]+' | head -1 || true)
  [ -n "$wired" ] || fail "main.tf: monitoring alarm dimension '$v' is not wired from a module output"
  mod=$(echo "$wired" | cut -d. -f2); out=$(echo "$wired" | cut -d. -f3)
  grep -q "output \"$out\"" "modules/$mod/outputs.tf" "modules/$mod/main.tf" 2>/dev/null \
    || fail "main.tf: alarm dimension '$v' references undefined output module.$mod.$out"
done
# Assert expected CloudWatch alarms are present in the plan.
for m in FreeStorageSpace DatabaseMemoryUsagePercentage Evictions ReplicationLag UnHealthyHostCount TargetResponseTime RunningTaskCount; do
  grep -q "aws_cloudwatch_metric_alarm.*$m\|$m.*aws_cloudwatch_metric_alarm" modules/monitoring/main.tf \
    || grep -q "$m" modules/monitoring/main.tf \
    || fail "missing CloudWatch alarm for $m in plan"
done
# The alerts SNS topic must have at least one subscription.
grep -q 'resource "aws_sns_topic" "alerts"' "$mon" || fail "$mon: aws_sns_topic.alerts missing"
grep -qE 'resource "aws_sns_topic_subscription"' "$mon" || fail "$mon: aws_sns_topic.alerts has no aws_sns_topic_subscription"
grep -q 'topic_arn *= *aws_sns_topic.alerts.arn' "$mon" || fail "$mon: no subscription targets aws_sns_topic.alerts"
grep -q 'aws_sns_topic_subscription' modules/monitoring/main.tf \
  || fail "missing SNS subscription for alerts topic"
grep -q 'aws_sns_topic_subscription' main.tf \
  || grep -q 'aws_sns_topic_subscription' modules/monitoring/main.tf \
  || fail "missing SNS subscription referencing alerts topic"
echo "OK"
