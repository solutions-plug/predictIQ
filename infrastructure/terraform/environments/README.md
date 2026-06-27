# Terraform Environments

This directory contains environment-specific configurations for PredictIQ infrastructure.

## Directory Structure

```
environments/
├── dev.tfvars                    # Development environment variables
├── staging/
│   ├── terraform.tfvars         # Staging environment variables
│   └── backend.hcl              # Staging backend configuration
└── production/
    ├── terraform.tfvars         # Production environment variables
    └── backend.hcl              # Production backend configuration
```

## Environment Separation

Each environment has:
- **Separate state files**: Stored in different S3 buckets with distinct keys
- **Separate DynamoDB tables**: For state locking to prevent concurrent modifications
- **Distinct resource naming**: All resources are prefixed with environment name
- **Different resource sizing**: Production has higher capacity than staging

## Deployment Instructions

### Development Environment

```bash
cd infrastructure/terraform
terraform init
terraform plan -var-file="environments/dev.tfvars"
terraform apply -var-file="environments/dev.tfvars"
```

### Staging Environment

```bash
cd infrastructure/terraform

# First time: bootstrap the backend
./bootstrap.sh us-east-1 staging

# Initialize with staging backend
terraform init -backend-config=environments/staging/backend.hcl

# Plan and apply
terraform plan -var-file="environments/staging/terraform.tfvars"
terraform apply -var-file="environments/staging/terraform.tfvars"
```

### Production Environment

```bash
cd infrastructure/terraform

# First time: bootstrap the backend
./bootstrap.sh us-east-1 production

# Initialize with production backend
terraform init -backend-config=environments/production/backend.hcl

# Plan and apply (requires explicit approval)
terraform plan -var-file="environments/production/terraform.tfvars"
terraform apply -var-file="environments/production/terraform.tfvars"
```

## CI/CD Deployment

### Staging Deployment

Staging deployments are automatic on merge to `main` branch:

```yaml
- name: Deploy to Staging
  run: |
    cd infrastructure/terraform
    terraform init -backend-config=environments/staging/backend.hcl
    terraform plan -var-file="environments/staging/terraform.tfvars"
    terraform apply -auto-approve -var-file="environments/staging/terraform.tfvars"
```

### Production Deployment

Production deployments require explicit approval:

```yaml
- name: Plan Production Changes
  run: |
    cd infrastructure/terraform
    terraform init -backend-config=environments/production/backend.hcl
    terraform plan -var-file="environments/production/terraform.tfvars" -out=tfplan

- name: Approve and Apply Production
  if: github.event_name == 'workflow_dispatch'
  run: |
    cd infrastructure/terraform
    terraform apply tfplan
```

## State File Locations

| Environment | S3 Bucket | DynamoDB Table | State Key |
|-------------|-----------|----------------|-----------|
| Development | Local | N/A | N/A |
| Staging | `predictiq-terraform-state-staging` | `terraform-locks-staging` | `staging/terraform.tfstate` |
| Production | `predictiq-terraform-state-production` | `terraform-locks-production` | `production/terraform.tfstate` |

## Important Notes

### Preventing Accidental Production Changes

1. **State Locking**: DynamoDB tables prevent concurrent modifications
2. **Separate Backends**: Production state is isolated from staging
3. **CI/CD Approval**: Production changes require manual approval
4. **Resource Naming**: All resources include environment prefix (e.g., `predictiq-prod-vpc`)

### Switching Environments

When switching between environments, always reinitialize Terraform:

```bash
# Switch from staging to production
terraform init -backend-config=environments/production/backend.hcl -reconfigure

# Switch from production to staging
terraform init -backend-config=environments/staging/backend.hcl -reconfigure
```

### Disaster Recovery

If state is corrupted:

1. **Staging**: Can be recreated from scratch
2. **Production**: Contact infrastructure team before any recovery action

```bash
# Force unlock if state is locked
terraform force-unlock <LOCK_ID>

# Refresh state from AWS
terraform refresh -var-file="environments/production/terraform.tfvars"
```

## Monitoring Environment Health

```bash
# Check staging resources
aws ec2 describe-instances --filters "Name=tag:Environment,Values=staging"

# Check production resources
aws ec2 describe-instances --filters "Name=tag:Environment,Values=prod"

# View state file versions
aws s3api list-object-versions --bucket predictiq-terraform-state-production
```

## Best Practices

1. **Always test in staging first** before applying to production
2. **Review terraform plan output** carefully before applying
3. **Use terraform workspace** for additional isolation if needed
4. **Keep backend configurations** in version control (no secrets)
5. **Enable MFA** for production deployments
6. **Document all manual changes** made outside Terraform
7. **Regularly backup state files** using S3 versioning
