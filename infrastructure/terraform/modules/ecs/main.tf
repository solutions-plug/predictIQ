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

locals {
  common_tags = {
    Project   = "predictiq"
    Environment = var.environment
    Owner     = "infrastructure-team"
    ManagedBy = "terraform"
  }
}

resource "aws_ecs_cluster" "main" {
  name = "predictiq-${var.environment}"

  setting {
    name  = "containerInsights"
    value = "enabled"
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-cluster"
    }
  )
}

resource "aws_cloudwatch_log_group" "ecs" {
  name              = "/ecs/predictiq-${var.environment}"
  retention_in_days = var.environment == "prod" ? 30 : 7

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-logs"
    }
  )
}

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

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-api-task"
    }
  )
}

resource "aws_security_group" "alb" {
  name   = "predictiq-${var.environment}-alb-sg"
  vpc_id = var.vpc_id

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

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-alb-sg"
    }
  )
}

resource "aws_lb" "main" {
  name               = "predictiq-${var.environment}-alb"
  internal           = false
  load_balancer_type = "application"
  security_groups    = [aws_security_group.alb.id]
  subnets            = var.public_subnet_ids

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-alb"
    }
  )
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

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-api-tg"
    }
  )
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

resource "aws_security_group" "ecs_tasks" {
  name   = "predictiq-${var.environment}-ecs-tasks-sg"
  vpc_id = var.vpc_id

  ingress {
    from_port       = var.api_container_port
    to_port         = var.api_container_port
    protocol        = "tcp"
    security_groups = [aws_security_group.alb.id]
  }

  egress {
    from_port   = 0
    to_port     = 0
    protocol    = "-1"
    cidr_blocks = ["0.0.0.0/0"]
  }

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-ecs-tasks-sg"
    }
  )
}

resource "aws_ecs_service" "api" {
  name            = "predictiq-${var.environment}-api"
  cluster         = aws_ecs_cluster.main.id
  task_definition = aws_ecs_task_definition.api.arn
  desired_count   = var.api_desired_count
  launch_type     = "FARGATE"

  network_configuration {
    subnets          = var.private_subnet_ids
    security_groups  = [aws_security_group.ecs_tasks.id]
    assign_public_ip = false
  }

  load_balancer {
    target_group_arn = aws_lb_target_group.api.arn
    container_name   = "api"
    container_port   = var.api_container_port
  }

  depends_on = [aws_lb_listener.api]

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-api-service"
    }
  )
}

resource "aws_secretsmanager_secret" "redis_url" {
  name = "predictiq/${var.environment}/redis-url"

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-redis-url"
    }
  )
}

resource "aws_secretsmanager_secret_version" "redis_url" {
  secret_id       = aws_secretsmanager_secret.redis_url.id
  secret_string   = var.redis_url
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

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-ecs-task-execution-role"
    }
  )
}

resource "aws_iam_role_policy_attachment" "ecs_task_execution_role_policy" {
  role       = aws_iam_role.ecs_task_execution_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy"
}

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
          aws_secretsmanager_secret.redis_url.arn
        ]
      }
    ]
  })
}

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

  tags = merge(
    local.common_tags,
    {
      Name = "predictiq-${var.environment}-ecs-task-role"
    }
  )
}

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
