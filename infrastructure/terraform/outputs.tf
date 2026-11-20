output "vpc_id" {
  description = "VPC ID"
  value       = module.vpc.vpc_id
}

output "rds_endpoint" {
  description = "RDS endpoint"
  value       = module.rds.endpoint
  sensitive   = true
}

output "redis_endpoint" {
  description = "Redis endpoint"
  value       = module.redis.endpoint
  sensitive   = true
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
