#!/bin/bash
# Email Service Integration Tests

set -e

API_URL="${API_URL:-http://localhost:8080}"
TEST_EMAIL="${TEST_EMAIL:-test@example.com}"

echo "🧪 Email Service Integration Tests"
echo "API URL: $API_URL"
echo "Test Email: $TEST_EMAIL"
echo ""

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

test_passed() {
    echo -e "${GREEN}✓ $1${NC}"
}

test_failed() {
    echo -e "${RED}✗ $1${NC}"
    exit 1
}

# Test 1: Health Check
echo "Test 1: Health Check"
response=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/health")
if [ "$response" = "200" ]; then
    test_passed "Health check passed"
else
    test_failed "Health check failed (HTTP $response)"
fi

# Test 2: Email Preview - Newsletter Confirmation
echo ""
echo "Test 2: Email Preview - Newsletter Confirmation"
response=$(curl -s "$API_URL/api/v1/email/preview/newsletter_confirmation")
if echo "$response" | grep -q "subject"; then
    test_passed "Newsletter confirmation preview works"
else
    test_failed "Newsletter confirmation preview failed"
fi

# Test 3: Email Preview - Waitlist Confirmation
echo ""
echo "Test 3: Email Preview - Waitlist Confirmation"
response=$(curl -s "$API_URL/api/v1/email/preview/waitlist_confirmation")
if echo "$response" | grep -q "subject"; then
    test_passed "Waitlist confirmation preview works"
else
    test_failed "Waitlist confirmation preview failed"
fi

# Test 4: Email Preview - Contact Form Auto Response
echo ""
echo "Test 4: Email Preview - Contact Form Auto Response"
response=$(curl -s "$API_URL/api/v1/email/preview/contact_form_auto_response")
if echo "$response" | grep -q "subject"; then
    test_passed "Contact form auto response preview works"
else
    test_failed "Contact form auto response preview failed"
fi

# Test 5: Email Preview - Welcome Email
echo ""
echo "Test 5: Email Preview - Welcome Email"
response=$(curl -s "$API_URL/api/v1/email/preview/welcome_email")
if echo "$response" | grep -q "subject"; then
    test_passed "Welcome email preview works"
else
    test_failed "Welcome email preview failed"
fi

# Test 6: Queue Statistics
echo ""
echo "Test 6: Queue Statistics"
response=$(curl -s "$API_URL/api/v1/email/queue/stats")
if echo "$response" | grep -q "pending"; then
    test_passed "Queue statistics endpoint works"
else
    test_failed "Queue statistics endpoint failed"
fi

# Test 7: Email Analytics
echo ""
echo "Test 7: Email Analytics"
response=$(curl -s "$API_URL/api/v1/email/analytics?days=7")
if [ "$response" != "" ]; then
    test_passed "Email analytics endpoint works"
else
    test_failed "Email analytics endpoint failed"
fi

# Test 8: Send Test Email (only if SENDGRID_API_KEY is set)
if [ -n "$SENDGRID_API_KEY" ]; then
    echo ""
    echo "Test 8: Send Test Email"
    response=$(curl -s -X POST "$API_URL/api/v1/email/test" \
        -H "Content-Type: application/json" \
        -d "{\"recipient\":\"$TEST_EMAIL\",\"template_name\":\"newsletter_confirmation\"}")
    
    if echo "$response" | grep -q "success"; then
        test_passed "Test email sent successfully"
    else
        test_failed "Test email sending failed: $response"
    fi
else
    echo ""
    echo "⚠️  Skipping Test 8: SENDGRID_API_KEY not set"
fi

echo ""
echo "========================================="
echo "✅ All tests passed!"
echo "========================================="
