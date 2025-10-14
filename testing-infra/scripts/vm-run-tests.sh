#!/bin/bash
set -euo pipefail

# This script is executed inside the VM to run the test suite.

# --- Environment Setup ---
export DISPLAY=:0
export RUST_LOG=info,coldvox_text_injection=debug
export RUST_BACKTRACE=1

# Set up D-Bus session for the graphical environment
eval "$(dbus-launch --sh-syntax)"

echo "--- Environment ---"
echo "User: $(whoami)"
echo "Display: $DISPLAY"
echo "DBUS_SESSION_BUS_ADDRESS: $DBUS_SESSION_BUS_ADDRESS"
echo "-------------------"

# --- Test Execution ---
echo "--- Running ColdVox Text Injection Tests ---"
LOG_FILE="/tmp/test_run.log"

# Execute the test binary, capturing output to a log file.
# The --test-threads=1 flag is important for reliable text injection tests.
if ! ./coldvox-text-injection --test --test-threads=1 --nocapture &> "${LOG_FILE}"; then
    echo "Test suite failed. See log for details."
    TEST_SUCCESS=false
else
    echo "Test suite completed successfully."
    TEST_SUCCESS=true
fi

# --- Result Collection ---
echo "--- Packaging results ---"
RESULTS_ARCHIVE="/tmp/results.tar.gz"
tar -czf "${RESULTS_ARCHIVE}" "${LOG_FILE}"

echo "--- Script finished ---"

# Exit with a non-zero status code if tests failed
if [ "$TEST_SUCCESS" = false ]; then
    exit 1
fi