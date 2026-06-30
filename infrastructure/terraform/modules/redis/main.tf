variable "environment" {
  type = string
}

variable "vpc_id" {
  type = string
}

variable "subnet_ids" {
  type = list(string)
}

variable "node_type" {
  type = string
}

variable "num_cache_clusters" {
  type    = number
  default = 2
}

variable "engine_version" {
  type = string
}

variable "redis_multi_az_enabled" {
  type        = bool
  default     = true
  description = "Enable Multi-AZ automatic failover for the Redis replication group."
}

variable "ecs_tasks_sg_id" {
  type        = string
  description = "Security group ID of the ECS tasks that are allowed to connect"
}

locals {
  common_tags = {
    Project     = "predictiq"
    Environment = var.environment
    Owner       = "infrastructure-team"
    ManagedBy   = "terraform"
  }
}

resource "aws_elasticache_subnet_group" "main" {
  name       = "predictiq-${var.environment}-redis-subnet"
  subnet_ids = var.subnet_ids

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-redis-subnet-group"
    }
  )
}

resource "aws_security_group" "redis" {
  name   = "predictiq-${var.environment}-redis-sg"
  vpc_id = var.vpc_id

  # Inbound Redis from ECS tasks only
  ingress {
    from_port       = 6379
    to_port         = 6379
    protocol        = "tcp"
    security_groups = [var.ecs_tasks_sg_id]
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-redis-sg"
    }
  )
}

resource "aws_elasticache_replication_group" "main" {
  replication_group_id = "predictiq-${var.environment}"
  description          = "PredictIQ Redis replication group (${var.environment})"

  engine               = "redis"
  engine_version       = var.engine_version
  node_type            = var.node_type
  num_cache_clusters   = var.num_cache_clusters >= 2 ? var.num_cache_clusters : 2
  parameter_group_name = "default.redis7"
  port                 = 6379

  subnet_group_name  = aws_elasticache_subnet_group.main.name
  security_group_ids = [aws_security_group.redis.id]

  automatic_failover_enabled = var.redis_multi_az_enabled
  multi_az_enabled           = var.redis_multi_az_enabled

  at_rest_encryption_enabled = true
  transit_encryption_enabled = true

  maintenance_window = "mon:03:00-mon:04:00"

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-redis"
    }
  )
}

output "sg_id" {
  value = aws_security_group.redis.id
}

output "endpoint" {
  value     = aws_elasticache_replication_group.main.primary_endpoint_address
  sensitive = true
}

output "redis_url" {
  value     = "redis://${aws_elasticache_replication_group.main.primary_endpoint_address}:6379"
  sensitive = true
}
