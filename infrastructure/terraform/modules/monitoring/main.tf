variable "environment" {
  type = string
}

variable "ecs_cluster" {
  type = string
}

variable "ecs_service" {
  type = string
}

locals {
  common_tags = {
    Project   = "predictiq"
    Environment = var.environment
    Owner     = "infrastructure-team"
    ManagedBy = "terraform"
  }
}

resource "aws_cloudwatch_dashboard" "main" {
  dashboard_name = "predictiq-${var.environment}"

  dashboard_body = jsonencode({
    widgets = [
      {
        type = "metric"
        properties = {
          metrics = [
            ["AWS/ECS", "CPUUtilization", "ClusterName", var.ecs_cluster, "ServiceName", var.ecs_service],
            [".", "MemoryUtilization", ".", ".", ".", "."]
          ]
          period = 300
          stat   = "Average"
          region = data.aws_region.current.name
          title  = "ECS Service Metrics"
        }
      },
      {
        type = "metric"
        properties = {
          metrics = [
            ["AWS/ApplicationELB", "TargetResponseTime", "LoadBalancer", "app/predictiq-${var.environment}-alb/*"],
            [".", "RequestCount", ".", "."],
            [".", "HTTPCode_Target_5XX_Count", ".", "."]
          ]
          period = 300
          stat   = "Sum"
          region = data.aws_region.current.name
          title  = "ALB Metrics"
        }
      }
    ]
  })
}

resource "aws_sns_topic" "alerts" {
  name = "predictiq-${var.environment}-alerts"

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-alerts"
    }
  )
}

resource "aws_cloudwatch_metric_alarm" "ecs_cpu" {
  alarm_name          = "predictiq-${var.environment}-ecs-cpu-high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "CPUUtilization"
  namespace           = "AWS/ECS"
  period              = 300
  statistic           = "Average"
  threshold           = 80
  alarm_description   = "Alert when ECS CPU exceeds 80%"
  alarm_actions       = [aws_sns_topic.alerts.arn]

  dimensions = {
    ClusterName = var.ecs_cluster
    ServiceName = var.ecs_service
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-ecs-cpu-high"
    }
  )
}

resource "aws_cloudwatch_metric_alarm" "ecs_memory" {
  alarm_name          = "predictiq-${var.environment}-ecs-memory-high"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 2
  metric_name         = "MemoryUtilization"
  namespace           = "AWS/ECS"
  period              = 300
  statistic           = "Average"
  threshold           = 80
  alarm_description   = "Alert when ECS memory exceeds 80%"
  alarm_actions       = [aws_sns_topic.alerts.arn]

  dimensions = {
    ClusterName = var.ecs_cluster
    ServiceName = var.ecs_service
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-ecs-memory-high"
    }
  )
}

resource "aws_cloudwatch_metric_alarm" "alb_5xx" {
  alarm_name          = "predictiq-${var.environment}-alb-5xx-errors"
  comparison_operator = "GreaterThanThreshold"
  evaluation_periods  = 1
  metric_name         = "HTTPCode_Target_5XX_Count"
  namespace           = "AWS/ApplicationELB"
  period              = 300
  statistic           = "Sum"
  threshold           = 10
  alarm_description   = "Alert when ALB 5XX errors exceed 10"
  alarm_actions       = [aws_sns_topic.alerts.arn]

  dimensions = {
    LoadBalancer = "app/predictiq-${var.environment}-alb/*"
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-alb-5xx-errors"
    }
  )
}

data "aws_region" "current" {}

output "sns_topic_arn" {
  value = aws_sns_topic.alerts.arn
}
