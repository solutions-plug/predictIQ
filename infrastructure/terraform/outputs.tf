output "alerts_topic_arn" {
  description = "ARN of the SNS topic used for CloudWatch alarm notifications"
  value       = module.monitoring.alerts_topic_arn
}

output "alerts_subscription_arn" {
  description = "ARN of the SNS subscription that delivers alarm notifications to the configured endpoint"
  value       = try(aws_sns_topic_subscription.alerts[0].arn, null)
}

output "alb_access_log_bucket_name" {
  description = "S3 bucket name receiving ALB access logs"
  value       = aws_s3_bucket.alb_access_logs.bucket
}

output "rds_endpoint" {
  description = "RDS endpoint"
  value       = module.rds.endpoint
  sensitive   = true
}

output "rds_host" {
  description = "RDS hostname (without credentials) for assembling the database connection URL"
  value       = module.rds.host
}

output "rds_port" {
  description = "RDS port for assembling the database connection URL"
  value       = module.rds.port
}

output "rds_database_name" {
  description = "RDS database name for assembling the database connection URL"
  value       = module.rds.database_name
}

output "rds_username" {
  description = "RDS master username for assembling the database connection URL"
  value       = module.rds.username
}

output "redis_endpoint" {
  description = "Redis endpoint"
  value       = module.redis.endpoint
  sensitive   = true
}

output "redis_host" {
  description = "Redis hostname (without credentials) for assembling the Redis connection URL"
  value       = module.redis.host
}

output "redis_port" {
  description = "Redis port for assembling the Redis connection URL"
  value       = module.redis.port
}

output "redis_parameter_group_name" {
  description = "Name of the managed ElastiCache parameter group attached to the Redis cluster"
  value       = module.redis.parameter_group_name
}

output "ecs_cluster_name" {
  description = "ECS cluster name"
  value       = module.ecs.cluster_name
}

output "ecs_service_name" {
  description = "ECS service name"
  value       = module.ecs.service_name
}

output "alb_dns_name" {
  description = "ALB DNS name"
  value       = module.ecs.alb_dns_name
}

output "logs_kms_key_arn" {
  description = "ARN of the customer-managed KMS key used to encrypt CloudWatch log groups"
  value       = aws_kms_key.logs.arn
}

output "secrets_kms_key_arn" {
  description = "ARN of the customer-managed KMS key used to encrypt secrets"
  value       = aws_kms_key.secrets.arn
}
