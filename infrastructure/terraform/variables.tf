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

variable "api_min_count" {
  description = "Minimum API tasks for autoscaling"
  type        = number
  default     = 2

  validation {
    condition     = var.api_min_count >= 1 && var.api_min_count <= var.api_max_count
    error_message = "api_min_count must be >= 1 and <= api_max_count."
  }
}

variable "api_max_count" {
  description = "Maximum API tasks for autoscaling"
  type        = number
  default     = 10

  validation {
    condition     = var.api_max_count >= var.api_min_count && var.api_max_count <= 100
    error_message = "api_max_count must be >= api_min_count and <= 100."
  }
}

variable "api_cpu_target_utilization" {
  description = "Target average CPU utilization (percent) for ECS service autoscaling"
  type        = number
  default     = 70

  validation {
    condition     = var.api_cpu_target_utilization >= 1 && var.api_cpu_target_utilization <= 100
    error_message = "api_cpu_target_utilization must be between 1 and 100."
  }
}

variable "api_request_count_target" {
  description = "Target ALBRequestCountPerTarget for ECS service autoscaling"
  type        = number
  default     = 1000

  validation {
    condition     = var.api_request_count_target >= 1
    error_message = "api_request_count_target must be >= 1."
  }
}

variable "rds_free_storage_alarm_bytes" {
  description = "Alarm when RDS free storage drops below this many bytes"
  type        = number
  default     = 5368709120
}

variable "alb_latency_alarm_seconds" {
  description = "Alarm when ALB p95 TargetResponseTime exceeds this many seconds"
  type        = number
  default     = 1
}

variable "alerts_sns_endpoint" {
  description = "Endpoint (e.g. email address or HTTPS URL) subscribed to the alerts SNS topic. No default; must be supplied to receive alarm notifications."
  type        = string

  validation {
    condition     = length(trimspace(var.alerts_sns_endpoint)) > 0
    error_message = "alerts_sns_endpoint must be a non-empty endpoint (e.g. an email address or HTTPS URL)."
  }
}

variable "alerts_sns_protocol" {
  description = "Protocol for the alerts SNS topic subscription (e.g. email, https, sqs, lambda)"
  type        = string
  default     = "email"

  validation {
    condition     = contains(["email", "email-json", "https", "http", "sqs", "lambda", "sms"], var.alerts_sns_protocol)
    error_message = "alerts_sns_protocol must be one of: email, email-json, https, http, sqs, lambda, sms."
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
  description = "HMAC key used to sign and verify request payloads. Stored in AWS Secrets Manager."
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
  }
}

variable "jwt_secret" {
  description = "Secret used to sign and verify JWT access tokens. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.jwt_secret) >= 32
    error_message = "JWT secret must be at least 32 characters."
  }
}

variable "encryption_key" {
  description = "Symmetric key used to encrypt sensitive data at rest. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.encryption_key) >= 32
    error_message = "Encryption key must be at least 32 characters."
  }
}

variable "stripe_secret_key" {
  description = "Stripe secret API key for payment processing. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = can(regex("^sk_", var.stripe_secret_key))
    error_message = "Stripe secret key must start with 'sk_'."
  }
}

variable "stripe_webhook_secret" {
  description = "Stripe webhook signing secret. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = can(regex("^whsec_", var.stripe_webhook_secret))
    error_message = "Stripe webhook secret must start with 'whsec_'."
  }
}

variable "smtp_password" {
  description = "SMTP password for outbound email. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.smtp_password) >= 8
    error_message = "SMTP password must be at least 8 characters."
  }
}

variable "oauth_client_secret" {
  description = "OAuth client secret for third-party identity providers. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.oauth_client_secret) >= 16
    error_message = "OAuth client secret must be at least 16 characters."
  }
}

variable "webhook_signing_secret" {
  description = "Secret used to sign outbound webhook payloads. Stored in AWS Secrets Manager."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.webhook_signing_secret) >= 32
    error_message = "Webhook signing secret must be at least 32 characters."
  }
}
