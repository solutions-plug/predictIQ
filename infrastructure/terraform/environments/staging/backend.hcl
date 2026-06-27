# Backend configuration for staging environment
# Usage: terraform init -backend-config=environments/staging/backend.hcl

bucket         = "predictiq-terraform-state-staging"
key            = "staging/terraform.tfstate"
region         = "us-east-1"
encrypt        = true
dynamodb_table = "terraform-locks-staging"
