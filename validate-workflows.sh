#!/bin/bash

# CI/CD Workflow Validation Script
# Checks if all workflows are properly configured

set -e

echo "🔍 Validating CI/CD Workflows..."
echo ""

WORKFLOWS_DIR=".github/workflows"
ERRORS=0

# Check if workflows directory exists
if [ ! -d "$WORKFLOWS_DIR" ]; then
    echo "❌ Workflows directory not found: $WORKFLOWS_DIR"
    exit 1
fi

echo "✅ Workflows directory found"
echo ""

# List all workflows
echo "📋 Found workflows:"
for workflow in "$WORKFLOWS_DIR"/*.yml; do
    echo "  - $(basename "$workflow")"
done
echo ""

# Validate YAML syntax
echo "🔧 Validating YAML syntax..."
for workflow in "$WORKFLOWS_DIR"/*.yml; do
    if command -v yamllint &> /dev/null; then
        if yamllint "$workflow" 2>/dev/null; then
            echo "  ✅ $(basename "$workflow")"
        else
            echo "  ⚠️  $(basename "$workflow") - syntax warnings (non-critical)"
        fi
    else
        # Basic YAML check
        if python3 -c "import yaml; yaml.safe_load(open('$workflow'))" 2>/dev/null; then
            echo "  ✅ $(basename "$workflow")"
        else
            echo "  ❌ $(basename "$workflow") - invalid YAML"
            ERRORS=$((ERRORS + 1))
        fi
    fi
done
echo ""

# Check for required files
echo "📦 Checking required files..."

# Frontend checks
if [ -d "frontend" ]; then
    echo "  ✅ frontend/ directory exists"
    
    if [ -f "frontend/package.json" ]; then
        echo "  ✅ frontend/package.json exists"
    else
        echo "  ❌ frontend/package.json missing"
        ERRORS=$((ERRORS + 1))
    fi
    
    if [ -f "frontend/playwright.config.ts" ]; then
        echo "  ✅ frontend/playwright.config.ts exists"
    else
        echo "  ⚠️  frontend/playwright.config.ts missing (E2E tests may fail)"
    fi
    
    if [ -d "frontend/e2e" ]; then
        echo "  ✅ frontend/e2e/ directory exists"
        TEST_COUNT=$(find frontend/e2e -name "*.spec.ts" | wc -l)
        echo "     Found $TEST_COUNT test files"
    else
        echo "  ⚠️  frontend/e2e/ directory missing"
    fi
else
    echo "  ⚠️  frontend/ directory missing"
fi
echo ""

# Contract checks
if [ -d "contracts/predict-iq" ]; then
    echo "  ✅ contracts/predict-iq/ directory exists"
    
    if [ -f "contracts/predict-iq/Cargo.toml" ]; then
        echo "  ✅ contracts/predict-iq/Cargo.toml exists"
    else
        echo "  ❌ contracts/predict-iq/Cargo.toml missing"
        ERRORS=$((ERRORS + 1))
    fi
    
    if [ -d "contracts/predict-iq/src" ]; then
        echo "  ✅ contracts/predict-iq/src/ directory exists"
    else
        echo "  ❌ contracts/predict-iq/src/ directory missing"
        ERRORS=$((ERRORS + 1))
    fi
else
    echo "  ❌ contracts/predict-iq/ directory missing"
    ERRORS=$((ERRORS + 1))
fi
echo ""

# Check workflow triggers
echo "🎯 Checking workflow triggers..."
for workflow in "$WORKFLOWS_DIR"/*.yml; do
    name=$(basename "$workflow")
    if grep -q "on:" "$workflow"; then
        echo "  ✅ $name has triggers configured"
    else
        echo "  ❌ $name missing triggers"
        ERRORS=$((ERRORS + 1))
    fi
done
echo ""

# Check for common issues
echo "🔍 Checking for common issues..."

# Check for hardcoded package-lock.json cache paths
if grep -r "cache-dependency-path.*package-lock.json" "$WORKFLOWS_DIR" 2>/dev/null; then
    echo "  ⚠️  Found hardcoded package-lock.json cache paths (may fail if file doesn't exist)"
fi

# Check for npm ci without fallback
if grep -r "npm ci" "$WORKFLOWS_DIR" | grep -v "npm install" 2>/dev/null; then
    echo "  ⚠️  Found 'npm ci' without fallback to 'npm install'"
fi

echo ""

# Summary
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
if [ $ERRORS -eq 0 ]; then
    echo "✅ All validations passed!"
    echo ""
    echo "Workflows are configured correctly and should pass CI/CD checks."
    echo ""
    echo "Next steps:"
    echo "  1. Commit changes: git add .github/workflows/"
    echo "  2. Push to trigger workflows: git push"
    echo "  3. Monitor at: https://github.com/YOUR_ORG/predictIQ/actions"
    exit 0
else
    echo "❌ Found $ERRORS critical error(s)"
    echo ""
    echo "Please fix the errors above before pushing to CI/CD."
    exit 1
fi
