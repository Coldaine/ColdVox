#!/bin/bash
set -euo pipefail

# claudeZ Automated Debugging Script
# Analyzes test failures and applies fixes using Claude Code with Z.ai endpoints.
#
# Usage: ./claudeZ-analyze-fix.sh <test-log-file>
#
# Required environment variables:
#   ZAI_API_KEY - API key for Z.ai endpoints
#
# Optional environment variables:
#   CLAUDE_SETTINGS - Path to Claude settings file (default: ~/.claude/zai.json)
#   MAX_FIX_ATTEMPTS - Maximum fix attempts per run (default: 1)

readonly SCRIPT_NAME="$(basename "$0")"
readonly CLAUDE_BIN="${CLAUDE_BIN:-/home/coldaine/.claude/local/claude}"
readonly CLAUDE_SETTINGS="${CLAUDE_SETTINGS:-$HOME/.claude/zai.json}"
readonly MAX_FIX_ATTEMPTS="${MAX_FIX_ATTEMPTS:-1}"

log() {
    echo "[${SCRIPT_NAME}] $*"
}

error() {
    echo "[${SCRIPT_NAME}] ERROR: $*" >&2
}

check_prerequisites() {
    if [[ ! -f "${CLAUDE_BIN}" ]]; then
        error "Claude binary not found at ${CLAUDE_BIN}"
        exit 1
    fi

    if [[ ! -f "${CLAUDE_SETTINGS}" ]]; then
        error "Claude settings file not found at ${CLAUDE_SETTINGS}"
        exit 1
    fi

    if [[ -z "${ZAI_API_KEY:-}" ]]; then
        error "ZAI_API_KEY environment variable is not set"
        exit 1
    fi
}

extract_failure_context() {
    local test_log_file="$1"
    local context_file="$2"

    # Extract test failures and surrounding context
    {
        echo "# Test Failure Log"
        echo ""
        echo "## Failed Tests"
        grep -E "(FAILED|ERROR|error\[|panicked)" "${test_log_file}" | head -50 || true
        echo ""
        echo "## Full Log (last 200 lines)"
        tail -200 "${test_log_file}"
    } > "${context_file}"
}

run_claude_fix() {
    local context_file="$1"
    local failure_context
    failure_context=$(cat "${context_file}")

    log "Running Claude to analyze and fix failures..."

    # Use headless mode (-p) with Z.ai settings
    # Claude will directly edit files using its Edit tool
    ANTHROPIC_AUTH_TOKEN="${ZAI_API_KEY}" "${CLAUDE_BIN}" \
        --settings "${CLAUDE_SETTINGS}" \
        -p "You are analyzing a Rust test failure in the ColdVox project.

TEST FAILURE CONTEXT:
${failure_context}

INSTRUCTIONS:
1. Analyze the test failure to identify the root cause
2. Find the relevant source files that need to be fixed
3. Apply minimal, targeted fixes to make the tests pass
4. Only modify source files in crates/, not test files unless the test itself is wrong
5. Do not add new dependencies or make architectural changes

Focus on fixing the immediate issue. Be conservative with changes." \
        --allowedTools Read Glob Grep Edit \
        --verbose \
        2>&1 | tee /tmp/claudeZ-output.log

    local exit_code=${PIPESTATUS[0]}
    return ${exit_code}
}

main() {
    log "Automated debugging process initiated"

    # Parse arguments
    if [[ $# -eq 0 ]]; then
        error "Usage: $0 <test-log-file>"
        exit 1
    fi

    local test_log_file="$1"

    if [[ ! -f "${test_log_file}" ]]; then
        error "Test log file not found: ${test_log_file}"
        exit 1
    fi

    # Check prerequisites
    check_prerequisites

    # Check for failing tests
    local failing_tests
    failing_tests=$(grep -cE "(FAILED|test .* \.\.\. FAILED)" "${test_log_file}" || echo "0")

    if [[ "${failing_tests}" -eq 0 ]]; then
        log "No failing tests found in the log file. Exiting."
        exit 0
    fi

    log "Found ${failing_tests} failing test(s)"

    # Extract context for Claude
    local context_file="/tmp/claudeZ-context-$$.md"
    extract_failure_context "${test_log_file}" "${context_file}"

    # Run Claude to fix
    if run_claude_fix "${context_file}"; then
        log "Claude analysis complete"

        # Check if any files were modified
        if git diff --quiet; then
            log "No changes were made by Claude"
            exit 0
        fi

        log "Changes applied. Modified files:"
        git diff --name-only

        exit 0
    else
        error "Claude fix failed"
        exit 1
    fi

    # Cleanup
    rm -f "${context_file}"
}

main "$@"
