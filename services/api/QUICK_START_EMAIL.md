# Email Service Quick Start Guide

Get the email service up and running in 5 minutes.

## Prerequisites

- Rust 1.70+
- PostgreSQL 14+
- Redis 6+
- SendGrid account (free tier works)

## Step 1: SendGrid Setup (2 minutes)

1. **Sign up for SendGrid**
   - Go to https://sendgrid.com/free/
   - Create free account (100 emails/day)

2. **Verify Sender Email**
   - Go to Settings → Sender Authentication → Single Sender Verification
   - Add your email (e.g., noreply@yourdomain.com)
   - Check your email and verify

3. **Create API Key**
   - Go to Settings → API Keys
   - Click "Create API Key"
   - Name: "PredictIQ API"
   - Permissions: "Restricted Access" → Enable "Mail Send"
   - Copy the API key (starts with `SG.`)

## Step 2: Configure Environment (1 minute)

```bash
cd services/api

# Copy example env file
cp .env.example .env

# Edit .env and add your SendGrid credentials
nano .env
```

Update these lines:
```bash
SENDGRID_API_KEY=SG.your_actual_api_key_here
FROM_EMAIL=your-verified-email@example.com
BASE_URL=http://localhost:8080
```

## Step 3: Database Setup (1 minute)

```bash
# Run migrations
./scripts/run_migrations.sh

# Or manually:
psql $DATABASE_URL -f database/migrations/008_create_email_tracking.sql
```

## Step 4: Start the Service (1 minute)

```bash
# Build and run
cargo run

# Or with hot reload (requires cargo-watch)
cargo watch -x run
```

You should see:
```
Starting email queue worker
API listening on 0.0.0.0:8080
```

## Step 5: Test It! (30 seconds)

### Test 1: Preview an Email
```bash
curl http://localhost:8080/api/v1/email/preview/newsletter_confirmation | jq
```

Expected output:
```json
{
  "subject": "Confirm your newsletter subscription",
  "html_content": "<html>...",
  "text_content": "Please confirm..."
}
```

### Test 2: Send a Test Email
```bash
curl -X POST http://localhost:8080/api/v1/email/test \
  -H "Content-Type: application/json" \
  -d '{
    "recipient": "your-email@example.com",
    "template_name": "newsletter_confirmation"
  }' | jq
```

Expected output:
```json
{
  "success": true,
  "message": "Test email sent successfully",
  "message_id": "abc123..."
}
```

Check your inbox! 📧

### Test 3: Check Queue Stats
```bash
curl http://localhost:8080/api/v1/email/queue/stats | jq
```

Expected output:
```json
{
  "pending": 0,
  "processing": 0,
  "retry": 0
}
```

## Step 6: Configure Webhooks (Optional, 2 minutes)

For production, set up SendGrid webhooks to track email events:

1. **Expose Your Local Server** (for testing)
   ```bash
   # Install ngrok: https://ngrok.com/download
   ngrok http 8080
   ```
   
   Copy the HTTPS URL (e.g., `https://abc123.ngrok.io`)

2. **Configure SendGrid Webhook**
   - Go to Settings → Mail Settings → Event Webhook
   - HTTP Post URL: `https://abc123.ngrok.io/webhooks/sendgrid`
   - Select events: Delivered, Opened, Clicked, Bounced, Dropped, Spam Report, Unsubscribe
   - Click "Save"

3. **Test Webhook**
   Send a test email and watch the logs:
   ```bash
   # In your API logs, you should see:
   Processing SendGrid event: delivered for user@example.com
   ```

## Common Issues

### "SENDGRID_API_KEY not configured"
- Make sure `.env` file exists and has `SENDGRID_API_KEY=SG.xxx`
- Restart the service after updating `.env`

### "FROM_EMAIL not configured"
- Add `FROM_EMAIL=your-verified-email@example.com` to `.env`
- Make sure the email is verified in SendGrid

### "SendGrid API error 403"
- Your sender email is not verified
- Go to SendGrid → Settings → Sender Authentication
- Verify your email address

### "Connection refused" to Redis
- Make sure Redis is running: `redis-cli ping`
- Should return `PONG`
- Start Redis: `redis-server` or `brew services start redis`

### "Connection refused" to PostgreSQL
- Make sure PostgreSQL is running
- Check connection: `psql $DATABASE_URL -c "SELECT 1"`
- Update `DATABASE_URL` in `.env` if needed

## Next Steps

✅ **Production Setup**
- Read [EMAIL_SERVICE.md](./EMAIL_SERVICE.md) for full documentation
- Set up domain authentication in SendGrid
- Configure webhook with your production URL
- Set up monitoring and alerts

✅ **Customize Templates**
- Edit templates in `templates/` directory
- Add your branding and styling
- Test with preview endpoint

✅ **Monitor Performance**
- Check `/metrics` endpoint for Prometheus metrics
- Monitor queue stats: `/api/v1/email/queue/stats`
- View analytics: `/api/v1/email/analytics?days=30`

## Testing Checklist

Run the integration test suite:

```bash
./tests/email_integration_test.sh
```

Or test manually:

- [ ] Preview all 4 email templates
- [ ] Send test email to yourself
- [ ] Check email arrives in inbox
- [ ] Click links in email
- [ ] Verify webhook events are tracked
- [ ] Check analytics endpoint
- [ ] Test unsubscribe flow

## Support

- 📖 Full docs: [EMAIL_SERVICE.md](./EMAIL_SERVICE.md)
- 🐛 Issues: Check logs with `RUST_LOG=debug cargo run`
- 💬 Questions: Review the troubleshooting section in EMAIL_SERVICE.md

## Success! 🎉

You now have a production-ready email service with:
- ✅ Reliable email delivery via SendGrid
- ✅ Automatic retry on failures
- ✅ Event tracking and analytics
- ✅ Bounce and complaint handling
- ✅ Beautiful HTML email templates

Start sending emails in your application:

```rust
// In your handler
state.email_queue.enqueue(
    EmailJobType::WelcomeEmail,
    "user@example.com",
    "welcome_email",
    serde_json::json!({
        "name": "John Doe",
        "dashboard_url": "https://predictiq.com/dashboard"
    }),
    0,
).await?;
```

Happy emailing! 📧
