# Backend configuration for Terraform state management
# This file is used during terraform init to configure the S3 backend
# Usage: terraform init -backend-config=backend-config.hcl

bucket         = "predictiq-terraform-state"
key            = "terraform.tfstate"
region         = "us-east-1"
encrypt        = true
dynamodb_table = "terraform-locks"
