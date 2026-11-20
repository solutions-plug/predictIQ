terraform {
  required_version = ">= 1.0"
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }

  # Backend configuration is provided via -backend-config flag during init
  # See backend-config.hcl for details
  backend "s3" {}
}

provider "aws" {
  region = var.aws_region

  default_tags {
    tags = {
      Environment = var.environment
      Project     = "predictiq"
      ManagedBy   = "terraform"
    }
  }
}

# The ECS tasks SG is created at root level to break the circular dependency
# between the ecs module (which needs the ALB SG to wire the ingress rule) and
# rds/redis modules (which need this SG ID for their ingress rules).
# Egress rules are added separately via aws_security_group_rule so that they
# can reference the rds/redis module outputs without creating a cycle.
resource "aws_security_group" "ecs_tasks" {
  name   = "predictiq-${var.environment}-ecs-tasks-sg"
  vpc_id = module.vpc.vpc_id

  tags = {
    Name        = "predictiq-${var.environment}-ecs-tasks-sg"
    Project     = "predictiq"
    Environment = var.environment
    ManagedBy   = "terraform"
  }
}

# Outbound to RDS (PostgreSQL)
resource "aws_security_group_rule" "ecs_tasks_egress_rds" {
  type                     = "egress"
  from_port                = 5432
  to_port                  = 5432
  protocol                 = "tcp"
  security_group_id        = aws_security_group.ecs_tasks.id
  source_security_group_id = module.rds.sg_id
}

# Outbound to Redis
resource "aws_security_group_rule" "ecs_tasks_egress_redis" {
  type                     = "egress"
  from_port                = 6379
  to_port                  = 6379
  protocol                 = "tcp"
  security_group_id        = aws_security_group.ecs_tasks.id
  source_security_group_id = module.redis.sg_id
}

# Outbound HTTPS for AWS API calls (Secrets Manager, ECR, CloudWatch)
resource "aws_security_group_rule" "ecs_tasks_egress_https" {
  type              = "egress"
  from_port         = 443
  to_port           = 443
  protocol          = "tcp"
  security_group_id = aws_security_group.ecs_tasks.id
  cidr_blocks       = ["0.0.0.0/0"]
}

module "vpc" {
  source = "./modules/vpc"

  environment = var.environment
  cidr_block  = var.vpc_cidr_block
}

module "rds" {
  source = "./modules/rds"

  environment          = var.environment
  vpc_id               = module.vpc.vpc_id
  private_subnet_ids   = module.vpc.private_subnet_ids
  db_name              = var.db_name
  db_username          = var.db_username
  db_password          = var.db_password
  db_instance_class    = var.db_instance_class
  allocated_storage    = var.allocated_storage
  backup_retention     = var.backup_retention_days
  ecs_tasks_sg_id      = aws_security_group.ecs_tasks.id
}

module "redis" {
  source = "./modules/redis"

  environment        = var.environment
  vpc_id             = module.vpc.vpc_id
  subnet_ids         = module.vpc.private_subnet_ids
  node_type          = var.redis_node_type
  num_cache_clusters = var.redis_num_nodes
  engine_version     = var.redis_engine_version
  redis_auth_token   = var.redis_auth_token
  ecs_tasks_sg_id    = aws_security_group.ecs_tasks.id
}

module "ecs" {
  source = "./modules/ecs"

  environment           = var.environment
  vpc_id                = module.vpc.vpc_id
  private_subnet_ids    = module.vpc.private_subnet_ids
  public_subnet_ids     = module.vpc.public_subnet_ids
  api_image_uri         = var.api_image_uri
  api_container_port    = var.api_container_port
  api_desired_count     = var.api_desired_count
  api_min_count         = var.api_min_count
  api_max_count         = var.api_max_count
  api_cpu               = var.api_cpu
  api_memory            = var.api_memory
  db_host               = module.rds.address
  db_port               = 5432
  db_name               = var.db_name
  db_user               = var.db_username
  db_password           = var.db_password
  redis_host            = module.redis.endpoint
  redis_port            = module.redis.port
  redis_auth_token      = var.redis_auth_token
  acm_certificate_arn   = var.acm_certificate_arn
  hmac_key              = var.hmac_key
  sendgrid_api_key      = var.sendgrid_api_key
  api_signing_key       = var.api_signing_key
  app_env                    = var.app_env
  base_url                   = var.base_url
  cors_allowed_origins       = var.cors_allowed_origins
  from_email                 = var.from_email
  blockchain_rpc_url         = var.blockchain_rpc_url
  predictiq_contract_id      = var.predictiq_contract_id
  api_keys                   = var.api_keys
  unsubscribe_signing_secret = var.unsubscribe_signing_secret
  sendgrid_webhook_secret    = var.sendgrid_webhook_secret
  email_idempotency_secret   = var.email_idempotency_secret
  ecs_tasks_sg_id       = aws_security_group.ecs_tasks.id
  otlp_endpoint         = var.otlp_endpoint
}

# ECS service autoscaling: target tracking on CPU and ALB request count.
# Capacity bounds are configurable via api_min_count / api_max_count.
resource "aws_appautoscaling_target" "ecs_api" {
  service_namespace  = "ecs"
  scalable_dimension = "ecs:service:DesiredCount"
  resource_id        = "service/${module.ecs.cluster_name}/${module.ecs.service_name}"
  min_capacity       = var.api_min_count
  max_capacity       = var.api_max_count
}

resource "aws_appautoscaling_policy" "ecs_api_cpu" {
  name               = "predictiq-${var.environment}-api-cpu-scaling"
  policy_type        = "TargetTrackingScaling"
  service_namespace  = aws_appautoscaling_target.ecs_api.service_namespace
  resource_id        = aws_appautoscaling_target.ecs_api.resource_id
  scalable_dimension = aws_appautoscaling_target.ecs_api.scalable_dimension

  target_tracking_scaling_policy_configuration {
    target_value       = var.api_cpu_target_utilization
    scale_in_cooldown  = 300
    scale_out_cooldown = 60

    predefined_metric_specification {
      predefined_metric_type = "ECSServiceAverageCPUUtilization"
    }
  }
}

resource "aws_appautoscaling_policy" "ecs_api_request_count" {
  name               = "predictiq-${var.environment}-api-request-count-scaling"
  policy_type        = "TargetTrackingScaling"
  service_namespace  = aws_appautoscaling_target.ecs_api.service_namespace
  resource_id        = aws_appautoscaling_target.ecs_api.resource_id
  scalable_dimension = aws_appautoscaling_target.ecs_api.scalable_dimension

  target_tracking_scaling_policy_configuration {
    target_value       = var.api_request_count_target
    scale_in_cooldown  = 300
    scale_out_cooldown = 60

    predefined_metric_specification {
      predefined_metric_type = "ALBRequestCountPerTarget"
      resource_label         = "${module.ecs.alb_arn_suffix}/${module.ecs.target_group_arn_suffix}"
    }
  }
}

module "monitoring" {
  source = "./modules/monitoring"

  environment = var.environment
  ecs_cluster = module.ecs.cluster_name
  ecs_service = module.ecs.service_name

  alb_arn_suffix          = module.ecs.alb_arn_suffix
  target_group_arn_suffix = module.ecs.target_group_arn_suffix
  rds_instance_id         = module.rds.instance_id
  redis_replication_group = module.redis.replication_group_id
  api_desired_count       = var.api_desired_count
  rds_free_storage_bytes  = var.rds_free_storage_alarm_bytes
  alb_latency_seconds     = var.alb_latency_alarm_seconds

  alert_email_endpoints = var.alert_email_endpoints
  alert_https_endpoints = var.alert_https_endpoints
}

# Subscribe a configurable endpoint to the alerts SNS topic so that CloudWatch
# alarms actually page someone. The endpoint has no default; when it is unset
# no subscription is created (see the count guard below).
resource "aws_sns_topic_subscription" "alerts" {
  count = var.alerts_subscription_endpoint == "" ? 0 : 1

  topic_arn = module.monitoring.alerts_topic_arn
  protocol  = var.alerts_subscription_protocol
  endpoint  = var.alerts_subscription_endpoint

  depends_on = [module.monitoring]
}

# ---------------------------------------------------------------------------
# CloudWatch alarms for RDS, Redis, ALB and ECS task count (issue #1871).
# Each alarm publishes to the existing alerts SNS topic and uses concrete
# resource dimensions (DB identifier, cache cluster id, ALB/target group
# suffixes, cluster/service name) rather than wildcards. Thresholds follow the
# SLO targets in performance/config/slo.json and the matching runbooks under
# docs/runbooks/.
# ---------------------------------------------------------------------------

# RDS: CPU utilisation (slow-database-queries runbook)
resource "aws_cloudwatch_metric_alarm" "rds_cpu" {
  alarm_name          = "predictiq-${var.environment}-rds-cpu-high"
  alarm_description   = "RDS CPU utilisation above threshold; see docs/runbooks/slow-database-queries.md"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "CPUUtilization"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = var.rds_cpu_alarm_percent
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = module.rds.instance_id
  }

  alarm_actions = [module.monitoring.alerts_topic_arn]
  ok_actions    = [module.monitoring.alerts_topic_arn]
}

# RDS: free storage space (slow-database-queries runbook)
resource "aws_cloudwatch_metric_alarm" "rds_free_storage" {
  alarm_name          = "predictiq-${var.environment}-rds-free-storage-low"
  alarm_description   = "RDS free storage space below threshold; see docs/runbooks/slow-database-queries.md"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 1
  metric_name         = "FreeStorageSpace"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = var.rds_free_storage_alarm_bytes
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = module.rds.instance_id
  }

  alarm_actions = [module.monitoring.alerts_topic_arn]
  ok_actions    = [module.monitoring.alerts_topic_arn]
}

# RDS: database connections (slow-database-queries runbook)
resource "aws_cloudwatch_metric_alarm" "rds_connections" {
  alarm_name          = "predictiq-${var.environment}-rds-connections-high"
  alarm_description   = "RDS database connection count above threshold; see docs/runbooks/slow-database-queries.md"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "DatabaseConnections"
  namespace           = "AWS/RDS"
  period              = 300
  statistic           = "Average"
  threshold           = var.rds_connections_alarm_count
  treat_missing_data  = "notBreaching"

  dimensions = {
    DBInstanceIdentifier = module.rds.instance_id
  }

  alarm_actions = [module.monitoring.alerts_topic_arn]
  ok_actions    = [module.monitoring.alerts_topic_arn]
}

# Redis: memory usage (redis-failure runbook)
resource "aws_cloudwatch_metric_alarm" "redis_memory" {
  alarm_name          = "predictiq-${var.environment}-redis-memory-high"
  alarm_description   = "Redis memory usage above threshold; see docs/runbooks/redis-failure.md"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "DatabaseMemoryUsagePercentage"
  namespace           = "AWS/ElastiCache"
  period              = 300
  statistic           = "Average"
  threshold           = var.redis_memory_alarm_percent
  treat_missing_data  = "notBreaching"

  dimensions = {
    CacheClusterId = module.redis.replication_group_id
  }

  alarm_actions = [module.monitoring.alerts_topic_arn]
  ok_actions    = [module.monitoring.alerts_topic_arn]
}

# Redis: evictions (redis-failure runbook)
resource "aws_cloudwatch_metric_alarm" "redis_evictions" {
  alarm_name          = "predictiq-${var.environment}-redis-evictions"
  alarm_description   = "Redis key evictions detected; see docs/runbooks/redis-failure.md"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "Evictions"
  namespace           = "AWS/ElastiCache"
  period              = 300
  statistic           = "Sum"
  threshold           = var.redis_evictions_alarm_count
  treat_missing_data  = "notBreaching"

  dimensions = {
    CacheClusterId = module.redis.replication_group_id
  }

  alarm_actions = [module.monitoring.alerts_topic_arn]
  ok_actions    = [module.monitoring.alerts_topic_arn]
}

# ALB: target response time (high-api-response-time runbook)
resource "aws_cloudwatch_metric_alarm" "alb_target_response_time" {
  alarm_name          = "predictiq-${var.environment}-alb-target-response-time-high"
  alarm_description   = "ALB target response time above threshold; see docs/runbooks/high-api-response-time.md"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 3
  metric_name         = "TargetResponseTime"
  namespace           = "AWS/ApplicationELB"
  period              = 300
  statistic           = "Average"
  threshold           = var.alb_latency_alarm_seconds
  treat_missing_data  = "notBreaching"

  dimensions = {
    LoadBalancer = module.ecs.alb_arn_suffix
    TargetGroup  = module.ecs.target_group_arn_suffix
  }

  alarm_actions = [module.monitoring.alerts_topic_arn]
  ok_actions    = [module.monitoring.alerts_topic_arn]
}

# ALB: target 5xx count (high-api-response-time runbook)
resource "aws_cloudwatch_metric_alarm" "alb_target_5xx" {
  alarm_name          = "predictiq-${var.environment}-alb-target-5xx-high"
  alarm_description   = "ALB target 5xx responses above threshold; see docs/runbooks/high-api-response-time.md"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "HTTPCode_Target_5XX_Count"
  namespace           = "AWS/ApplicationELB"
  period              = 300
  statistic           = "Sum"
  threshold           = var.alb_5xx_alarm_count
  treat_missing_data  = "notBreaching"

  dimensions = {
    LoadBalancer = module.ecs.alb_arn_suffix
    TargetGroup  = module.ecs.target_group_arn_suffix
  }

  alarm_actions = [module.monitoring.alerts_topic_arn]
  ok_actions    = [module.monitoring.alerts_topic_arn]
}

# ECS: running task count compared against the desired count
# (ecs-task-crash-loop runbook)
resource "aws_cloudwatch_metric_alarm" "ecs_running_task_count" {
  alarm_name          = "predictiq-${var.environment}-ecs-running-task-count-low"
  alarm_description   = "ECS running task count below desired count; see docs/runbooks/ecs-task-crash-loop.md"
  comparison_operator = "LessThanThreshold"
  evaluation_periods  = 2
  metric_name         = "RunningTaskCount"
  namespace           = "ECS/ContainerInsights"
  period              = 60
  statistic           = "Average"
  threshold           = var.api_desired_count
  treat_missing_data  = "breaching"

  dimensions = {
    ClusterName = module.ecs.cluster_name
    ServiceName = module.ecs.service_name
  }

  alarm_actions = [module.monitoring.alerts_topic_arn]
  ok_actions    = [module.monitoring.alerts_topic_arn]
}
