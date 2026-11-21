# Environments

This directory holds per-environment Terraform variable files (tfvars) used to
configure the root module in `infrastructure/terraform`.

## Required variables

| Variable | Description |
| --- | --- |
| `environment` | Deployment environment name (e.g. `dev`, `staging`, `prod`). |
| `alerts_subscription_endpoint` | Endpoint that receives CloudWatch alarm notifications from the alerts SNS topic (e.g. an email address or HTTPS URL). **Required to receive pages** — when left empty, no `aws_sns_topic_subscription` is created and alarms are silent. Do not commit real endpoints to tfvars files; supply this via `-var`, `TF_VAR_alerts_subscription_endpoint`, or a CI secret. |

## Optional variables

| Variable | Default | Description |
| --- | --- | --- |
| `aws_region` | `us-east-1` | AWS region to deploy into. |
| `alerts_subscription_protocol` | `email` | SNS subscription protocol for the alerts topic (`email`, `https`, `sqs`, `lambda`, ...). |

## Notes

- The `ci-validate.tfvars` file only sets a placeholder for
  `alerts_subscription_endpoint` so that `terraform validate`/`plan` succeed in
  CI. Real environments must provide a real endpoint out-of-band.
