#!/bin/bash

# E2E Test Execution Script
# Quick test runner for PredictIQ E2E tests

set -e

echo "🧪 PredictIQ E2E Test Runner"
echo "=============================="
echo ""

# Colors
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Check if we're in the right directory
if [ ! -f "frontend/package.json" ]; then
    echo -e "${RED}❌ Error: Must run from project root${NC}"
    exit 1
fi

cd frontend

# Check if node_modules exists
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}⚠️  node_modules not found. Installing dependencies...${NC}"
    npm install
fi

# Check if Playwright is installed
if [ ! -d "node_modules/@playwright/test" ]; then
    echo -e "${YELLOW}⚠️  Playwright not found. Installing...${NC}"
    npm install @playwright/test
fi

# Check if browsers are installed
if [ ! -d "node_modules/@playwright/test/.local-browsers" ] && [ ! -d "$HOME/.cache/ms-playwright" ]; then
    echo -e "${YELLOW}⚠️  Playwright browsers not installed. Installing...${NC}"
    npx playwright install --with-deps
fi

echo ""
echo "Select test suite to run:"
echo "1) All tests"
echo "2) User journeys only"
echo "3) Mobile tests only"
echo "4) Performance tests only"
echo "5) Visual regression tests only"
echo "6) Accessibility tests only"
echo "7) Chrome only (all tests)"
echo "8) Firefox only (all tests)"
echo "9) Safari only (all tests)"
echo "10) Run with UI mode"
echo "11) Debug mode"
echo ""
read -p "Enter choice [1-11]: " choice

case $choice in
    1)
        echo -e "${GREEN}Running all E2E tests...${NC}"
        npm run test:e2e
        ;;
    2)
        echo -e "${GREEN}Running user journey tests...${NC}"
        npx playwright test user-journeys.spec.ts
        ;;
    3)
        echo -e "${GREEN}Running mobile tests...${NC}"
        npm run test:e2e:mobile
        ;;
    4)
        echo -e "${GREEN}Running performance tests...${NC}"
        npx playwright test performance.spec.ts
        ;;
    5)
        echo -e "${GREEN}Running visual regression tests...${NC}"
        npx playwright test visual-regression.spec.ts
        ;;
    6)
        echo -e "${GREEN}Running accessibility tests...${NC}"
        npx playwright test accessibility.spec.ts
        ;;
    7)
        echo -e "${GREEN}Running Chrome tests...${NC}"
        npm run test:e2e:chrome
        ;;
    8)
        echo -e "${GREEN}Running Firefox tests...${NC}"
        npm run test:e2e:firefox
        ;;
    9)
        echo -e "${GREEN}Running Safari tests...${NC}"
        npm run test:e2e:safari
        ;;
    10)
        echo -e "${GREEN}Running with UI mode...${NC}"
        npm run test:e2e:ui
        ;;
    11)
        echo -e "${GREEN}Running in debug mode...${NC}"
        npm run test:e2e:debug
        ;;
    *)
        echo -e "${RED}Invalid choice${NC}"
        exit 1
        ;;
esac

echo ""
echo -e "${GREEN}✅ Test execution complete!${NC}"
echo ""
echo "View HTML report: npm run test:e2e:report"
echo "Report location: frontend/playwright-report/index.html"
