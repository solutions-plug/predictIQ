#!/bin/bash
set -euo pipefail

# Bootstrap script to create S3 bucket and DynamoDB table for Terraform state management
# Usage: ./bootstrap.sh <aws-region> <environment>
# Idempotent: safe to run multiple times.

AWS_REGION=${1:-us-east-1}
ENVIRONMENT=${2:-dev}
BUCKET_NAME="predictiq-terraform-state-${ENVIRONMENT}"
LOCK_TABLE="terraform-locks-${ENVIRONMENT}"

echo "Bootstrapping Terraform state backend for environment: $ENVIRONMENT in region: $AWS_REGION"

# ---------------------------------------------------------------------------
# S3 bucket
# ---------------------------------------------------------------------------
echo "Checking S3 bucket: $BUCKET_NAME"
if aws s3api head-bucket --bucket "$BUCKET_NAME" --region "$AWS_REGION" 2>/dev/null; then
  echo "  → Bucket already exists, skipping creation."
else
  echo "  → Creating S3 bucket: $BUCKET_NAME"
  if [ "$AWS_REGION" = "us-east-1" ]; then
    aws s3api create-bucket \
      --bucket "$BUCKET_NAME" \
      --region "$AWS_REGION"
  else
    aws s3api create-bucket \
      --bucket "$BUCKET_NAME" \
      --region "$AWS_REGION" \
      --create-bucket-configuration "LocationConstraint=$AWS_REGION"
  fi

  # Verify bucket was created
  aws s3api head-bucket --bucket "$BUCKET_NAME" --region "$AWS_REGION"
  echo "  → Bucket created and verified."
fi

echo "Enabling versioning on S3 bucket"
aws s3api put-bucket-versioning \
  --bucket "$BUCKET_NAME" \
  --versioning-configuration Status=Enabled \
  --region "$AWS_REGION"
VERSIONING=$(aws s3api get-bucket-versioning --bucket "$BUCKET_NAME" --region "$AWS_REGION" --query 'Status' --output text)
[ "$VERSIONING" = "Enabled" ] || { echo "ERROR: Versioning not enabled on $BUCKET_NAME"; exit 1; }

echo "Enabling server-side encryption on S3 bucket"
aws s3api put-bucket-encryption \
  --bucket "$BUCKET_NAME" \
  --server-side-encryption-configuration '{
    "Rules": [{"ApplyServerSideEncryptionByDefault": {"SSEAlgorithm": "AES256"}}]
  }' \
  --region "$AWS_REGION"
aws s3api get-bucket-encryption --bucket "$BUCKET_NAME" --region "$AWS_REGION" > /dev/null
echo "  → Encryption verified."

echo "Blocking public access to S3 bucket"
aws s3api put-public-access-block \
  --bucket "$BUCKET_NAME" \
  --public-access-block-configuration \
  "BlockPublicAcls=true,IgnorePublicAcls=true,BlockPublicPolicy=true,RestrictPublicBuckets=true" \
  --region "$AWS_REGION"
BLOCK=$(aws s3api get-public-access-block --bucket "$BUCKET_NAME" --region "$AWS_REGION" \
  --query 'PublicAccessBlockConfiguration.BlockPublicAcls' --output text)
[ "$BLOCK" = "True" ] || { echo "ERROR: Public access block not enabled on $BUCKET_NAME"; exit 1; }

# ---------------------------------------------------------------------------
# DynamoDB lock table
# ---------------------------------------------------------------------------
echo "Checking DynamoDB table: $LOCK_TABLE"
if aws dynamodb describe-table --table-name "$LOCK_TABLE" --region "$AWS_REGION" > /dev/null 2>&1; then
  echo "  → Table already exists, skipping creation."
else
  echo "  → Creating DynamoDB table: $LOCK_TABLE"
  aws dynamodb create-table \
    --table-name "$LOCK_TABLE" \
    --attribute-definitions AttributeName=LockID,AttributeType=S \
    --key-schema AttributeName=LockID,KeyType=HASH \
    --billing-mode PAY_PER_REQUEST \
    --region "$AWS_REGION"

  echo "  → Waiting for table to become active..."
  aws dynamodb wait table-exists --table-name "$LOCK_TABLE" --region "$AWS_REGION"

  # Verify
  STATUS=$(aws dynamodb describe-table --table-name "$LOCK_TABLE" --region "$AWS_REGION" \
    --query 'Table.TableStatus' --output text)
  [ "$STATUS" = "ACTIVE" ] || { echo "ERROR: DynamoDB table $LOCK_TABLE is not ACTIVE (status: $STATUS)"; exit 1; }
  echo "  → Table created and verified."
fi

echo "Enabling point-in-time recovery on DynamoDB table"
aws dynamodb update-continuous-backups \
  --table-name "$LOCK_TABLE" \
  --point-in-time-recovery-specification PointInTimeRecoveryEnabled=true \
  --region "$AWS_REGION" > /dev/null
PITR=$(aws dynamodb describe-continuous-backups --table-name "$LOCK_TABLE" --region "$AWS_REGION" \
  --query 'ContinuousBackupsDescription.PointInTimeRecoveryDescription.PointInTimeRecoveryStatus' --output text)
[ "$PITR" = "ENABLED" ] || { echo "ERROR: PITR not enabled on $LOCK_TABLE"; exit 1; }

# ---------------------------------------------------------------------------
# Success
# ---------------------------------------------------------------------------
echo ""
echo "✅ Bootstrap complete!"
echo "   S3 Bucket:       $BUCKET_NAME"
echo "   DynamoDB Table:  $LOCK_TABLE"
echo "   Region:          $AWS_REGION"
echo ""
echo "Paste the following backend configuration into your main.tf or backend-config.hcl:"
echo ""
echo "  terraform {"
echo "    backend \"s3\" {"
echo "      bucket         = \"$BUCKET_NAME\""
echo "      key            = \"${ENVIRONMENT}/terraform.tfstate\""
echo "      region         = \"$AWS_REGION\""
echo "      dynamodb_table = \"$LOCK_TABLE\""
echo "      encrypt        = true"
echo "    }"
echo "  }"
echo ""
echo "Then run: terraform init -backend-config=backend-config.hcl"
