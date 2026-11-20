#!/usr/bin/env bash
# Regression checks for issues #1775, #1782, #1783, #1784, #1872.
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
echo "OK"
