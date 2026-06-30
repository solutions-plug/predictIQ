variable "environment" {
  type = string
}

variable "vpc_id" {
  type = string
}

variable "private_subnet_ids" {
  type = list(string)
}

variable "public_subnet_ids" {
  type = list(string)
}

variable "api_image_uri" {
  type = string
}

variable "api_container_port" {
  type = number
}

variable "api_desired_count" {
  type = number
}

variable "api_cpu" {
  type = number
}

variable "api_memory" {
  type = number
}

variable "database_url" {
  type      = string
  sensitive = true
}

variable "redis_url" {
  type      = string
  sensitive = true
}

variable "hmac_key" {
  description = "HMAC secret key used to sign API payloads"
  type        = string
  sensitive   = true
}

variable "sendgrid_api_key" {
  description = "SendGrid API key for transactional email"
  type        = string
  sensitive   = true
}

variable "api_signing_key" {
  description = "Private key used to sign API responses"
  type        = string
  sensitive   = true
}

variable "ecs_tasks_sg_id" {
  type        = string
  description = "Security group ID of the ECS tasks (managed at root level)"
}

locals {
  common_tags = {
    Project     = "predictiq"
    Environment = var.environment
    Owner       = "infrastructure-team"
    ManagedBy   = "terraform"
  }
}

# ── ECS Cluster ────────────────────────────────────────────────────────────────

resource "aws_ecs_cluster" "main" {
  name = "predictiq-${var.environment}"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-cluster"
  })
}

resource "aws_cloudwatch_log_group" "ecs" {
  name              = "/ecs/predictiq-${var.environment}"
  retention_in_days = var.environment == "prod" ? 30 : 7

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-logs"
  })
}

# ── ECS Task Definition ────────────────────────────────────────────────────────

resource "aws_ecs_task_definition" "api" {
  family                   = "predictiq-${var.environment}-api"
  network_mode             = "awsvpc"
  requires_compatibilities = ["FARGATE"]
  cpu                      = var.api_cpu
  memory                   = var.api_memory
  execution_role_arn       = aws_iam_role.ecs_task_execution_role.arn
  task_role_arn            = aws_iam_role.ecs_task_role.arn

  container_definitions = jsonencode([
    {
      name      = "api"
      image     = var.api_image_uri
      essential = true
      portMappings = [
        {
          containerPort = var.api_container_port
          hostPort      = var.api_container_port
          protocol      = "tcp"
        }
      ]
      environment = [
        {
          name  = "ENVIRONMENT"
          value = var.environment
        }
      ]
      # All secrets are injected from AWS Secrets Manager — no plaintext
      # environment variables for sensitive values.
      secrets = [
        {
          name      = "DATABASE_URL"
          valueFrom = aws_secretsmanager_secret.database_url.arn
        },
        {
          name      = "REDIS_URL"
          valueFrom = aws_secretsmanager_secret.redis_url.arn
        },
        {
          name      = "HMAC_KEY"
          valueFrom = aws_secretsmanager_secret.hmac_key.arn
        },
        {
          name      = "SENDGRID_API_KEY"
          valueFrom = aws_secretsmanager_secret.sendgrid_api_key.arn
        },
        {
          name      = "API_SIGNING_KEY"
          valueFrom = aws_secretsmanager_secret.api_signing_key.arn
        }
      ]
      logConfiguration = {
        logDriver = "awslogs"
        options = {
          "awslogs-group"         = aws_cloudwatch_log_group.ecs.name
          "awslogs-region"        = data.aws_region.current.name
          "awslogs-stream-prefix" = "ecs"
        }
      }
    }
  ])

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-api-task"
  })
}

# ── Networking ─────────────────────────────────────────────────────────────────

resource "aws_security_group" "alb" {
  name   = "predictiq-${var.environment}-alb-sg"
  vpc_id = var.vpc_id

  # Allow inbound HTTP/HTTPS from the public internet
  ingress {
    from_port   = 80
    to_port     = 80
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  ingress {
    from_port   = 443
    to_port     = 443
    protocol    = "tcp"
    cidr_blocks = ["0.0.0.0/0"]
  }

  # Restrict egress to the container port on ECS tasks only
  egress {
    from_port       = var.api_container_port
    to_port         = var.api_container_port
    protocol        = "tcp"
    security_groups = [var.ecs_tasks_sg_id]
  }

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-alb-sg"
  })
}

# Allow inbound from the ALB on the container port — added as a rule on the
# externally-managed ecs_tasks SG to avoid a circular module dependency.
resource "aws_security_group_rule" "ecs_tasks_ingress_alb" {
  type                     = "ingress"
  from_port                = var.api_container_port
  to_port                  = var.api_container_port
  protocol                 = "tcp"
  security_group_id        = var.ecs_tasks_sg_id
  source_security_group_id = aws_security_group.alb.id
}

resource "aws_lb" "main" {
  name               = "predictiq-${var.environment}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = var.public_subnet_ids

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-alb"
  })
}

resource "aws_lb_target_group" "api" {
  name        = "predictiq-${var.environment}-api"
  port        = var.api_container_port
  protocol    = "HTTP"
  vpc_id      = var.vpc_id
  target_type = "ip"

  health_check {
    healthy_threshold   = 2
    unhealthy_threshold = 2
    timeout             = 3
    interval            = 30
    path                = "/health"
    matcher             = "200"
  }

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-api-tg"
  })
}

resource "aws_lb_listener" "api" {
  load_balancer_arn = aws_lb.main.arn
  port              = 80
  protocol          = "HTTP"

  default_action {
    type             = "forward"
    target_group_arn = aws_lb_target_group.api.arn
  }
}

# ── ECS Service ────────────────────────────────────────────────────────────────

resource "aws_ecs_service" "api" {
  name            = "predictiq-${var.environment}-api"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.api.arn
  desired_count   = var.api_desired_count
  launch_type     = "FARGATE"

  # Zero-downtime rolling deploy: always keep 100 % capacity during updates,
  # allow up to 200 % so new tasks start before old ones are drained.
  deployment_minimum_healthy_percent = 100
  deployment_maximum_percent         = 200

  deployment_circuit_breaker {
    enable   = true
    rollback = true
  }

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [var.ecs_tasks_sg_id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.api.arn
    container_name   = "api"
    container_port   = var.api_container_port
  }

  depends_on = [aws_lb_listener.api]

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-api-service"
  })
}

# ── AWS Secrets Manager — secrets inventory ────────────────────────────────────
#
# All application secrets are stored in Secrets Manager and injected into the
# ECS task via the `secrets` block above.  No plaintext secrets are passed
# through Terraform environment variables or ECS `environment` blocks.

resource "aws_secretsmanager_secret" "database_url" {
  name        = "predictiq/${var.environment}/database-url"
  description = "PostgreSQL connection string for the predictIQ API"

  tags = merge(local.common_tags, {
    Name            = "predictiq-${var.environment}-database-url"
    SecretType      = "connection-string"
    RotationEnabled = "false"
  })
}

resource "aws_secretsmanager_secret_version" "database_url" {
  secret_id     = aws_secretsmanager_secret.database_url.id
  secret_string = var.database_url
}

resource "aws_secretsmanager_secret" "redis_url" {
  name        = "predictiq/${var.environment}/redis-url"
  description = "Redis connection URL for the predictIQ API"

  tags = merge(local.common_tags, {
    Name            = "predictiq-${var.environment}-redis-url"
    SecretType      = "connection-string"
    RotationEnabled = "false"
  })
}

resource "aws_secretsmanager_secret_version" "redis_url" {
  secret_id     = aws_secretsmanager_secret.redis_url.id
  secret_string = var.redis_url
}

resource "aws_secretsmanager_secret" "hmac_key" {
  name        = "predictiq/${var.environment}/hmac-key"
  description = "HMAC secret key used to sign API payloads and webhook signatures"

  tags = merge(local.common_tags, {
    Name            = "predictiq-${var.environment}-hmac-key"
    SecretType      = "signing-key"
    RotationEnabled = "true"
    RotationSchedule = "90-days"
  })
}

resource "aws_secretsmanager_secret_version" "hmac_key" {
  secret_id     = aws_secretsmanager_secret.hmac_key.id
  secret_string = var.hmac_key
}

resource "aws_secretsmanager_secret" "sendgrid_api_key" {
  name        = "predictiq/${var.environment}/sendgrid-api-key"
  description = "SendGrid API key for sending transactional email"

  tags = merge(local.common_tags, {
    Name            = "predictiq-${var.environment}-sendgrid-api-key"
    SecretType      = "api-key"
    RotationEnabled = "true"
    RotationSchedule = "180-days"
  })
}

resource "aws_secretsmanager_secret_version" "sendgrid_api_key" {
  secret_id     = aws_secretsmanager_secret.sendgrid_api_key.id
  secret_string = var.sendgrid_api_key
}

resource "aws_secretsmanager_secret" "api_signing_key" {
  name        = "predictiq/${var.environment}/api-signing-key"
  description = "Private key used to sign API responses"

  tags = merge(local.common_tags, {
    Name            = "predictiq-${var.environment}-api-signing-key"
    SecretType      = "signing-key"
    RotationEnabled = "true"
    RotationSchedule = "90-days"
  })
}

resource "aws_secretsmanager_secret_version" "api_signing_key" {
  secret_id     = aws_secretsmanager_secret.api_signing_key.id
  secret_string = var.api_signing_key
}

# ── IAM — ECS Task Execution Role ─────────────────────────────────────────────

resource "aws_iam_role" "ecs_task_execution_role" {
  name = "predictiq-${var.environment}-ecs-task-execution-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ecs-tasks.amazonaws.com"
        }
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-ecs-task-execution-role"
  })
}

resource "aws_iam_role_policy_attachment" "ecs_task_execution_role_policy" {
  role       = aws_iam_role.ecs_task_execution_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

# Grant the execution role access to all five Secrets Manager secrets.
resource "aws_iam_role_policy" "ecs_task_execution_secrets" {
  name = "predictiq-${var.environment}-ecs-task-execution-secrets"
  role = aws_iam_role.ecs_task_execution_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Effect = "Allow"
        Action = [
          "secretsmanager:GetSecretValue"
        ]
        Resource = [
          aws_secretsmanager_secret.database_url.arn,
          aws_secretsmanager_secret.redis_url.arn,
          aws_secretsmanager_secret.hmac_key.arn,
          aws_secretsmanager_secret.sendgrid_api_key.arn,
          aws_secretsmanager_secret.api_signing_key.arn,
        ]
      }
    ]
  })
}

# ── IAM — ECS Task Role ────────────────────────────────────────────────────────

resource "aws_iam_role" "ecs_task_role" {
  name = "predictiq-${var.environment}-ecs-task-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "ecs-tasks.amazonaws.com"
        }
      }
    ]
  })

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-ecs-task-role"
  })
}

# ── Data sources / Outputs ─────────────────────────────────────────────────────

data "aws_region" "current" {}

output "cluster_name" {
  value = aws_ecs_cluster.main.name
}

output "service_name" {
  value = aws_ecs_service.api.name
}

output "alb_dns_name" {
  value = aws_lb.main.dns_name
}

# Expose secret ARNs so other modules or CI/CD pipelines can reference them.
output "secret_arns" {
  description = "Map of secret name → ARN for all Secrets Manager secrets managed by this module"
  value = {
    database_url     = aws_secretsmanager_secret.database_url.arn
    redis_url        = aws_secretsmanager_secret.redis_url.arn
    hmac_key         = aws_secretsmanager_secret.hmac_key.arn
    sendgrid_api_key = aws_secretsmanager_secret.sendgrid_api_key.arn
    api_signing_key  = aws_secretsmanager_secret.api_signing_key.arn
  }
  sensitive = true
}
