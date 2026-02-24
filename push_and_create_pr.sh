#!/bin/bash

# Script to push branch and create PR for Issue #14
# Run this from the project root: bash push_and_create_pr.sh

set -e

echo "╔══════════════════════════════════════════════════════════════╗"
echo "║  Pushing Issue #14 Branch and Creating Pull Request         ║"
echo "╚══════════════════════════════════════════════════════════════╝"
echo ""

# Check if we're on the correct branch
CURRENT_BRANCH=$(git branch --show-current)
EXPECTED_BRANCH="features/issue-14-Permissioned-Creation-Tiered-Market-Levels"

if [ "$CURRENT_BRANCH" != "$EXPECTED_BRANCH" ]; then
    echo "❌ Error: Not on the correct branch"
    echo "   Current: $CURRENT_BRANCH"
    echo "   Expected: $EXPECTED_BRANCH"
    exit 1
fi

echo "✅ On correct branch: $CURRENT_BRANCH"
echo ""

# Show commits to be pushed
echo "📦 Commits to be pushed:"
git log --oneline -2
echo ""

# Push the branch
echo "🚀 Pushing branch to origin..."
git push origin "$CURRENT_BRANCH"
echo ""

echo "✅ Branch pushed successfully!"
echo ""

# Check if gh CLI is available
if command -v gh &> /dev/null; then
    echo "📝 GitHub CLI detected. Creating PR..."
    echo ""
    
    # Create PR using gh CLI
    gh pr create \
        --base main \
        --title "feat: Permissioned Creation & Tiered Market Levels (Issue #14)" \
        --body-file PR_TEMPLATE_ISSUE_14.md \
        --label "enhancement" \
        --label "breaking-change"
    
    echo ""
    echo "✅ Pull Request created successfully!"
else
    echo "ℹ️  GitHub CLI not found. Please create PR manually:"
    echo ""
    echo "1. Visit your repository on GitHub"
    echo "2. Click 'Compare & pull request' button"
    echo "3. Set base branch to: main (or develop if it exists)"
    echo "4. Copy content from PR_TEMPLATE_ISSUE_14.md into PR description"
    echo "5. Add labels: enhancement, breaking-change"
    echo ""
    echo "Or install GitHub CLI: https://cli.github.com/"
fi

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📚 Documentation files available:"
echo "   • IMPLEMENTATION_ISSUE_14.md"
echo "   • PR_TEMPLATE_ISSUE_14.md"
echo "   • QUICK_REFERENCE_ISSUE_14.md"
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo ""
echo "✨ Done!"
