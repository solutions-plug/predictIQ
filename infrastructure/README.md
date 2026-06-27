# PredictIQ Infrastructure as Code

This directory contains all infrastructure definitions for PredictIQ using Terraform.

## Structure

```
infrastructure/
├── terraform/
│   ├── main.tf              # Main configuration
│   ├── variables.tf         # Variable definitions with validation
│   ├── outputs.tf           # Output definitions
│   ├── locals.tf            # Common tags and locals
│   ├── bootstrap.sh         # Bootstrap script for state backend
│   ├── backend-config.hcl   # Default backend configuration
│   ├── environments/        # Environment-specific configurations
│   │   ├── README.md        # Environment separation documentation
│   │   ├── dev.tfvars       # Development environment variables
│   │   ├── staging/
│   │   │   ├── terraform.tfvars
│   │   │   └── backend.hcl
│   │   └── production/
│   │       ├── terraform.tfvars
│   │       └── backend.hcl
│   └── modules/             # Reusable modules
│       ├── vpc/
│       ├── rds/
│       ├── redis/
│       ├── ecs/
│       └── monitoring/
├── ROLLBACK.md              # Rollback procedures
└── README.md                # This file
```

## Prerequisites

- Terraform >= 1.5.0
- AWS CLI configured
- Appropriate AWS IAM permissions
- Access to Terraform state bucket
- GitHub repository secrets configured (see Deployment Process section)

## Required GitHub Secrets

The deployment workflow requires the following secrets to be configured in repository settings:

| Secret | Description | Example |
|--------|-------------|---------|
| `AWS_ROLE_DEV` | IAM role ARN for dev environment | `arn:aws:iam::123456789:role/terraform-dev` |
| `AWS_ROLE_STAGING` | IAM role ARN for staging environment | `arn:aws:iam::123456789:role/terraform-staging` |
| `AWS_ROLE_PROD` | IAM role ARN for production environment | `arn:aws:iam::123456789:role/terraform-prod` |

**Note:** The deploy workflow validates these secrets before attempting deployment. If any are missing, the workflow will fail with a clear error message.

## Quick Start

### Bootstrap Terraform State Backend (First Time Only)

Before initializing Terraform, you must create the S3 bucket and DynamoDB table for remote state management:

```bash
cd infrastructure/terraform

# Bootstrap for development environment
./bootstrap.sh us-east-1 dev

# Bootstrap for staging environment
./bootstrap.sh us-east-1 staging

# Bootstrap for production environment
./bootstrap.sh us-east-1 prod
```

The bootstrap script will:
1. Create an S3 bucket for Terraform state
2. Enable versioning and encryption on the bucket
3. Block public access to the bucket
4. Create a DynamoDB table for state locking
5. Enable point-in-time recovery on the DynamoDB table

### Initialize Terraform

```bash
cd infrastructure/terraform

# Initialize with backend configuration
terraform init -backend-config=backend-config.hcl
```

**Note:** The `backend-config.hcl` file contains the S3 bucket and DynamoDB table names. Update this file if you used different names during bootstrap.

### Plan Infrastructure Changes

```bash
# For development environment
terraform plan -var-file="environments/dev.tfvars"

# For staging environment
terraform plan -var-file="environments/staging/terraform.tfvars"

# For production environment
terraform plan -var-file="environments/production/terraform.tfvars"
```

### Apply Infrastructure Changes

```bash
# Apply changes (requires approval)
terraform apply -var-file="environments/production/terraform.tfvars"

# Auto-approve (use with caution)
terraform apply -auto-approve -var-file="environments/production/terraform.tfvars"
```

## Environment Separation

PredictIQ uses separate Terraform state files and backends for each environment:

- **Development**: Local state (for testing only)
- **Staging**: Remote state in S3 with DynamoDB locking
- **Production**: Remote state in separate S3 bucket with DynamoDB locking

See `environments/README.md` for detailed environment management instructions.

## Environments

### Development (dev)
- Single-node Redis
- Micro RDS instance
- 1 API task
- Minimal resources for testing

### Staging (staging)
- Multi-node Redis (2 nodes)
- Small RDS instance
- 2 API tasks
- Production-like configuration

### Production (prod)
- Multi-node Redis (3 nodes)
- Medium RDS instance
- 3 API tasks
- High availability setup

## Key Components

### VPC Module
- Creates isolated network environment
- Configurable CIDR blocks
- Public and private subnets
- NAT Gateway for private subnet egress

### RDS Module
- PostgreSQL database
- Automated backups
- Multi-AZ deployment (prod)
- Encryption at rest

### Redis Module
- ElastiCache cluster
- Automatic failover
- Parameter group configuration
- Subnet group for VPC placement

### ECS Module
- Fargate launch type
- Application Load Balancer
- Auto-scaling policies
- CloudWatch logging

### Monitoring Module
- CloudWatch dashboards
- SNS alerts
- Log groups
- Metric alarms

## State Management

Terraform state is stored in S3 with:
- Encryption enabled
- Versioning enabled
- DynamoDB table for state locking
- Restricted IAM access

## Deployment Process

1. Create feature branch for infrastructure changes
2. Update Terraform files
3. Run `terraform plan` and review changes
4. Create pull request with plan output
5. After approval, merge to main
6. GitHub Actions automatically applies changes

## Monitoring

Monitor infrastructure health:

```bash
# View Terraform state
terraform show

# Get outputs
terraform output

# Check AWS resources
aws ec2 describe-instances --filters "Name=tag:Project,Values=predictiq"
aws rds describe-db-instances
aws elasticache describe-cache-clusters
```

## Troubleshooting

### State Lock Issues

```bash
# Force unlock (use with caution)
terraform force-unlock <LOCK_ID>
```

### Resource Conflicts

```bash
# Import existing resource
terraform import module.vpc.aws_vpc.main vpc-12345678

# Remove resource from state
terraform state rm module.vpc.aws_vpc.main
```

### Plan Failures

```bash
# Refresh state
terraform refresh

# Validate configuration
terraform validate

# Format configuration
terraform fmt -recursive
```

## Security Best Practices

- Never commit sensitive values to git
- Use AWS Secrets Manager for secrets
- Enable MFA for AWS console access
- Restrict Terraform state bucket access
- Enable CloudTrail for audit logging
- Use IAM roles instead of access keys
- Implement least privilege access

## Cost Optimization

- Use spot instances for non-critical workloads
- Right-size instances based on metrics
- Enable auto-scaling for variable workloads
- Use reserved instances for baseline capacity
- Monitor unused resources

## Support

For infrastructure issues:
1. Check CloudWatch logs
2. Review Terraform state
3. Consult ROLLBACK.md for recovery procedures
4. Contact infrastructure team

## References

- [Terraform Documentation](https://www.terraform.io/docs)
- [AWS Provider Documentation](https://registry.terraform.io/providers/hashicorp/aws/latest/docs)
- [PredictIQ Architecture](../docs/ARCHITECTURE.md)
