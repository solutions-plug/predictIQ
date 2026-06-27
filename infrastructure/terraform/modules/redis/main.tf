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

variable "num_cache_nodes" {
  type = number
}

variable "engine_version" {
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

  ingress {
    from_port   = 6379
    to_port     = 6379
    protocol    = "tcp"
    cidr_blocks = ["10.0.0.0/8"]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-redis-sg"
    }
  )
}

resource "aws_elasticache_cluster" "main" {
  cluster_id           = "predictiq-${var.environment}"
  engine               = "redis"
  engine_version       = var.engine_version
  node_type            = var.node_type
  num_cache_nodes      = var.num_cache_nodes
  parameter_group_name = "default.redis7"
  port                 = 6379
  subnet_group_name    = aws_elasticache_subnet_group.main.name
  security_group_ids   = [aws_security_group.redis.id]
  
  automatic_failover_enabled = var.num_cache_nodes > 1 ? true : false
  at_rest_encryption_enabled = true
  transit_encryption_enabled = true
  
  maintenance_window = "mon:03:00-mon:04:00"
  notification_topic_arn = null

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-redis"
    }
  )
}

output "endpoint" {
  value = aws_elasticache_cluster.main.cache_nodes[0].address
}

output "redis_url" {
  value = "redis://${aws_elasticache_cluster.main.cache_nodes[0].address}:6379"
}
