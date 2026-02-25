#!/bin/bash

# Rate Limiting Test Script for PredictIQ API

API_URL="${API_URL:-http://localhost:8080}"
COLORS=true

# Colors
if [ "$COLORS" = true ]; then
    RED='\033[0;31m'
    GREEN='\033[0;32m'
    YELLOW='\033[1;33m'
    BLUE='\033[0;34m'
    NC='\033[0m' # No Color
else
    RED=''
    GREEN=''
    YELLOW=''
    BLUE=''
    NC=''
fi

echo -e "${BLUE}=== PredictIQ API Rate Limiting Tests ===${NC}\n"

# Test 1: Global Rate Limit (100 req/min)
echo -e "${YELLOW}Test 1: Global Rate Limit (100 req/min per IP)${NC}"
echo "Sending 110 requests to /health endpoint..."

success_count=0
rate_limited_count=0

for i in {1..110}; do
    response=$(curl -s -o /dev/null -w "%{http_code}" "$API_URL/health")
    
    if [ "$response" = "200" ]; then
        ((success_count++))
    elif [ "$response" = "429" ]; then
        ((rate_limited_count++))
    fi
    
    # Show progress every 10 requests
    if [ $((i % 10)) -eq 0 ]; then
        echo "  Progress: $i/110 requests sent"
    fi
done

echo -e "  ${GREEN}Successful: $success_count${NC}"
echo -e "  ${RED}Rate Limited: $rate_limited_count${NC}"

if [ $rate_limited_count -gt 0 ]; then
    echo -e "  ${GREEN}✓ Global rate limiting is working${NC}\n"
else
    echo -e "  ${RED}✗ Global rate limiting may not be working${NC}\n"
fi

# Wait for rate limit window to reset
echo -e "${YELLOW}Waiting 5 seconds before next test...${NC}\n"
sleep 5

# Test 2: Newsletter Rate Limit (5 req/hour)
echo -e "${YELLOW}Test 2: Newsletter Rate Limit (5 req/hour per IP)${NC}"
echo "Sending 10 newsletter subscription requests..."

newsletter_success=0
newsletter_limited=0

for i in {1..10}; do
    response=$(curl -s -o /dev/null -w "%{http_code}" \
        -X POST "$API_URL/api/v1/newsletter/subscribe" \
        -H "Content-Type: application/json" \
        -d "{\"email\":\"test$i@example.com\"}")
    
    if [ "$response" = "200" ] || [ "$response" = "400" ] || [ "$response" = "409" ]; then
        ((newsletter_success++))
    elif [ "$response" = "429" ]; then
        ((newsletter_limited++))
    fi
    
    echo "  Request $i: HTTP $response"
done

echo -e "  ${GREEN}Processed: $newsletter_success${NC}"
echo -e "  ${RED}Rate Limited: $newsletter_limited${NC}"

if [ $newsletter_limited -gt 0 ]; then
    echo -e "  ${GREEN}✓ Newsletter rate limiting is working${NC}\n"
else
    echo -e "  ${RED}✗ Newsletter rate limiting may not be working${NC}\n"
fi

# Test 3: Security Headers
echo -e "${YELLOW}Test 3: Security Headers${NC}"
echo "Checking security headers on /health endpoint..."

headers=$(curl -s -I "$API_URL/health")

check_header() {
    header_name=$1
    if echo "$headers" | grep -qi "$header_name"; then
        echo -e "  ${GREEN}✓ $header_name present${NC}"
        return 0
    else
        echo -e "  ${RED}✗ $header_name missing${NC}"
        return 1
    fi
}

check_header "content-security-policy"
check_header "x-frame-options"
check_header "x-content-type-options"
check_header "x-xss-protection"
check_header "strict-transport-security"
check_header "referrer-policy"

echo ""

# Test 4: Input Validation
echo -e "${YELLOW}Test 4: Input Validation${NC}"
echo "Testing SQL injection detection..."

sql_injection_response=$(curl -s -o /dev/null -w "%{http_code}" \
    "$API_URL/api/content?page=1' OR '1'='1")

if [ "$sql_injection_response" = "400" ]; then
    echo -e "  ${GREEN}✓ SQL injection attempt blocked (HTTP 400)${NC}"
else
    echo -e "  ${RED}✗ SQL injection attempt not blocked (HTTP $sql_injection_response)${NC}"
fi

echo "Testing path traversal detection..."

path_traversal_response=$(curl -s -o /dev/null -w "%{http_code}" \
    "$API_URL/api/../../../etc/passwd")

if [ "$path_traversal_response" = "400" ] || [ "$path_traversal_response" = "404" ]; then
    echo -e "  ${GREEN}✓ Path traversal attempt blocked (HTTP $path_traversal_response)${NC}"
else
    echo -e "  ${RED}✗ Path traversal attempt not blocked (HTTP $path_traversal_response)${NC}"
fi

echo ""

# Test 5: API Key Authentication
echo -e "${YELLOW}Test 5: API Key Authentication${NC}"
echo "Testing admin endpoint without API key..."

no_key_response=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$API_URL/api/markets/1/resolve")

if [ "$no_key_response" = "401" ] || [ "$no_key_response" = "403" ]; then
    echo -e "  ${GREEN}✓ Request without API key blocked (HTTP $no_key_response)${NC}"
else
    echo -e "  ${RED}✗ Request without API key not blocked (HTTP $no_key_response)${NC}"
fi

echo "Testing admin endpoint with invalid API key..."

invalid_key_response=$(curl -s -o /dev/null -w "%{http_code}" \
    -X POST "$API_URL/api/markets/1/resolve" \
    -H "X-API-Key: invalid-key-12345")

if [ "$invalid_key_response" = "401" ] || [ "$invalid_key_response" = "403" ]; then
    echo -e "  ${GREEN}✓ Request with invalid API key blocked (HTTP $invalid_key_response)${NC}"
else
    echo -e "  ${RED}✗ Request with invalid API key not blocked (HTTP $invalid_key_response)${NC}"
fi

echo ""

# Summary
echo -e "${BLUE}=== Test Summary ===${NC}"
echo -e "All critical security features have been tested."
echo -e "Review the results above to ensure all protections are working."
echo ""
echo -e "${YELLOW}Note:${NC} Some tests may show warnings if the API is not fully configured."
echo -e "Ensure environment variables are set correctly for production use."
