#!/bin/bash
set -e

# Bootstrap script to create S3 bucket and DynamoDB table for Terraform state management
# Usage: ./bootstrap.sh <aws-region> <environment>

AWS_REGION=${1:-us-east-1}
ENVIRONMENT=${2:-dev}
BUCKET_NAME="predictiq-terraform-state-${ENVIRONMENT}"
LOCK_TABLE="terraform-locks-${ENVIRONMENT}"

echo "Bootstrapping Terraform state backend for environment: $ENVIRONMENT in region: $AWS_REGION"

# Create S3 bucket
echo "Creating S3 bucket: $BUCKET_NAME"
aws s3api create-bucket \
  --bucket "$BUCKET_NAME" \
  --region "$AWS_REGION" \
  $([ "$AWS_REGION" != "us-east-1" ] && echo "--create-bucket-configuration LocationConstraint=$AWS_REGION") \
  2>/dev/null || echo "Bucket already exists or error occurred"

# Enable versioning
echo "Enabling versioning on S3 bucket"
aws s3api put-bucket-versioning \
  --bucket "$BUCKET_NAME" \
  --versioning-configuration Status=Enabled \
  --region "$AWS_REGION"

# Enable encryption
echo "Enabling server-side encryption on S3 bucket"
aws s3api put-bucket-encryption \
  --bucket "$BUCKET_NAME" \
  --server-side-encryption-configuration '{
    "Rules": [
      {
        "ApplyServerSideEncryptionByDefault": {
          "SSEAlgorithm": "AES256"
        }
      }
    ]
  }' \
  --region "$AWS_REGION"

# Block public access
echo "Blocking public access to S3 bucket"
aws s3api put-public-access-block \
  --bucket "$BUCKET_NAME" \
  --public-access-block-configuration \
  "BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true" \
  --region "$AWS_REGION"

# Create DynamoDB table for state locking
echo "Creating DynamoDB table: $LOCK_TABLE"
aws dynamodb create-table \
  --table-name "$LOCK_TABLE" \
  --attribute-definitions AttributeName=LockID,AttributeType=S \
  --key-schema AttributeName=LockID,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST \
  --region "$AWS_REGION" \
  2>/dev/null || echo "Table already exists or error occurred"

# Enable point-in-time recovery
echo "Enabling point-in-time recovery on DynamoDB table"
aws dynamodb update-continuous-backups \
  --table-name "$LOCK_TABLE" \
  --point-in-time-recovery-specification PointInTimeRecoveryEnabled=true \
  --region "$AWS_REGION" \
  2>/dev/null || echo "PITR already enabled or error occurred"

echo "Bootstrap complete!"
echo "S3 Bucket: $BUCKET_NAME"
echo "DynamoDB Table: $LOCK_TABLE"
echo ""
echo "Next steps:"
echo "1. Update infrastructure/terraform/backend-config.hcl with the bucket and table names"
echo "2. Run: terraform init -backend-config=backend-config.hcl"
