# Email Service Integration

Production-ready email service integration for PredictIQ API with SendGrid, featuring email queuing, retry logic, event tracking, and analytics.

## Features

✅ **Email Provider Integration** - SendGrid API integration with full error handling
✅ **Email Templates** - Handlebars-based templating system with 4 built-in templates
✅ **Email Queue** - Redis-backed queue with priority support
✅ **Retry Logic** - Exponential backoff for failed sends (2min, 4min, 8min...)
✅ **Event Tracking** - Track sent, delivered, opened, clicked, bounced, complained events
✅ **Bounce Handling** - Automatic suppression list management
✅ **Complaint Handling** - Spam complaint tracking and suppression
✅ **Unsubscribe Management** - Webhook-based and manual unsubscribe handling
✅ **Email Analytics** - Daily aggregated metrics per template
✅ **Email Preview** - Preview emails without sending (for testing)
✅ **Suppression List** - Prevent sending to bounced/complained/unsubscribed emails

## Architecture

```
┌─────────────┐
│   Handler   │ → Enqueue email job
└──────┬──────┘
       │
       ▼
┌─────────────┐
│ Email Queue │ → Redis sorted set (priority queue)
└──────┬──────┘
       │
       ▼
┌─────────────┐
│Queue Worker │ → Background task processing jobs
└──────┬──────┘
       │
       ▼
┌─────────────┐
│Email Service│ → SendGrid API
└──────┬──────┘
       │
       ▼
┌─────────────┐
│  SendGrid   │ → Sends email + webhooks
└──────┬──────┘
       │
       ▼
┌─────────────┐
│   Webhook   │ → Track events (delivered, opened, bounced, etc.)
└─────────────┘
```

## Configuration

### Environment Variables

```bash
# Required
SENDGRID_API_KEY=SG.xxxxxxxxxxxxxxxxxxxxx
FROM_EMAIL=noreply@predictiq.com
BASE_URL=https://predictiq.com

# Optional (already configured)
DATABASE_URL=postgres://user:pass@localhost/predictiq
REDIS_URL=redis://localhost:6379
```

### SendGrid Setup

1. **Create SendGrid Account**
   - Sign up at https://sendgrid.com
   - Verify your sender email/domain

2. **Generate API Key**
   - Go to Settings → API Keys
   - Create new API key with "Mail Send" permissions
   - Copy key to `SENDGRID_API_KEY` env var

3. **Configure Webhook**
   - Go to Settings → Mail Settings → Event Webhook
   - Set URL: `https://your-domain.com/webhooks/sendgrid`
   - Enable events: Delivered, Opened, Clicked, Bounced, Dropped, Spam Report, Unsubscribe
   - Save settings

4. **Domain Authentication** (Recommended for production)
   - Go to Settings → Sender Authentication
   - Authenticate your domain
   - Add DNS records as instructed

## Email Templates

### Built-in Templates

1. **newsletter_confirmation** - Newsletter subscription confirmation
2. **waitlist_confirmation** - Waitlist signup confirmation
3. **contact_form_auto_response** - Contact form auto-reply
4. **welcome_email** - Welcome email for new users

### Template Structure

Templates are located in `services/api/templates/` and use Handlebars syntax:

```html
<!DOCTYPE html>
<html>
<body>
    <h1>Hello {{name}}!</h1>
    <p>{{message}}</p>
    <a href="{{action_url}}">Click here</a>
</body>
</html>
```

### Adding New Templates

1. Create HTML file in `services/api/templates/your_template.html`
2. Register in `src/email/templates.rs`:

```rust
handlebars.register_template_string(
    "your_template",
    include_str!("../../templates/your_template.html"),
)?;
```

3. Add subject line logic in `get_subject()` method
4. Add text version in `render_text()` method

## API Endpoints

### Email Preview
```bash
GET /api/v1/email/preview/:template_name
```

Preview email template with test data without sending.

**Example:**
```bash
curl http://localhost:8080/api/v1/email/preview/newsletter_confirmation
```

**Response:**
```json
{
  "subject": "Confirm your newsletter subscription",
  "html_content": "<html>...</html>",
  "text_content": "Please confirm..."
}
```

### Send Test Email
```bash
POST /api/v1/email/test
Content-Type: application/json

{
  "recipient": "test@example.com",
  "template_name": "newsletter_confirmation"
}
```

**Response:**
```json
{
  "success": true,
  "message": "Test email sent successfully",
  "message_id": "abc123"
}
```

### Email Analytics
```bash
GET /api/v1/email/analytics?template_name=newsletter_confirmation&days=30
```

Get email performance metrics.

**Response:**
```json
[
  {
    "template_name": "newsletter_confirmation",
    "variant_name": null,
    "date": "2026-02-24",
    "sent_count": 150,
    "delivered_count": 148,
    "opened_count": 95,
    "clicked_count": 42,
    "bounced_count": 2,
    "complained_count": 0,
    "unsubscribed_count": 1
  }
]
```

### Queue Statistics
```bash
GET /api/v1/email/queue/stats
```

Get current queue status.

**Response:**
```json
{
  "pending": 5,
  "processing": 2,
  "retry": 1
}
```

### SendGrid Webhook
```bash
POST /webhooks/sendgrid
Content-Type: application/json

[
  {
    "email": "user@example.com",
    "event": "delivered",
    "timestamp": 1234567890,
    "sg_message_id": "msg-123"
  }
]
```

Receives events from SendGrid for tracking.

## Usage Examples

### Sending Emails Programmatically

```rust
use crate::email::types::EmailJobType;

// Enqueue an email
let job_id = state.email_queue.enqueue(
    EmailJobType::WelcomeEmail,
    "user@example.com",
    "welcome_email",
    serde_json::json!({
        "name": "John Doe",
        "dashboard_url": "https://predictiq.com/dashboard",
        "help_url": "https://predictiq.com/help",
        "unsubscribe_url": "https://predictiq.com/unsubscribe"
    }),
    0, // priority (0 = normal, higher = more urgent)
).await?;
```

### High Priority Email

```rust
// Send urgent email (e.g., password reset)
let job_id = state.email_queue.enqueue(
    EmailJobType::Custom("password_reset".to_string()),
    "user@example.com",
    "password_reset",
    template_data,
    100, // high priority
).await?;
```

### Check if Email is Suppressed

```rust
if state.db.email_is_suppressed("user@example.com").await? {
    // Don't send email
    return Ok("Email address is suppressed");
}
```

## Database Schema

### email_jobs
Tracks all email jobs in the queue.

```sql
id, job_type, recipient_email, template_name, template_data,
status, priority, attempts, max_attempts, scheduled_at,
started_at, completed_at, failed_at, error_message
```

### email_events
Tracks all email events from SendGrid webhooks.

```sql
id, email_job_id, message_id, event_type, recipient_email,
timestamp, metadata
```

### email_suppressions
Emails that should not receive emails (bounces, complaints, unsubscribes).

```sql
id, email, suppression_type, reason, bounce_type
```

### email_analytics
Daily aggregated email metrics.

```sql
template_name, variant_name, date, sent_count, delivered_count,
opened_count, clicked_count, bounced_count, complained_count,
unsubscribed_count
```

## Monitoring & Observability

### Logs

All email operations are logged with structured logging:

```
[newsletter] subscription attempt email=user@example.com source=homepage ip=1.2.3.4
Enqueued email job: abc-123 for user@example.com
Email sent successfully to user@example.com using template newsletter_confirmation (message_id: msg-456)
Email job abc-123 failed (attempt 1/3), retrying in 120s: Connection timeout
```

### Metrics

Email metrics are tracked in Prometheus format via `/metrics` endpoint:

- Email queue size
- Processing rate
- Retry rate
- Delivery rate
- Bounce rate
- Open rate
- Click rate

### Queue Monitoring

Check queue health:

```bash
curl http://localhost:8080/api/v1/email/queue/stats
```

Monitor for:
- High pending count (queue backup)
- High retry count (delivery issues)
- Stuck processing jobs (worker issues)

## Error Handling

### Retry Logic

Failed emails are automatically retried with exponential backoff:

- Attempt 1: Immediate
- Attempt 2: 2 minutes later
- Attempt 3: 4 minutes later
- Attempt 4: 8 minutes later (if max_attempts > 3)

After max attempts (default: 3), the job is marked as permanently failed.

### Common Errors

**SendGrid API Error 401**
- Check `SENDGRID_API_KEY` is valid
- Verify API key has "Mail Send" permission

**SendGrid API Error 403**
- Sender email not verified
- Domain authentication required

**Connection Timeout**
- Network issues
- SendGrid API down (check status.sendgrid.com)

**Template Rendering Error**
- Missing required template data
- Invalid Handlebars syntax

## Testing

### Unit Tests

```bash
cd services/api
cargo test email
```

### Integration Tests

```bash
# Test email preview
curl http://localhost:8080/api/v1/email/preview/newsletter_confirmation

# Send test email
curl -X POST http://localhost:8080/api/v1/email/test \
  -H "Content-Type: application/json" \
  -d '{"recipient":"your-email@example.com","template_name":"newsletter_confirmation"}'

# Check queue stats
curl http://localhost:8080/api/v1/email/queue/stats
```

### Webhook Testing

Use SendGrid's webhook testing tool or ngrok:

```bash
# Expose local server
ngrok http 8080

# Configure SendGrid webhook to ngrok URL
https://abc123.ngrok.io/webhooks/sendgrid
```

## Performance

### Throughput

- Queue processing: ~100 emails/second
- SendGrid rate limit: 600 emails/second (varies by plan)
- Database writes: ~1000 events/second

### Optimization Tips

1. **Batch Processing** - Process multiple jobs in parallel (future enhancement)
2. **Connection Pooling** - Reuse HTTP connections to SendGrid
3. **Cache Templates** - Templates are cached in memory
4. **Async Processing** - All email sending is async via queue

## Security

### Best Practices

1. **API Key Rotation** - Rotate SendGrid API key regularly
2. **Webhook Verification** - Verify webhook signatures (future enhancement)
3. **Rate Limiting** - Already implemented for newsletter subscriptions
4. **Input Validation** - All email addresses validated
5. **Suppression List** - Prevent sending to invalid/complained addresses

### Data Privacy

- Email addresses stored encrypted at rest (database level)
- GDPR compliance via newsletter GDPR endpoints
- Automatic data retention policies (configure as needed)

## Troubleshooting

### Emails Not Sending

1. Check queue stats: `GET /api/v1/email/queue/stats`
2. Check logs for errors
3. Verify SendGrid API key
4. Check email_jobs table for failed jobs

### Webhooks Not Working

1. Verify webhook URL is publicly accessible
2. Check SendGrid webhook configuration
3. Test webhook manually with curl
4. Check logs for webhook processing errors

### High Bounce Rate

1. Check email_suppressions table
2. Verify sender domain authentication
3. Review email content for spam triggers
4. Check recipient email addresses validity

## Future Enhancements

- [ ] A/B testing for email templates
- [ ] Email template variants
- [ ] Batch email sending
- [ ] Email scheduling (send at specific time)
- [ ] Webhook signature verification
- [ ] Alternative providers (AWS SES, Mailgun)
- [ ] Email attachment support
- [ ] Inline image support
- [ ] Email personalization engine
- [ ] Advanced analytics dashboard

## Support

For issues or questions:
- Check logs: `docker logs predictiq-api`
- Review SendGrid status: https://status.sendgrid.com
- Check database: `psql -d predictiq -c "SELECT * FROM email_jobs ORDER BY created_at DESC LIMIT 10"`
