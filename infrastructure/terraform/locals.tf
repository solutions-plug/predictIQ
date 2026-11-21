# Common locals for consistent tagging across all modules
locals {
  common_tags = {
    Project   = "predictiq"
    Environment = var.environment
    Owner     = "infrastructure-team"
    ManagedBy = "terraform"
  }

  # VPC flow logs and endpoint configuration (issue #1872)
  flow_log_group_name = "/aws/vpc/${var.environment}-flow-logs"

  # Gateway endpoints route S3/DynamoDB traffic off the NAT gateway
  gateway_endpoints = {
    s3       = "com.amazonaws.${var.aws_region}.s3"
    dynamodb = "com.amazonaws.${var.aws_region}.dynamodb"
  }

  # Interface endpoints keep ECR, CloudWatch Logs and Secrets Manager traffic private
  interface_endpoints = {
    ecr_api        = "com.amazonaws.${var.aws_region}.ecr.api"
    ecr_dkr        = "com.amazonaws.${var.aws_region}.ecr.dkr"
    logs           = "com.amazonaws.${var.aws_region}.logs"
    secretsmanager = "com.amazonaws.${var.aws_region}.secretsmanager"
  }

  # Redis connection URL (issue #1875). The ElastiCache cluster enforces
  # in-transit encryption, so the scheme must be rediss:// (TLS) rather than
  # the plaintext redis:// form. Assembled here so every consumer shares it.
  redis_url = "rediss://${aws_elasticache_replication_group.redis.primary_endpoint_address}:${aws_elasticache_replication_group.redis.port}"

  # Alarm thresholds aligned with performance/config/slo.json SLO targets.
  # Each threshold maps to a runbook in docs/runbooks/.
  alarm_thresholds = {
    # RDS (docs/runbooks/slow-database-queries.md)
    rds_cpu_utilization    = 80
    rds_free_storage_bytes = 5368709120 # 5 GiB
    rds_connections        = 80

    # Redis (docs/runbooks/redis-failure.md)
    redis_memory_usage_bytes = 4294967296 # 4 GiB
    redis_evictions          = 1

    # ALB (docs/runbooks/high-api-response-time.md)
    alb_target_response_time = 1.0 # seconds
    alb_target_5xx_count     = 5

    # ECS (docs/runbooks/ecs-task-crash-loop.md)
    ecs_running_task_delta = 1
  }
}
