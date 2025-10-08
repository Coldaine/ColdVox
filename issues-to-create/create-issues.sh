#!/bin/bash
# Script to create GitHub issues from templates
# Usage: ./create-issues.sh [dry-run]

set -euo pipefail

REPO="Coldaine/ColdVox"
DRY_RUN="${1:-}"

# Function to extract title from frontmatter
get_title() {
    local file="$1"
    grep '^title:' "$file" | sed 's/title: "\(.*\)"/\1/'
}

# Function to extract labels from frontmatter
get_labels() {
    local file="$1"
    grep '^labels:' "$file" | sed 's/labels: \[\(.*\)\]/\1/' | tr -d '"' | tr ',' '\n' | xargs
}

# Function to create body (skip frontmatter)
get_body() {
    local file="$1"
    sed -n '/^---$/,/^---$/!p' "$file" | sed '1,/^---$/d'
}

# Function to create a single issue
create_issue() {
    local file="$1"
    local title=$(get_title "$file")
    local labels=$(get_labels "$file" | tr '\n' ',' | sed 's/,$//')
    local body=$(get_body "$file")
    
    echo "Creating issue: $title"
    echo "  File: $file"
    echo "  Labels: $labels"
    
    if [ "$DRY_RUN" = "dry-run" ]; then
        echo "  [DRY RUN] Would create issue with:"
        echo "    Title: $title"
        echo "    Labels: $labels"
        echo ""
        return 0
    fi
    
    # Create the issue
    if command -v gh &> /dev/null; then
        echo "$body" | gh issue create \
            --repo "$REPO" \
            --title "$title" \
            --label "$labels" \
            --body-file -
        echo "  ✓ Created"
    else
        echo "  ✗ Error: GitHub CLI (gh) not found"
        echo "  Install from: https://cli.github.com/"
        exit 1
    fi
    echo ""
}

# Main script
main() {
    echo "=================================="
    echo "GitHub Issue Creator"
    echo "=================================="
    echo "Repository: $REPO"
    
    if [ "$DRY_RUN" = "dry-run" ]; then
        echo "Mode: DRY RUN (no issues will be created)"
    else
        echo "Mode: LIVE (issues will be created)"
        echo ""
        read -p "Are you sure you want to create 20 issues? (yes/no): " confirm
        if [ "$confirm" != "yes" ]; then
            echo "Aborted."
            exit 0
        fi
    fi
    echo ""
    
    # Check if gh is authenticated
    if [ "$DRY_RUN" != "dry-run" ]; then
        if ! gh auth status &> /dev/null; then
            echo "Error: GitHub CLI is not authenticated"
            echo "Run: gh auth login"
            exit 1
        fi
    fi
    
    # Create issues in priority order
    echo "Creating P0 issues (Correctness & Reliability)..."
    for file in P0-*.md; do
        [ -f "$file" ] || continue
        create_issue "$file"
    done
    
    echo "Creating P1 issues (Performance & Maintainability)..."
    for file in P1-*.md; do
        [ -f "$file" ] || continue
        create_issue "$file"
    done
    
    echo "Creating P2 issues (Structure, Testing & Documentation)..."
    for file in P2-*.md; do
        [ -f "$file" ] || continue
        create_issue "$file"
    done
    
    echo "=================================="
    echo "Summary"
    echo "=================================="
    echo "P0 issues: $(ls P0-*.md 2>/dev/null | wc -l)"
    echo "P1 issues: $(ls P1-*.md 2>/dev/null | wc -l)"
    echo "P2 issues: $(ls P2-*.md 2>/dev/null | wc -l)"
    echo "Total: $(ls P[012]-*.md 2>/dev/null | wc -l)"
    
    if [ "$DRY_RUN" = "dry-run" ]; then
        echo ""
        echo "This was a dry run. To create issues for real, run:"
        echo "  ./create-issues.sh"
    else
        echo ""
        echo "✓ All issues created successfully!"
        echo ""
        echo "View issues at: https://github.com/$REPO/issues"
    fi
}

# Run main function
main
