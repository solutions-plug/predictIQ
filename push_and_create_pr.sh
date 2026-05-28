#!/bin/bash
# push_and_create_pr.sh — push current branch and open a PR via GitHub CLI.
# Usage: bash push_and_create_pr.sh [--help]

set -e

usage() {
    cat <<EOF
Usage: bash push_and_create_pr.sh [--help]

Pushes the current branch to origin and creates a GitHub Pull Request.

Requirements:
  - gh CLI installed and authenticated (https://cli.github.com/)
  - Must be run from inside the git repository

Options:
  --help    Show this help message and exit
EOF
    exit 0
}

[[ "${1:-}" == "--help" ]] && usage

# Validate gh CLI is installed
if ! command -v gh &>/dev/null; then
    echo "❌ Error: 'gh' CLI is not installed."
    echo "   Install it from https://cli.github.com/ then run: gh auth login"
    exit 1
fi

# Validate gh CLI is authenticated
if ! gh auth status &>/dev/null; then
    echo "❌ Error: 'gh' CLI is not authenticated."
    echo "   Run: gh auth login"
    exit 1
fi

# Derive repo from git remote (works after forks/renames)
REPO=$(git remote get-url origin | sed -E 's|.*[:/]([^/]+/[^/]+)(\.git)?$|\1|')
CURRENT_BRANCH=$(git branch --show-current)

if [[ -z "$CURRENT_BRANCH" ]]; then
    echo "❌ Error: Could not determine current branch (detached HEAD?)."
    exit 1
fi

echo "📦 Repository : $REPO"
echo "🌿 Branch     : $CURRENT_BRANCH"
echo ""

echo "🚀 Pushing branch to origin..."
git push -u origin "$CURRENT_BRANCH"
echo ""

echo "📝 Creating Pull Request..."
gh pr create \
    --repo "$REPO" \
    --base main \
    --head "$CURRENT_BRANCH" \
    --fill

echo ""
echo "✅ Done!"
