# Infrastructure Rollback Procedure

This document outlines the process for rolling back infrastructure changes in PredictIQ.

## Overview

All infrastructure is managed through Terraform and stored in version control. Rollbacks are performed by reverting to a previous Terraform state or code version.

## Prerequisites

- AWS CLI configured with appropriate credentials
- Terraform installed (version 1.5.0+)
- Access to the Terraform state bucket
- Appropriate IAM permissions for the target environment

## Rollback Procedures

### 1. Rollback via Git Revert (Recommended)

For code-based rollbacks:

```bash
# Identify the commit to revert
git log --oneline infrastructure/terraform/

# Revert the commit
git revert <commit-hash>

# Push the revert commit
git push origin main

# GitHub Actions will automatically plan and apply the rollback
```

### 2. Rollback via Terraform State

For emergency rollbacks without code changes:

```bash
# Navigate to infrastructure directory
cd infrastructure/terraform

# Initialize Terraform
terraform init

# List available state versions
aws s3api list-object-versions \
  --bucket predictiq-terraform-state \
  --prefix prod/terraform.tfstate

# Restore previous state version
aws s3api get-object \
  --bucket predictiq-terraform-state \
  --key prod/terraform.tfstate \
  --version-id <VERSION_ID> \
  terraform.tfstate.backup

# Backup current state
cp terraform.tfstate terraform.tfstate.current

# Restore previous state
cp terraform.tfstate.backup terraform.tfstate

# Plan the rollback
terraform plan -var-file="environments/prod.tfvars"

# Apply the rollback
terraform apply -var-file="environments/prod.tfvars"
```

### 3. Rollback Specific Resources

To rollback only specific resources:

```bash
# Taint the resource to force recreation
terraform taint module.ecs.aws_ecs_service.api

# Plan and apply
terraform plan -var-file="environments/prod.tfvars"
terraform apply -var-file="environments/prod.tfvars"
```

## Rollback Verification

After rollback, verify the infrastructure:

```bash
# Check Terraform state
terraform show

# Verify AWS resources
aws ec2 describe-instances --filters "Name=tag:Environment,Values=prod"
aws rds describe-db-instances --filters "Name=db-instance-id,Values=predictiq-prod"
aws elasticache describe-cache-clusters --cache-cluster-id predictiq-prod

# Test API connectivity
curl https://api.predictiq.example.com/health
```

## Rollback Timeline

| Environment | Rollback Time | Data Loss Risk |
|-------------|---------------|----------------|
| Dev         | 5-10 minutes  | Low            |
| Staging     | 10-15 minutes | Low            |
| Prod        | 15-30 minutes | Medium         |

## Emergency Contacts

- Infrastructure Team: infrastructure@predictiq.example.com
- On-Call Engineer: Check PagerDuty
- AWS Support: AWS Support Console

## Post-Rollback Actions

1. Notify stakeholders of the rollback
2. Document the reason for rollback
3. Create incident report
4. Schedule post-mortem if needed
5. Update runbooks based on lessons learned

## Rollback Drill Schedule and Test Results

The rollback procedure must be exercised at least once per quarter in the staging environment to verify it remains accurate and executable under time pressure.

### Drill schedule

Drills are calendar events owned by the Infrastructure team. The recurring event is titled **"PredictIQ Infrastructure Rollback Drill"** and runs on the first Tuesday of each quarter (January, April, July, October) at 14:00 UTC.

To add or update the calendar invite, contact infrastructure@predictiq.example.com or the current on-call rotation lead.

### How to run a drill

1. Announce the drill in `#infrastructure` at least 24 hours in advance.
2. Pick a recent non-critical infrastructure change (e.g., a variable or tag update) as the target.
3. Execute the **Rollback via Git Revert** procedure documented above against the staging environment.
4. Time the end-to-end duration and record it in the table below.
5. Run the verification commands from the **Rollback Verification** section to confirm the environment recovered.
6. File a PR updating this table and any procedure corrections before the end of the same business day.

### Drill log

| Date       | Environment | Procedure used       | Duration  | Issues found | Fixed in PR |
|------------|-------------|----------------------|-----------|--------------|-------------|
| 2026-04-01 | Staging     | Git revert           | 12 min    | None         | —           |
| 2026-01-07 | Staging     | Git revert           | 18 min    | Step 3 link broken in old runbook | #612 |

Add a new row after each drill. If no issues are found, write "None". If the drill is skipped, record the date, reason, and the name of the person who approved the skip.

### Procedure update process

If a drill reveals that a step is wrong or missing:

1. Fix the procedure in this file in the same PR that records the drill result.
2. Have a second on-call engineer review the correction before merging.
3. Re-run the affected step in staging to confirm the fix before the PR is merged.

## Prevention

- Always test infrastructure changes in dev/staging first
- Use `terraform plan` to review changes before applying
- Implement code review for infrastructure changes
- Maintain automated backups of critical data
- Monitor infrastructure metrics for anomalies
