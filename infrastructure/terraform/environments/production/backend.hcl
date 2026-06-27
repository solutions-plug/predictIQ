# Backend configuration for production environment
# Usage: terraform init -backend-config=environments/production/backend.hcl

bucket         = "predictiq-terraform-state-production"
key            = "production/terraform.tfstate"
region         = "us-east-1"
encrypt        = true
dynamodb_table = "terraform-locks-production"
