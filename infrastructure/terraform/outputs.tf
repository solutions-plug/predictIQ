output "alerts_topic_arn" {
  description = "ARN of the SNS topic used for CloudWatch alarm notifications"
  value       = module.monitoring.alerts_topic_arn
}

output "alerts_subscription_arn" {
  description = "ARN of the SNS subscription that delivers alarm notifications to the configured endpoint"
  value       = try(aws_sns_topic_subscription.alerts[0].arn, null)
}
