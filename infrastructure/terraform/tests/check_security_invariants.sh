#!/usr/bin/env bash
# Regression checks for issues #1775, #1782, #1783, #1784, #1872, #1877, #1917.
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

# VPC flow logs and endpoints (#1872)
ROOT_TF="$(dirname "$0")/../main.tf"
grep -q 'resource "aws_flow_log"' "$ROOT_TF" || fail "VPC flow log missing (#1872)"
grep -q 'aws_cloudwatch_log_group" "vpc_flow_logs"' "$ROOT_TF" || fail "flow-log log group missing (#1872)"
grep -q 'aws_iam_role" "flow_log"' "$ROOT_TF" || fail "flow-log IAM role missing (#1872)"
grep -q 'log_destination *= *aws_cloudwatch_log_group.vpc_flow_logs.arn' "$ROOT_TF" || fail "flow log not referencing log group ARN (#1872)"
grep -q 'service_name *= *"com.amazonaws.${var.aws_region}.s3"' "$ROOT_TF" || fail "S3 gateway endpoint missing (#1872)"
grep -q 'service_name *= *"com.amazonaws.${var.aws_region}.dynamodb"' "$ROOT_TF" || fail "DynamoDB gateway endpoint missing (#1872)"
grep -q 'route_table_ids *= *\[aws_route_table.private.id\]' "$ROOT_TF" || fail "gateway endpoints not on private route table (#1872)"
grep -q 'ecr.api' "$ROOT_TF" || fail "ECR api interface endpoint missing (#1872)"
grep -q 'ecr.dkr' "$ROOT_TF" || fail "ECR dkr interface endpoint missing (#1872)"
grep -q 'secretsmanager' "$ROOT_TF" || fail "Secrets Manager interface endpoint missing (#1872)"

# #1877: separate CMKs for logs and secrets, log groups encrypted, secrets rotated.
grep -q 'aws_kms_key' main.tf || fail "no KMS CMKs declared (#1877)"
grep -q 'aws_kms_alias' main.tf || fail "KMS CMKs lack aliases (#1877)"
grep -q 'enable_key_rotation *= *true' main.tf || fail "KMS key rotation off (#1877)"
! grep -qE 'aws_cloudwatch_log_group[^{]*\{' main.tf || true
if grep -q 'aws_cloudwatch_log_group' main.tf; then
  awk '/resource "aws_cloudwatch_log_group"/{f=1} f&&/kms_key_id/{found=1} f&&/^}/{if(!found)exit 1; f=0; found=0} END{exit 0}' main.tf \
    || fail "a CloudWatch log group lacks kms_key_id (#1877)"
fi
grep -q 'aws_secretsmanager_secret_rotation' main.tf || fail "secrets lack rotation (#1877)"
grep -qE 'output "(logs|secrets)_kms_key_arn"' ../outputs.tf || fail "KMS key ARNs not exposed (#1877)"

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
