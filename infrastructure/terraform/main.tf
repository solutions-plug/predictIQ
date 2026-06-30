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
}

module "redis" {
  source = "./modules/redis"

  environment        = var.environment
  vpc_id             = module.vpc.vpc_id
  subnet_ids         = module.vpc.private_subnet_ids
  node_type          = var.redis_node_type
  num_cache_nodes    = var.redis_num_nodes
  engine_version     = var.redis_engine_version
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
  acm_certificate_arn   = var.acm_certificate_arn
}

module "monitoring" {
  source = "./modules/monitoring"

  environment = var.environment
  ecs_cluster = module.ecs.cluster_name
  ecs_service = module.ecs.service_name
}
