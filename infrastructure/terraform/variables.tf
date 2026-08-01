variable "aws_region" {
  description = "AWS region"
  type        = string
  default     = "us-east-1"
  
  validation {
    condition     = can(regex("^[a-z]{2}-[a-z]+-\\d{1}$", var.aws_region))
    error_message = "AWS region must be a valid region format (e.g., us-east-1, eu-west-1)."
  }
}

variable "environment" {
  description = "Environment name (dev, staging, prod)"
  type        = string
  validation {
    condition     = contains(["dev", "staging", "prod"], var.environment)
    error_message = "Environment must be dev, staging, or prod."
  }
}

variable "vpc_cidr_block" {
  description = "CIDR block for VPC"
  type        = string
  default     = "10.0.0.0/16"
  
  validation {
    condition     = can(cidrhost(var.vpc_cidr_block, 0))
    error_message = "VPC CIDR block must be a valid CIDR notation (e.g., 10.0.0.0/16)."
  }
}

variable "db_name" {
  description = "Database name"
  type        = string
  default     = "predictiq"
  
  validation {
    condition     = can(regex("^[a-z][a-z0-9_]*$", var.db_name)) && length(var.db_name) <= 63
    error_message = "Database name must start with a letter, contain only lowercase letters, numbers, and underscores, and be at most 63 characters."
  }
}

variable "db_username" {
  description = "Database master username"
  type        = string
  sensitive   = true
  
  validation {
    condition     = length(var.db_username) >= 1 && length(var.db_username) <= 16
    error_message = "Database username must be between 1 and 16 characters."
  }
}

variable "db_password" {
  description = "Database master password"
  type        = string
  sensitive   = true

  validation {
    condition = (
      length(var.db_password) >= 24 &&
      can(regex("[A-Z]", var.db_password)) &&
      can(regex("[a-z]", var.db_password)) &&
      can(regex("[0-9]", var.db_password)) &&
      can(regex("[^a-zA-Z0-9]", var.db_password))
    )
    error_message = "Database password must be at least 24 characters and contain uppercase letters, lowercase letters, numbers, and special characters."
  }
}

variable "db_instance_class" {
  description = "RDS instance class"
  type        = string
  default     = "db.t3.micro"
  
  validation {
    condition     = can(regex("^db\\.[a-z0-9]+\\.[a-z0-9]+$", var.db_instance_class))
    error_message = "RDS instance class must be a valid instance type (e.g., db.t3.micro, db.t3.small)."
  }
}

variable "allocated_storage" {
  description = "Allocated storage in GB"
  type        = number
  default     = 20
  
  validation {
    condition     = var.allocated_storage >= 20 && var.allocated_storage <= 65536
    error_message = "Allocated storage must be between 20 and 65536 GB."
  }
}

variable "backup_retention_days" {
  description = "Backup retention period in days"
  type        = number
  default     = 7
  
  validation {
    condition     = var.backup_retention_days >= 1 && var.backup_retention_days <= 35
    error_message = "Backup retention days must be between 1 and 35."
  }
}

variable "redis_node_type" {
  description = "ElastiCache node type"
  type        = string
  default     = "cache.t3.micro"
  
  validation {
    condition     = can(regex("^cache\\.[a-z0-9]+\\.[a-z0-9]+$", var.redis_node_type))
    error_message = "Redis node type must be a valid ElastiCache node type (e.g., cache.t3.micro, cache.t3.small)."
  }
}

variable "redis_num_nodes" {
  description = "Number of cache nodes"
  type        = number
  default     = 1
  
  validation {
    condition     = var.redis_num_nodes >= 1 && var.redis_num_nodes <= 500
    error_message = "Number of Redis nodes must be between 1 and 500."
  }
}

variable "redis_engine_version" {
  description = "Redis engine version"
  type        = string
  default     = "7.0"
  
  validation {
    condition     = can(regex("^\\d+\\.\\d+$", var.redis_engine_version))
    error_message = "Redis engine version must be in format X.Y (e.g., 7.0, 6.2)."
  }
}

variable "redis_auth_token" {
  description = "Auth token for Redis in-transit encryption"
  type        = string
  sensitive   = true

  validation {
    condition = (
      length(var.redis_auth_token) >= 24 &&
      can(regex("[A-Z]", var.redis_auth_token)) &&
      can(regex("[a-z]", var.redis_auth_token)) &&
      can(regex("[0-9]", var.redis_auth_token)) &&
      can(regex("[^a-zA-Z0-9]", var.redis_auth_token))
    )
    error_message = "Redis auth token must be at least 24 characters and contain uppercase letters, lowercase letters, numbers, and special characters."
  }
}

variable "api_image_uri" {
  description = "ECR image URI for API"
  type        = string
  
  validation {
    condition     = can(regex("^\\d+\\.dkr\\.ecr\\.[a-z0-9-]+\\.amazonaws\\.com/.+:.+$", var.api_image_uri))
    error_message = "API image URI must be a valid ECR image URI (e.g., 123456789.dkr.ecr.us-east-1.amazonaws.com/predictiq:latest)."
  }
}

variable "api_container_port" {
  description = "API container port"
  type        = number
  default     = 8080
  
  validation {
    condition     = var.api_container_port >= 1024 && var.api_container_port <= 65535
    error_message = "API container port must be between 1024 and 65535."
  }
}

variable "api_desired_count" {
  description = "Desired number of API tasks"
  type        = number
  default     = 2
  
  validation {
    condition     = var.api_desired_count >= 1 && var.api_desired_count <= 10
    error_message = "API desired count must be between 1 and 10."
  }
}

variable "api_cpu" {
  description = "API task CPU units"
  type        = number
  default     = 256
  
  validation {
    condition     = contains([256, 512, 1024, 2048, 4096], var.api_cpu)
    error_message = "API CPU must be one of: 256, 512, 1024, 2048, 4096."
  }
}

variable "api_memory" {
  description = "API task memory in MB"
  type        = number
  default     = 512
  
  validation {
    condition     = contains([512, 1024, 2048, 3072, 4096, 5120, 6144, 7168, 8192], var.api_memory)
    error_message = "API memory must be one of: 512, 1024, 2048, 3072, 4096, 5120, 6144, 7168, 8192."
  }
}

variable "acm_certificate_arn" {
  description = "ARN of the ACM certificate used by the ALB HTTPS listener."
  type        = string

  validation {
    condition     = can(regex("^arn:aws:acm:", var.acm_certificate_arn))
    error_message = "acm_certificate_arn must be a valid ACM certificate ARN."
  }
}

variable "hmac_key" {
  description = "HMAC secret key used to sign API payloads and webhook signatures. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.hmac_key) >= 32
    error_message = "HMAC key must be at least 32 characters for adequate security."
  }
}

variable "sendgrid_api_key" {
  description = "SendGrid API key for transactional email. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = can(regex("^SG\\.", var.sendgrid_api_key))
    error_message = "SendGrid API key must start with 'SG.'."
  }
}

variable "api_signing_key" {
  description = "Private key used to sign API responses. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.api_signing_key) >= 32
    error_message = "API signing key must be at least 32 characters."

