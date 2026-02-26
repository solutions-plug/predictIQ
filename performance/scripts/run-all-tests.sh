#!/bin/bash

# Performance Test Runner
# Runs all performance tests and generates reports

set -e

echo "🚀 Starting Performance Test Suite"
echo "=================================="

# Configuration
API_URL="${API_URL:-http://localhost:8080}"
REPORTS_DIR="backend/reports"

# Create reports directory
mkdir -p "$REPORTS_DIR"

# Colors for output
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to run test and check result
run_test() {
    local test_name=$1
    local test_file=$2
    
    echo ""
    echo "📊 Running $test_name..."
    echo "-----------------------------------"
    
    if k6 run --out json="$REPORTS_DIR/${test_name}-raw.json" "$test_file"; then
        echo -e "${GREEN}✓ $test_name completed successfully${NC}"
        return 0
    else
        echo -e "${RED}✗ $test_name failed${NC}"
        return 1
    fi
}

# Check if API is running
echo "🔍 Checking API availability at $API_URL..."
if curl -s -f "$API_URL/health" > /dev/null; then
    echo -e "${GREEN}✓ API is running${NC}"
else
    echo -e "${RED}✗ API is not responding at $API_URL${NC}"
    echo "Please start the API server first"
    exit 1
fi

# Run tests
FAILED_TESTS=0

run_test "smoke-test" "backend/k6/smoke-test.js" || ((FAILED_TESTS++))
run_test "load-test" "backend/k6/load-test.js" || ((FAILED_TESTS++))
run_test "stress-test" "backend/k6/stress-test.js" || ((FAILED_TESTS++))
run_test "spike-test" "backend/k6/spike-test.js" || ((FAILED_TESTS++))
run_test "rate-limit-test" "backend/k6/rate-limit-test.js" || ((FAILED_TESTS++))
run_test "cache-test" "backend/k6/cache-test.js" || ((FAILED_TESTS++))

# Summary
echo ""
echo "=================================="
echo "📈 Performance Test Summary"
echo "=================================="

if [ $FAILED_TESTS -eq 0 ]; then
    echo -e "${GREEN}✓ All tests passed!${NC}"
    echo ""
    echo "Reports generated in: $REPORTS_DIR"
    exit 0
else
    echo -e "${RED}✗ $FAILED_TESTS test(s) failed${NC}"
    echo ""
    echo "Check reports in: $REPORTS_DIR"
    exit 1
fi
