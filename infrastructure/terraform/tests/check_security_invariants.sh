#!/usr/bin/env bash
# Regression checks for issues #1775, #1782, #1783, #1784, #1917.
set -euo pipefail
cd "$(dirname "$0")/../modules"
fail() { echo "FAIL: $1" >&2; exit 1; }

grep -q 'aws_sns_topic_subscription' monitoring/main.tf || fail "alerts SNS topic has no subscriptions (#1775)"
! grep -q 'engine_version *= *"15\.3"' rds/main.tf || fail "RDS pins outdated minor version (#1782)"
! grep -q 'timestamp()' rds/main.tf || fail "RDS uses timestamp() (#1782)"
grep -q 'performance_insights_enabled *= *true' rds/main.tf || fail "RDS Performance Insights off (#1782)"
grep -q 'enabled_cloudwatch_logs_exports' rds/main.tf || fail "RDS log exports unset (#1782)"
! grep -qE 'output "(database_url|redis_url)"' rds/main.tf redis/main.tf || fail "URL outputs embed passwords (#1783)"
grep -q 'kms_key_id' ecs/main.tf || fail "ECS secrets/logs lack KMS (#1784)"
grep -q 'enable_key_rotation *= *true' ecs/main.tf || fail "KMS key rotation off (#1784)"

# #1917: every aws_cloudwatch_log_group must set a non-empty kms_key_id,
# and the referenced aws_kms_key resource must exist in the configuration.
log_group_files=$(grep -rl 'resource *"aws_cloudwatch_log_group"' . 2>/dev/null || true)
if [ -n "$log_group_files" ]; then
  for tf in $log_group_files; do
    # Extract each log group block and verify it declares a non-empty kms_key_id.
    while IFS= read -r block; do
      name=$(printf '%s\n' "$block" | sed -n 's/.*resource *"aws_cloudwatch_log_group" *"\([^"]*\)".*/\1/p' | head -n1)
      [ -n "$name" ] || name="<unknown>"
      if ! printf '%s\n' "$block" | grep -qE 'kms_key_id *= *"[^"]+"'; then
        fail "log group '$name' in $tf lacks kms_key_id (#1917)"
      fi
    done < <(awk '
      /resource *"aws_cloudwatch_log_group"/ { inblock=1; depth=0 }
      inblock {
        print
        n=gsub(/\{/,"{"); m=gsub(/\}/,"}")
        depth += n - m
        if (depth <= 0 && n+m > 0) { inblock=0; print "---BLOCK---" }
      }
    ' "$tf" | awk 'BEGIN{RS="---BLOCK---\n"} {if ($0 ~ /aws_cloudwatch_log_group/) print $0 "\n---BLOCK---"}')
  done

  # Assert at least one aws_kms_key resource is defined in the configuration.
  grep -rq 'resource *"aws_kms_key"' . 2>/dev/null || fail "no aws_kms_key resource found for log group encryption (#1917)"
fi

echo "OK"