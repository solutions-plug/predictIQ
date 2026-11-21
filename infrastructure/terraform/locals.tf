# Common locals for consistent tagging across all modules
locals {
  common_tags = {
    Project   = "predictiq"
    Environment = var.environment
    Owner     = "infrastructure-team"
    ManagedBy = "terraform"
  }

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
