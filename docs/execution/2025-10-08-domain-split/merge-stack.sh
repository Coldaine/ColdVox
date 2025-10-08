#!/bin/bash
# ColdVox PR Stack Merge Automation
# Usage: ./merge-stack.sh [--dry-run] [--start-from PR_NUMBER]
#
# This script automates the sequential merge of the 9-PR stack
# with automatic restacking via Graphite after each merge.

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# PR stack in merge order
declare -a PRS=(
    "123:01-config-settings"
    "124:02-audio-capture"
    "125:03-vad"
    "126:04-stt"
    "127:05-app-runtime-wav"
    "128:06-text-injection"
    "129:07-testing"
    "130:08-logging-observability"
    "131:09-docs-changelog"
)

# Parse arguments
DRY_RUN=false
START_FROM=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN=true
            shift
            ;;
        --start-from)
            START_FROM="$2"
            shift 2
            ;;
        *)
            echo "Unknown option: $1"
            echo "Usage: $0 [--dry-run] [--start-from PR_NUMBER]"
            exit 1
            ;;
    esac
done

# Logging functions
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Check prerequisites
check_prerequisites() {
    log_info "Checking prerequisites..."

    # Check for gh CLI
    if ! command -v gh &> /dev/null; then
        log_error "GitHub CLI (gh) not found. Install from https://cli.github.com/"
        exit 1
    fi

    # Check for gt (Graphite)
    if ! command -v gt &> /dev/null; then
        log_error "Graphite CLI (gt) not found. Install from https://graphite.dev/"
        exit 1
    fi

    # Check authentication
    if ! gh auth status &> /dev/null; then
        log_error "Not authenticated with GitHub. Run: gh auth login"
        exit 1
    fi

    log_success "Prerequisites check passed"
}

# Check if PR is ready to merge
check_pr_ready() {
    local pr_number=$1
    local branch_name=$2

    log_info "Checking PR #$pr_number ($branch_name)..."

    # Check CI status
    local ci_status=$(gh pr checks $pr_number --json state -q '.[0].state' 2>/dev/null || echo "UNKNOWN")

    if [[ "$ci_status" != "SUCCESS" && "$ci_status" != "SKIPPED" ]]; then
        log_warning "PR #$pr_number CI status: $ci_status (not SUCCESS)"
        return 1
    fi

    # Check review status
    local review_status=$(gh pr view $pr_number --json reviewDecision -q '.reviewDecision' 2>/dev/null || echo "NONE")

    if [[ "$review_status" != "APPROVED" ]]; then
        log_warning "PR #$pr_number review status: $review_status (not APPROVED)"
        return 1
    fi

    log_success "PR #$pr_number is ready to merge"
    return 0
}

# Special check for PR #127 (requires both #125 and #126 merged)
check_pr_127_dependencies() {
    log_info "Checking PR #127 special dependencies..."

    # Check if 03-vad is merged
    if ! git branch -r --merged origin/main | grep -q "origin/03-vad"; then
        log_error "PR #125 (03-vad) must be merged before PR #127"
        return 1
    fi

    # Check if 04-stt is merged
    if ! git branch -r --merged origin/main | grep -q "origin/04-stt"; then
        log_error "PR #126 (04-stt) must be merged before PR #127"
        return 1
    fi

    log_success "PR #127 dependencies satisfied"
    return 0
}

# Merge a single PR
merge_pr() {
    local pr_number=$1
    local branch_name=$2

    log_info "=========================================="
    log_info "Merging PR #$pr_number ($branch_name)"
    log_info "=========================================="

    # Special handling for PR #127
    if [[ "$pr_number" == "127" ]]; then
        if ! check_pr_127_dependencies; then
            log_error "Skipping PR #127 due to missing dependencies"
            return 1
        fi
    fi

    # Check if PR is ready
    if ! check_pr_ready "$pr_number" "$branch_name"; then
        log_error "PR #$pr_number is not ready to merge"
        read -p "Continue anyway? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            return 1
        fi
    fi

    if [[ "$DRY_RUN" == true ]]; then
        log_warning "[DRY RUN] Would merge PR #$pr_number"
        log_warning "[DRY RUN] Would run: gh pr merge $pr_number --squash --delete-branch"
    else
        # Merge PR
        log_info "Merging PR #$pr_number..."
        if gh pr merge "$pr_number" --squash --delete-branch; then
            log_success "PR #$pr_number merged successfully"
        else
            log_error "Failed to merge PR #$pr_number"
            return 1
        fi
    fi

    # Restack with Graphite
    log_info "Restacking downstream PRs with Graphite..."
    if [[ "$DRY_RUN" == true ]]; then
        log_warning "[DRY RUN] Would run: gt sync"
    else
        if gt sync; then
            log_success "Restack completed"
        else
            log_warning "Restack had issues, but continuing..."
        fi
    fi

    # Wait for CI on downstream PRs
    if [[ "$DRY_RUN" == false ]]; then
        log_info "Waiting 30s for downstream CI to start..."
        sleep 30
    fi

    log_success "PR #$pr_number merge complete"
    echo ""
}

# Main execution
main() {
    log_info "ColdVox PR Stack Merge Automation"
    log_info "=================================="
    echo ""

    if [[ "$DRY_RUN" == true ]]; then
        log_warning "DRY RUN MODE - No actual merges will occur"
        echo ""
    fi

    check_prerequisites
    echo ""

    # Determine starting point
    local start_index=0
    if [[ -n "$START_FROM" ]]; then
        for i in "${!PRS[@]}"; do
            local pr_info="${PRS[$i]}"
            local pr_number="${pr_info%%:*}"
            if [[ "$pr_number" == "$START_FROM" ]]; then
                start_index=$i
                log_info "Starting from PR #$START_FROM"
                break
            fi
        done
    fi

    # Merge loop
    local success_count=0
    local total_count=0

    for i in "${!PRS[@]}"; do
        # Skip PRs before start point
        if [[ $i -lt $start_index ]]; then
            continue
        fi

        local pr_info="${PRS[$i]}"
        local pr_number="${pr_info%%:*}"
        local branch_name="${pr_info#*:}"

        total_count=$((total_count + 1))

        if merge_pr "$pr_number" "$branch_name"; then
            success_count=$((success_count + 1))
        else
            log_error "Failed to merge PR #$pr_number, stopping"
            break
        fi
    done

    # Final summary
    echo ""
    log_info "=========================================="
    log_info "Merge Summary"
    log_info "=========================================="
    log_info "Successfully merged: $success_count/$total_count PRs"

    if [[ $success_count -eq ${#PRS[@]} ]]; then
        log_success "All PRs merged successfully!"
        log_info "Next steps:"
        log_info "  1. Verify: git diff main anchor/oct-06-2025"
        log_info "  2. Run: cargo test --workspace --features vosk,text-injection"
        log_info "  3. Update project status documentation"
        log_info "  4. Archive execution artifacts"
    else
        log_warning "Not all PRs merged. Resume with:"
        log_info "  ./merge-stack.sh --start-from <NEXT_PR_NUMBER>"
    fi
}

# Run main function
main
