#!/bin/bash

# E2E Test Setup Verification Script
# Verifies that all E2E testing components are properly installed
# This script is idempotent: running it multiple times produces the same result

set -e

echo "🔍 Verifying E2E Test Setup..."
echo ""

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Trap errors and provide helpful messages
trap 'echo -e "${RED}❌ Script failed. Prerequisites may not be met.${NC}"; exit 1' ERR

# Check if we're in the right directory
if [ ! -d "frontend" ]; then
    echo -e "${RED}❌ Error: Must run from project root${NC}"
    echo "Current directory: $(pwd)"
    exit 1
fi

cd frontend

echo "📦 Checking dependencies..."

# Check if package.json exists
if [ ! -f "package.json" ]; then
    echo -e "${RED}❌ package.json not found${NC}"
    exit 1
fi
echo -e "${GREEN}✓${NC} package.json found"

# Check if Playwright is in package.json
if grep -q "@playwright/test" package.json; then
    echo -e "${GREEN}✓${NC} Playwright dependency found"
else
    echo -e "${RED}❌ Playwright dependency not found${NC}"
    exit 1
fi

# Check if node_modules exists, install if missing
if [ ! -d "node_modules" ]; then
    echo -e "${YELLOW}⚠${NC}  node_modules not found. Installing dependencies..."
    npm install
    echo -e "${GREEN}✓${NC} Dependencies installed"
else
    echo -e "${GREEN}✓${NC} node_modules found"
fi

echo ""
echo "📁 Checking test files..."

# Check test files
TEST_FILES=(
    "e2e/user-journeys.spec.ts"
    "e2e/interactions.spec.ts"
    "e2e/mobile.spec.ts"
    "e2e/performance.spec.ts"
    "e2e/visual-regression.spec.ts"
    "e2e/accessibility.spec.ts"
    "e2e/helpers.ts"
)

MISSING_TEST_FILES=0
for file in "${TEST_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $file"
    else
        echo -e "${YELLOW}⚠${NC}  $file not found (optional)"
        MISSING_TEST_FILES=$((MISSING_TEST_FILES + 1))
    fi
done

if [ $MISSING_TEST_FILES -gt 0 ]; then
    echo -e "${YELLOW}Note: Some test files are missing. This may be expected if tests are still being created.${NC}"
fi

echo ""
echo "⚙️  Checking configuration..."

# Check config files
CONFIG_FILES=(
    "playwright.config.ts"
    "scripts/run-e2e-tests.js"
)

MISSING_CONFIG_FILES=0
for file in "${CONFIG_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $file"
    else
        echo -e "${YELLOW}⚠${NC}  $file not found (optional)"
        MISSING_CONFIG_FILES=$((MISSING_CONFIG_FILES + 1))
    fi
done

if [ $MISSING_CONFIG_FILES -gt 0 ]; then
    echo -e "${YELLOW}Note: Some config files are missing. This may be expected in minimal setups.${NC}"
fi

echo ""
echo "📚 Checking documentation..."

# Check documentation (optional)
DOC_FILES=(
    "E2E_TESTING_GUIDE.md"
    "e2e/README.md"
)

for file in "${DOC_FILES[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $file"
    else
        echo -e "${BLUE}ℹ${NC}  $file not found (optional)"
    fi
done

cd ..

# Check root documentation (optional)
ROOT_DOCS=(
    "IMPLEMENTATION_SUMMARY_ISSUE_92.md"
    "E2E_QUICK_REFERENCE.md"
    "PR_DESCRIPTION_ISSUE_92.md"
)

for file in "${ROOT_DOCS[@]}"; do
    if [ -f "$file" ]; then
        echo -e "${GREEN}✓${NC} $file"
    else
        echo -e "${BLUE}ℹ${NC}  $file not found (optional)"
    fi
done

echo ""
echo "🔧 Checking CI/CD..."

if [ -f ".github/workflows/e2e-tests.yml" ]; then
    echo -e "${GREEN}✓${NC} GitHub Actions workflow found"
else
    echo -e "${BLUE}ℹ${NC}  GitHub Actions workflow not found (optional)"
fi

echo ""
echo "📊 Summary"
echo "=========="

cd frontend

# Count test files (if they exist)
if [ -d "e2e" ]; then
    TEST_COUNT=$(find e2e -name "*.spec.ts" 2>/dev/null | wc -l)
    echo "Test files: $TEST_COUNT"
else
    echo "Test files: 0 (e2e directory not found)"
fi

# Check if Playwright is installed
if [ -d "node_modules/@playwright" ]; then
    echo -e "Playwright: ${GREEN}Installed${NC}"
    
    # Check if browsers are installed
    if [ -d "node_modules/@playwright/test" ]; then
        BROWSER_COUNT=$(find node_modules/@playwright -name "*.so" -o -name "*.dylib" -o -name "*.exe" 2>/dev/null | wc -l)
        if [ $BROWSER_COUNT -gt 0 ]; then
            echo -e "Browsers: ${GREEN}Installed${NC}"
        else
            echo -e "Browsers: ${YELLOW}Not installed${NC} (run: npm run playwright:install)"
        fi
    fi
else
    echo -e "Playwright: ${YELLOW}Not installed${NC} (run: npm install)"
fi

echo ""
echo "✅ Setup verification complete!"
echo ""
echo "Next steps:"
echo "1. cd frontend"
echo "2. npm install (if not done)"
echo "3. npm run playwright:install"
echo "4. npm run test:e2e:ui"
echo ""
echo "For more info, see: E2E_TESTING_GUIDE.md"
