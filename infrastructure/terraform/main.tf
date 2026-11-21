# Root Terraform configuration for the application VPC and networking.

terraform {
  required_version = ">= 1.5.0"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

provider "aws" {
  region = var.aws_region
}

locals {
  name_prefix = "${var.project_name}-${var.environment}"

  # Redis connection URL. The ElastiCache cluster enforces in-transit
  # encryption, so the scheme must be rediss:// (TLS) rather than redis://.
  # See issue #1875.
  redis_url = "rediss://${var.redis_endpoint}:${var.redis_port}"
}

resource "aws_vpc" "main" {
  cidr_block           = var.vpc_cidr
  enable_dns_support   = true
  enable_dns_hostnames = true

  tags = {
    Name = "${local.name_prefix}-vpc"
  }
}

resource "aws_subnet" "public" {
  count             = length(var.public_subnet_cidrs)
  vpc_id            = aws_vpc.main.id
  cidr_block        = var.public_subnet_cidrs[count.index]
  availability_zone = var.availability_zones[count.index]

  tags = {
    Name = "${local.name_prefix}-public-${count.index}"
  }
}

resource "aws_subnet" "private" {
  count             = length(var.private_subnet_cidrs)
  vpc_id            = aws_vpc.main.id
  cidr_block        = var.private_subnet_cidrs[count.index]
  availability_zone = var.availability_zones[count.index]

  tags = {
    Name = "${local.name_prefix}-private-${count.index}"
  }
}

resource "aws_internet_gateway" "main" {
  vpc_id = aws_vpc.main.id

  tags = {
    Name = "${local.name_prefix}-igw"
  }
}

resource "aws_route_table" "public" {
  vpc_id = aws_vpc.main.id

  route {
    cidr_block = "0.0.0.0/0"
    gateway_id = aws_internet_gateway.main.id
  }

  tags = {
    Name = "${local.name_prefix}-public-rt"
  }
}

resource "aws_route_table" "private" {
  vpc_id = aws_vpc.main.id

  tags = {
    Name = "${local.name_prefix}-private-rt"
  }
}

resource "aws_route_table_association" "public" {
  count          = length(aws_subnet.public)
  subnet_id      = aws_subnet.public[count.index].id
  route_table_id = aws_route_table.public.id
}

resource "aws_route_table_association" "private" {
  count          = length(aws_subnet.private)
  subnet_id      = aws_subnet.private[count.index].id
  route_table_id = aws_route_table.private.id
}

resource "aws_eip" "nat" {
  domain = "vpc"

  tags = {
    Name = "${local.name_prefix}-nat-eip"
  }
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

  tags = {
    Name = "${local.name_prefix}-nat-eip"
  }
}

resource "aws_nat_gateway" "main" {
  allocation_id = aws_eip.nat.id
  subnet_id     = aws_subnet.public[0].id

  tags = {
    Name = "${local.name_prefix}-nat"
  }
}

resource "aws_route" "private_nat" {
  route_table_id         = aws_route_table.private.id
  destination_cidr_block = "0.0.0.0/0"
  nat_gateway_id         = aws_nat_gateway.main.id
}

resource "aws_security_group" "app" {
  name        = "${local.name_prefix}-app-sg"
  description = "Application security group"
  vpc_id      = aws_vpc.main.id

  ingress {
    description = "Application traffic"
    from_port   = var.app_port
    to_port     = var.app_port
    protocol    = "tcp"
    cidr_blocks = [var.vpc_cidr]
  }

  egress {
    description = "Allow all outbound"
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = {
    Name = "${local.name_prefix}-app-sg"
  }
}

# --- VPC flow logs (#1872) ---

resource "aws_cloudwatch_log_group" "vpc_flow_logs" {
  name              = "/aws/vpc/${local.name_prefix}-flow-logs"
  retention_in_days = var.flow_log_retention_days

  tags = {
    Name = "${local.name_prefix}-vpc-flow-logs"
  }
}

data "aws_iam_policy_document" "flow_log_assume_role" {
  statement {
    effect = "Allow"

    principals {
      type        = "Service"
      identifiers = ["vpc-flow-logs.amazonaws.com"]
    }

    actions = ["sts:AssumeRole"]
  }
}

data "aws_iam_policy_document" "flow_log_delivery" {
  statement {
    effect = "Allow"

    actions = [
      "logs:CreateLogStream",
      "logs:PutLogEvents",
      "logs:DescribeLogGroups",
      "logs:DescribeLogStreams",
    ]

    resources = ["${aws_cloudwatch_log_group.vpc_flow_logs.arn}:*"]
  }
}

resource "aws_iam_role" "flow_log" {
  name               = "${local.name_prefix}-vpc-flow-log-role"
  assume_role_policy = data.aws_iam_policy_document.flow_log_assume_role.json

  tags = {
    Name = "${local.name_prefix}-vpc-flow-log-role"
  }
}

resource "aws_iam_role_policy" "flow_log" {
  name   = "${local.name_prefix}-vpc-flow-log-policy"
  role   = aws_iam_role.flow_log.id
  policy = data.aws_iam_policy_document.flow_log_delivery.json
}

resource "aws_flow_log" "main" {
  vpc_id                   = aws_vpc.main.id
  traffic_type             = "ALL"
  iam_role_arn             = aws_iam_role.flow_log.arn
  log_destination_type     = "cloud-watch-logs"
  log_destination          = aws_cloudwatch_log_group.vpc_flow_logs.arn
  max_aggregation_interval = 60

  tags = {
    Name = "${local.name_prefix}-vpc-flow-log"
  }
}

# --- VPC endpoints (#1872) ---

resource "aws_vpc_endpoint" "s3" {
  vpc_id           = aws_vpc.main.id
  service_name     = "com.amazonaws.${var.aws_region}.s3"
  vpc_endpoint_type = "Gateway"
  route_table_ids  = [aws_route_table.private.id]

  tags = {
    Name = "${local.name_prefix}-s3-endpoint"
  }
}

resource "aws_vpc_endpoint" "dynamodb" {
  vpc_id           = aws_vpc.main.id
  service_name     = "com.amazonaws.${var.aws_region}.dynamodb"
  vpc_endpoint_type = "Gateway"
  route_table_ids  = [aws_route_table.private.id]

  tags = {
    Name = "${local.name_prefix}-dynamodb-endpoint"
  }
}

resource "aws_security_group" "vpc_endpoints" {
  name        = "${local.name_prefix}-vpc-endpoints-sg"
  description = "Allow HTTPS from the application security group to interface endpoints"
  vpc_id      = aws_vpc.main.id

  ingress {
    description     = "HTTPS from application security group"
    from_port       = 443
    to_port         = 443
    protocol        = "tcp"
    security_groups = [aws_security_group.app.id]
  }

  tags = {
    Name = "${local.name_prefix}-vpc-endpoints-sg"
  }
}

locals {
  interface_endpoints = {
    ecr_api        = "ecr.api"
    ecr_dkr        = "ecr.dkr"
    logs           = "logs"
    secretsmanager = "secretsmanager"
  }
}

resource "aws_vpc_endpoint" "interface" {
  for_each           = local.interface_endpoints
  vpc_id             = aws_vpc.main.id
  service_name       = "com.amazonaws.${var.aws_region}.${each.value}"
  vpc_endpoint_type  = "Interface"
  subnet_ids         = aws_subnet.private[*].id
  security_group_ids = [aws_security_group.vpc_endpoints.id]
  private_dns_enabled = true

  tags = {
    Name = "${local.name_prefix}-${each.key}-endpoint"
  }
}

# --- ALB hardening (#1873) ---

resource "aws_s3_bucket" "alb_logs" {
  bucket        = "${local.name_prefix}-alb-logs"
  force_destroy = false

  tags = {
    Name = "${local.name_prefix}-alb-logs"
  }
}

resource "aws_s3_bucket_public_access_block" "alb_logs" {
  bucket                  = aws_s3_bucket.alb_logs.id
  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true
}

data "aws_iam_policy_document" "alb_logs" {
  statement {
    effect = "Allow"

    principals {
      type        = "AWS"
      identifiers = ["arn:aws:iam::${var.elb_log_delivery_account_id}:root"]
    }

    actions   = ["s3:PutObject"]
    resources = ["${aws_s3_bucket.alb_logs.arn}/alb/AWSLogs/*"]
  }
}

resource "aws_s3_bucket_policy" "alb_logs" {
  bucket = aws_s3_bucket.alb_logs.id
  policy = data.aws_iam_policy_document.alb_logs.json
}

resource "aws_wafv2_web_acl" "alb" {
  name  = "${local.name_prefix}-alb-waf"
  scope = "REGIONAL"

  default_action {
    allow {}
  }

  rule {
    name     = "AWSManagedRulesCommonRuleSet"
    priority = 1

    override_action {
      none {}
    }

    statement {
      managed_rule_group_statement {
        name        = "AWSManagedRulesCommonRuleSet"
        vendor_name = "AWS"
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "AWSManagedRulesCommonRuleSet"
      sampled_requests_enabled   = true
    }
  }

  rule {
    name     = "AWSManagedRulesKnownBadInputsRuleSet"
    priority = 2

    override_action {
      none {}
    }

    statement {
      managed_rule_group_statement {
        name        = "AWSManagedRulesKnownBadInputsRuleSet"
        vendor_name = "AWS"
      }
    }

    visibility_config {
      cloudwatch_metrics_enabled = true
      metric_name                = "AWSManagedRulesKnownBadInputsRuleSet"
      sampled_requests_enabled   = true
    }
  }

  visibility_config {
    cloudwatch_metrics_enabled = true
    metric_name                = "${local.name_prefix}-alb-waf"
    sampled_requests_enabled   = true
  }

  tags = {
    Name = "${local.name_prefix}-alb-waf"
  }
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
