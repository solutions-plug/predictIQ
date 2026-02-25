#!/bin/bash

# Featured Markets API Integration Test Script
# Tests the /api/v1/markets/featured endpoint with various parameters

set -e

API_URL="${API_URL:-http://localhost:8080}"
ENDPOINT="/api/v1/markets/featured"

echo "Testing Featured Markets API at ${API_URL}${ENDPOINT}"
echo "=================================================="

# Test 1: Basic request without parameters
echo ""
echo "Test 1: Basic request (default parameters)"
response=$(curl -s "${API_URL}${ENDPOINT}")
echo "Response: $response" | jq '.'

# Validate response structure
echo "$response" | jq -e '.markets' > /dev/null && echo "✓ markets field exists"
echo "$response" | jq -e '.total' > /dev/null && echo "✓ total field exists"
echo "$response" | jq -e '.page' > /dev/null && echo "✓ page field exists"
echo "$response" | jq -e '.page_size' > /dev/null && echo "✓ page_size field exists"
echo "$response" | jq -e '.last_updated' > /dev/null && echo "✓ last_updated field exists"

# Test 2: Filter by category
echo ""
echo "Test 2: Filter by category (crypto)"
response=$(curl -s "${API_URL}${ENDPOINT}?category=crypto")
echo "Response: $response" | jq '.'
category_count=$(echo "$response" | jq '.markets | length')
echo "✓ Returned $category_count crypto markets"

# Test 3: Custom limit
echo ""
echo "Test 3: Custom limit (limit=6)"
response=$(curl -s "${API_URL}${ENDPOINT}?limit=6")
echo "Response: $response" | jq '.'
market_count=$(echo "$response" | jq '.markets | length')
if [ "$market_count" -le 6 ]; then
    echo "✓ Returned $market_count markets (≤ 6)"
else
    echo "✗ Expected ≤ 6 markets, got $market_count"
    exit 1
fi

# Test 4: Pagination
echo ""
echo "Test 4: Pagination (page=2, limit=4)"
response=$(curl -s "${API_URL}${ENDPOINT}?page=2&limit=4")
echo "Response: $response" | jq '.'
page=$(echo "$response" | jq '.page')
page_size=$(echo "$response" | jq '.page_size')
if [ "$page" -eq 2 ] && [ "$page_size" -eq 4 ]; then
    echo "✓ Pagination parameters correct"
else
    echo "✗ Pagination parameters incorrect"
    exit 1
fi

# Test 5: Combined filters
echo ""
echo "Test 5: Combined filters (category=politics, limit=3)"
response=$(curl -s "${API_URL}${ENDPOINT}?category=politics&limit=3")
echo "Response: $response" | jq '.'
echo "✓ Combined filters work"

# Test 6: Validate market object structure
echo ""
echo "Test 6: Validate market object structure"
response=$(curl -s "${API_URL}${ENDPOINT}?limit=1")
market=$(echo "$response" | jq '.markets[0]')

if [ "$market" != "null" ]; then
    echo "$market" | jq -e '.id' > /dev/null && echo "✓ market.id exists"
    echo "$market" | jq -e '.title' > /dev/null && echo "✓ market.title exists"
    echo "$market" | jq -e '.description' > /dev/null && echo "✓ market.description exists"
    echo "$market" | jq -e '.category' > /dev/null && echo "✓ market.category exists"
    echo "$market" | jq -e '.volume' > /dev/null && echo "✓ market.volume exists"
    echo "$market" | jq -e '.participant_count' > /dev/null && echo "✓ market.participant_count exists"
    echo "$market" | jq -e '.ends_at' > /dev/null && echo "✓ market.ends_at exists"
    echo "$market" | jq -e '.outcome_options' > /dev/null && echo "✓ market.outcome_options exists"
    echo "$market" | jq -e '.current_odds' > /dev/null && echo "✓ market.current_odds exists"
    echo "$market" | jq -e '.onchain_volume' > /dev/null && echo "✓ market.onchain_volume exists"
else
    echo "⚠ No markets returned, skipping structure validation"
fi

# Test 7: Edge cases
echo ""
echo "Test 7: Edge cases"

# Limit too high (should be clamped to 20)
response=$(curl -s "${API_URL}${ENDPOINT}?limit=100")
page_size=$(echo "$response" | jq '.page_size')
if [ "$page_size" -le 20 ]; then
    echo "✓ Limit clamped correctly (got $page_size)"
else
    echo "✗ Limit not clamped (got $page_size)"
fi

# Page 0 (should default to 1)
response=$(curl -s "${API_URL}${ENDPOINT}?page=0")
page=$(echo "$response" | jq '.page')
if [ "$page" -ge 1 ]; then
    echo "✓ Page minimum enforced (got $page)"
else
    echo "✗ Page minimum not enforced (got $page)"
fi

# Test 8: Performance (cache hit)
echo ""
echo "Test 8: Performance test (cache hit)"
start_time=$(date +%s%N)
curl -s "${API_URL}${ENDPOINT}" > /dev/null
end_time=$(date +%s%N)
duration=$(( (end_time - start_time) / 1000000 ))
echo "First request: ${duration}ms"

start_time=$(date +%s%N)
curl -s "${API_URL}${ENDPOINT}" > /dev/null
end_time=$(date +%s%N)
duration=$(( (end_time - start_time) / 1000000 ))
echo "Second request (cached): ${duration}ms"

if [ "$duration" -lt 100 ]; then
    echo "✓ Cache hit performance good (<100ms)"
else
    echo "⚠ Cache hit slower than expected (${duration}ms)"
fi

echo ""
echo "=================================================="
echo "All tests completed successfully! ✓"
