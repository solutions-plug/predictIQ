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

variable "redis_url" {
  type      = string
  sensitive = true
}

# Individual database credential components.
# Each is stored as its own Secrets Manager secret so they can be rotated
# independently, and the assembled connection string (containing the password)
# is never stored anywhere in plaintext.
variable "db_host" {
  type        = string
  description = "PostgreSQL hostname, e.g. rds-prod.cluster-xxxx.eu-west-1.rds.amazonaws.com"
}

variable "db_port" {
  type        = number
  default     = 5432
  description = "PostgreSQL port (default: 5432)"
}

variable "db_name" {
  type        = string
  description = "PostgreSQL database name, e.g. predictiq"
}

variable "db_user" {
  type        = string
  description = "PostgreSQL application user, e.g. predictiq_api"
}

variable "db_password" {
  type        = string
  sensitive   = true
  description = "PostgreSQL password for db_user. Store in your CI/CD secret store."
}

# SendGrid API key stored in Secrets Manager.
# Set this to the raw API key value; Terraform will create the secret version.
variable "sendgrid_api_key" {
  type      = string
  sensitive = true
}

# ISO-8601 date (YYYY-MM-DD) recording when SENDGRID_API_KEY was last rotated.
# Stored as Secrets Manager metadata so ops tooling and the startup check can
# read it without touching the key itself.
variable "sendgrid_key_rotated_at" {
  type        = string
  description = "Date the SendGrid API key was last rotated (YYYY-MM-DD, e.g. 2026-06-30)"
}

variable "acm_certificate_arn" {
  type        = string
  description = "ARN of the ACM certificate for HTTPS termination on the ALB."
}

variable "hmac_key" {
  description = "HMAC secret key used to sign API payloads"
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
          name      = "DB_HOST"
          valueFrom = aws_secretsmanager_secret.db_host.arn
        },
        {
          name      = "DB_PORT"
          valueFrom = aws_secretsmanager_secret.db_port.arn
        },
        {
          name      = "DB_NAME"
          valueFrom = aws_secretsmanager_secret.db_name.arn
        },
        {
          name      = "DB_USER"
          valueFrom = aws_secretsmanager_secret.db_user.arn
        },
        {
          name      = "DB_PASSWORD"
          valueFrom = aws_secretsmanager_secret.db_password.arn
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
          name      = "SENDGRID_KEY_ROTATED_AT"
          valueFrom = aws_secretsmanager_secret.sendgrid_key_rotated_at.arn
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
    unhealthy_threshold = 3
    timeout             = 5
    interval            = 45
    path                = "/health/ready"
    matcher             = "200"
  }

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-api-tg"
  })
}

resource "aws_lb_listener" "http" {
  load_balancer_arn = aws_lb.main.arn
  port              = 80
  protocol          = "HTTP"

  # Permanently redirect all plain-HTTP traffic to HTTPS (issue #889).
  # The ALB is the TLS termination point; the API service receives only
  # decrypted traffic from the ALB via the target group on port 80.
  default_action {
    type = "redirect"

    redirect {
      port        = "443"
      protocol    = "HTTPS"
      status_code = "HTTP_301"
    }
  }
}

resource "aws_lb_listener" "https" {
  load_balancer_arn = aws_lb.main.arn
  port              = 443
  protocol          = "HTTPS"
  ssl_policy        = "ELBSecurityPolicy-TLS13-1-2-2021-06"
  certificate_arn   = var.acm_certificate_arn

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

  depends_on = [aws_lb_listener.https]

  tags = merge(local.common_tags, {
    Name = "predictiq-${var.environment}-api-service"
  })
}

# ── AWS Secrets Manager — secrets inventory ────────────────────────────────────
#
# All application secrets are stored in Secrets Manager and injected into the
# ECS task via the `secrets` block above.  No plaintext secrets are passed
# through Terraform environment variables or ECS `environment` blocks.

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

# ── Database credentials (individual secrets) ────────────────────────────────
# Storing each component separately means the password can be rotated without
# touching the host/name/user, and the assembled connection string (with the
# password in the URL) never appears in Terraform state, ECS task logs, or
# CloudWatch Logs. The application assembles the connection string at runtime
# inside a secrecy::SecretString — see services/api/src/config.rs.

resource "aws_secretsmanager_secret" "db_host" {
  name = "predictiq/${var.environment}/db-host"
  tags = merge(local.common_tags, { Name = "predictiq-${var.environment}-db-host" })
}

resource "aws_secretsmanager_secret_version" "db_host" {
  secret_id     = aws_secretsmanager_secret.db_host.id
  secret_string = var.db_host
}

resource "aws_secretsmanager_secret" "db_port" {
  name = "predictiq/${var.environment}/db-port"
  tags = merge(local.common_tags, { Name = "predictiq-${var.environment}-db-port" })
}

resource "aws_secretsmanager_secret_version" "db_port" {
  secret_id     = aws_secretsmanager_secret.db_port.id
  secret_string = tostring(var.db_port)
}

resource "aws_secretsmanager_secret" "db_name" {
  name = "predictiq/${var.environment}/db-name"
  tags = merge(local.common_tags, { Name = "predictiq-${var.environment}-db-name" })
}

resource "aws_secretsmanager_secret_version" "db_name" {
  secret_id     = aws_secretsmanager_secret.db_name.id
  secret_string = var.db_name
}

resource "aws_secretsmanager_secret" "db_user" {
  name = "predictiq/${var.environment}/db-user"
  tags = merge(local.common_tags, { Name = "predictiq-${var.environment}-db-user" })
}

resource "aws_secretsmanager_secret_version" "db_user" {
  secret_id     = aws_secretsmanager_secret.db_user.id
  secret_string = var.db_user
}

resource "aws_secretsmanager_secret" "db_password" {
  name        = "predictiq/${var.environment}/db-password"
  description = "PostgreSQL password for the predictIQ application user. Never assembled into DATABASE_URL."
  tags = merge(local.common_tags, { Name = "predictiq-${var.environment}-db-password" })
}

resource "aws_secretsmanager_secret_version" "db_password" {
  secret_id     = aws_secretsmanager_secret.db_password.id
  secret_string = var.db_password
}

# ── SendGrid ────────────────────────────────────────────────────────────────

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
  description = "SendGrid API key used by the predictIQ API for transactional email. Rotate every 90 days — see docs/runbooks/sendgrid-api-key-rotation.md."

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-sendgrid-api-key"
    }
  )
}

resource "aws_secretsmanager_secret_version" "sendgrid_api_key" {
  secret_id     = aws_secretsmanager_secret.sendgrid_api_key.id
  secret_string = var.sendgrid_api_key
}

# Stores only the rotation date (YYYY-MM-DD string) — not the key itself.
# The application reads this at startup to warn if the key is older than 90 days.
resource "aws_secretsmanager_secret" "sendgrid_key_rotated_at" {
  name        = "predictiq/${var.environment}/sendgrid-key-rotated-at"
  description = "ISO-8601 date (YYYY-MM-DD) when SENDGRID_API_KEY was last rotated. Update this alongside the key itself."

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-sendgrid-key-rotated-at"
    }
  )
}

resource "aws_secretsmanager_secret_version" "sendgrid_key_rotated_at" {
  secret_id     = aws_secretsmanager_secret.sendgrid_key_rotated_at.id
  secret_string = var.sendgrid_key_rotated_at
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
          aws_secretsmanager_secret.db_host.arn,
          aws_secretsmanager_secret.db_port.arn,
          aws_secretsmanager_secret.db_name.arn,
          aws_secretsmanager_secret.db_user.arn,
          aws_secretsmanager_secret.db_password.arn,
          aws_secretsmanager_secret.redis_url.arn,
          aws_secretsmanager_secret.hmac_key.arn,
          aws_secretsmanager_secret.sendgrid_api_key.arn,
          aws_secretsmanager_secret.sendgrid_key_rotated_at.arn,
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
