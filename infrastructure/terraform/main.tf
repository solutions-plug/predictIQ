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

# ---------------------------------------------------------------------------
# KMS customer-managed keys
#
# Two separate CMKs are used so that a compromise of the logs key does not
# grant access to secret material (and vice versa). Key policies are scoped to
# the relevant services only.
# ---------------------------------------------------------------------------

data "aws_caller_identity" "current" {}

# CMK used to encrypt CloudWatch log groups.
resource "aws_kms_key" "logs" {
  description             = "predictiq-${var.environment} CloudWatch logs encryption key"
  deletion_window_in_days = 30
  enable_key_rotation     = true

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "EnableRootAccountAdmin"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "AllowCloudWatchLogs"
        Effect = "Allow"
        Principal = {
          Service = "logs.${var.aws_region}.amazonaws.com"
        }
        Action = [
          "kms:Encrypt*",
          "kms:Decrypt*",
          "kms:ReEncrypt*",
          "kms:GenerateDataKey*",
          "kms:Describe*",
        ]
        Resource = "*"
        Condition = {
          ArnLike = {
            "kms:EncryptionContext:aws:logs:arn" = "arn:aws:logs:${var.aws_region}:${data.aws_caller_identity.current.account_id}:log-group:*"
          }
        }
      },
    ]
  })

  tags = {
    Name        = "predictiq-${var.environment}-logs"
    Project     = "predictiq"
    Environment = var.environment
    ManagedBy   = "terraform"
  }
}

resource "aws_kms_alias" "logs" {
  name          = "alias/predictiq-${var.environment}-logs"
  target_key_id = aws_kms_key.logs.key_id
}

# CMK used to encrypt Secrets Manager secrets.
resource "aws_kms_key" "secrets" {
  description             = "predictiq-${var.environment} Secrets Manager encryption key"
  deletion_window_in_days = 30
  enable_key_rotation     = true

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "EnableRootAccountAdmin"
        Effect = "Allow"
        Principal = {
          AWS = "arn:aws:iam::${data.aws_caller_identity.current.account_id}:root"
        }
        Action   = "kms:*"
        Resource = "*"
      },
      {
        Sid    = "AllowSecretsManager"
        Effect = "Allow"
        Principal = {
          Service = "secretsmanager.amazonaws.com"
        }
        Action = [
          "kms:Encrypt",
          "kms:Decrypt",
          "kms:ReEncrypt*",
          "kms:GenerateDataKey*",
          "kms:CreateGrant",
          "kms:DescribeKey",
        ]
        Resource = "*"
      },
    ]
  })

  tags = {
    Name        = "predictiq-${var.environment}-secrets"
    Project     = "predictiq"
    Environment = var.environment
    ManagedBy   = "terraform"
  }
}

resource "aws_kms_alias" "secrets" {
  name          = "alias/predictiq-${var.environment}-secrets"
  target_key_id = aws_kms_key.secrets.key_id
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

# Managed ElastiCache parameter group so eviction behaviour is pinned per
# environment instead of inheriting the account default. The API uses the cache
# as a best-effort cache (see services/api/src/db.rs), so volatile-lru is the
# right policy: only keys with a TTL are evicted, never persistent data.
resource "aws_elasticache_parameter_group" "redis" {
  name        = "predictiq-${var.environment}-redis7"
  family      = "redis7"
  description = "Managed parameter group for predictiq ${var.environment} Redis"

  parameter {
    name  = "maxmemory-policy"
    value = "volatile-lru"
  }

  parameter {
    name  = "timeout"
    value = "300"
  }

  parameter {
    name  = "tcp-keepalive"
    value = "300"
  }

  tags = {
    Name        = "predictiq-${var.environment}-redis7"
    Project     = "predictiq"
    Environment = var.environment
    ManagedBy   = "terraform"
  }
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

  parameter_group_name     = aws_elasticache_parameter_group.redis.name
  snapshot_retention_limit = var.redis_snapshot_retention_limit
  snapshot_window          = var.redis_snapshot_window
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
  db_port               = module.rds.port
  db_name               = var.db_name
  db_username           = var.db_username
  db_password           = var.db_password
  redis_host            = module.redis.primary_endpoint_address
  redis_port            = module.redis.port
  redis_auth_token      = var.redis_auth_token
  secrets_kms_key_arn   = aws_kms_key.secrets.arn
  logs_kms_key_arn      = aws_kms_key.logs.arn
}
