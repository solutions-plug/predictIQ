# Common locals for consistent tagging across all modules
locals {
  common_tags = {
    Project   = "predictiq"
    Environment = var.environment
    Owner     = "infrastructure-team"
    ManagedBy = "terraform"
  }

  # VPC flow logs and endpoint configuration (issue #1872)
  flow_log_group_name = "/aws/vpc/${var.environment}-flow-logs"

  # Gateway endpoints route S3/DynamoDB traffic off the NAT gateway
  gateway_endpoints = {
    s3       = "com.amazonaws.${var.aws_region}.s3"
    dynamodb = "com.amazonaws.${var.aws_region}.dynamodb"
  }

  # Interface endpoints keep ECR, CloudWatch Logs and Secrets Manager traffic private
  interface_endpoints = {
    ecr_api        = "com.amazonaws.${var.aws_region}.ecr.api"
    ecr_dkr        = "com.amazonaws.${var.aws_region}.ecr.dkr"
    logs           = "com.amazonaws.${var.aws_region}.logs"
    secretsmanager = "com.amazonaws.${var.aws_region}.secretsmanager"
  }
}
