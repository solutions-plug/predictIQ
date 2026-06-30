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
      CreatedAt   = timestamp()
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
  num_cache_nodes    = var.redis_num_nodes
  engine_version     = var.redis_engine_version
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
  api_cpu               = var.api_cpu
  api_memory            = var.api_memory
  database_url          = module.rds.database_url
  redis_url             = module.redis.redis_url
  hmac_key              = var.hmac_key
  sendgrid_api_key      = var.sendgrid_api_key
  api_signing_key       = var.api_signing_key
  ecs_tasks_sg_id       = aws_security_group.ecs_tasks.id
}

module "monitoring" {
  source = "./modules/monitoring"

  environment = var.environment
  ecs_cluster = module.ecs.cluster_name
  ecs_service = module.ecs.service_name
}
