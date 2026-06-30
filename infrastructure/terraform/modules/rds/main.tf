variable "environment" {
  type = string
}

variable "vpc_id" {
  type = string
}

variable "private_subnet_ids" {
  type = list(string)
}

variable "db_name" {
  type = string
}

variable "db_username" {
  type      = string
  sensitive = true
}

variable "db_password" {
  type      = string
  sensitive = true
}

variable "db_instance_class" {
  type = string
}

variable "allocated_storage" {
  type = number
}

variable "backup_retention" {
  type = number
}

variable "ecs_tasks_sg_id" {
  type        = string
  description = "Security group ID of the ECS tasks that are allowed to connect"
}

locals {
  common_tags = {
    Project   = "predictiq"
    Environment = var.environment
    Owner     = "infrastructure-team"
    ManagedBy = "terraform"
  }
}

resource "aws_db_subnet_group" "main" {
  name       = "predictiq-${var.environment}-db-subnet"
  subnet_ids = var.private_subnet_ids

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-db-subnet-group"
    }
  )
}

resource "aws_security_group" "rds" {
  name   = "predictiq-${var.environment}-rds-sg"
  vpc_id = var.vpc_id

  # Inbound PostgreSQL from ECS tasks only
  ingress {
    from_port       = 5432
    to_port         = 5432
    protocol        = "tcp"
    security_groups = [var.ecs_tasks_sg_id]
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-rds-sg"
    }
  )
}

resource "aws_db_instance" "main" {
  identifier            = "predictiq-${var.environment}"
  engine                = "postgres"
  engine_version        = "15.3"
  instance_class        = var.db_instance_class
  allocated_storage     = var.allocated_storage
  storage_encrypted     = true
  db_name               = var.db_name
  username              = var.db_username
  password              = var.db_password
  db_subnet_group_name  = aws_db_subnet_group.main.name
  vpc_security_group_ids = [aws_security_group.rds.id]
  
  backup_retention_period = var.backup_retention
  backup_window           = "03:00-04:00"
  maintenance_window      = "mon:04:00-mon:05:00"
  
  multi_az               = var.environment == "prod" ? true : false
  publicly_accessible    = false
  skip_final_snapshot    = var.environment != "prod"
  final_snapshot_identifier = var.environment == "prod" ? "predictiq-${var.environment}-final-snapshot-${formatdate("YYYY-MM-DD-hhmm", timestamp())}" : null

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-db"
    }
  )
}

output "sg_id" {
  value = aws_security_group.rds.id
}

output "endpoint" {
  value     = aws_db_instance.main.endpoint
  sensitive = true
}

output "database_url" {
  value     = "postgresql://${var.db_username}:${var.db_password}@${aws_db_instance.main.endpoint}/${var.db_name}"
  sensitive = true
}
