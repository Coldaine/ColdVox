#!/bin/bash
set -euo pipefail

# This script is designed to be run in a CI environment when tests fail.
# It analyzes the test failures, generates a fix using claudeZ, and applies it directly to the codebase.

main() {
    echo "Automated debugging process initiated."

    # 1. Identify the failing tests
    #    This will require parsing the test output logs from the CI environment.
    #    For now, we'll assume the log file is passed as an argument.
    if [[ $# -eq 0 ]]; then
        echo "Usage: $0 <test-log-file>"
        exit 1
    fi
    local test_log_file="$1"
    echo "Analyzing test log file: ${test_log_file}"

    # 2. Extract relevant information from the log
    #    This includes the names of the failing tests and the error messages.
    local failing_tests
    failing_tests=$(grep -oP '(?<=test ).*(?= ... FAILED)' "${test_log_file}" || true)

    if [[ -z "${failing_tests}" ]]; then
        echo "No failing tests found in the log file. Exiting."
        exit 0
    fi

    echo "Failing tests identified:"
    echo "${failing_tests}"

    # 3. For each failing test, generate a fix using claudeZ
    local claudez_tool="/home/coldaine/.claude/local/claude"
    if [[ ! -f "${claudez_tool}" ]]; then
        echo "claudeZ tool not found at ${claudez_tool}"
        exit 1
    fi

    local claudez_output
    claudez_output=$("${claudez_tool}" "fix" "--log-file" "${test_log_file}")

    echo "claudeZ output:"
    echo "${claudez_output}"

    # 4. Apply the fix to the codebase
    #    The claudeZ tool is expected to output a git diff.
    echo "Applying the fix..."
    echo "${claudez_output}" | git apply --verbose

    echo "Automated debugging process complete."
}

main "$@"
