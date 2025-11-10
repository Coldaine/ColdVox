#!/bin/bash
#
# Test Runner for ColdVox Text Injection Backends
#
# This script automates the process of building and running the real-world
# text injection tests for the `coldvox-text-injection` crate. It allows
# you to selectively test different backends.
#
# Usage:
#   ./run_tests.sh [backend]
#
# Examples:
#   ./run_tests.sh          # Run all available backend tests
#   ./run_tests.sh atspi    # Run only the AT-SPI backend tests
#   ./run_tests.sh ydotool  # Run only the ydotool backend tests
#

set -euo pipefail

# --- Configuration ---

# The crate to test
CRATE_NAME="coldvox-text-injection"

# --- Helper Functions ---

# Print a message in blue
info() {
  echo -e "\033[1;34m[INFO]\033[0m $1"
}

# Print a message in yellow
warn() {
  echo -e "\033[1;33m[WARN]\033[0m $1"
}

# Print a message in green
success() {
  echo -e "\033[1;32m[SUCCESS]\033[0m $1"
}

# Print an error message and exit
fatal() {
  echo -e "\033[1;31m[FATAL]\033[0m $1" >&2
  exit 1
}

# --- Main Logic ---

# Ensure the script is run from the repository root
if [ ! -f "Cargo.toml" ]; then
  fatal "This script must be run from the repository root."
fi

# Determine which tests to run
TEST_TARGET="${1:-all}"

# Build the tests with the required features
info "Building tests for '$CRATE_NAME' with 'real-injection-tests' feature..."
cargo build --package "$CRATE_NAME" --features real-injection-tests --tests

# Run the tests
case "$TEST_TARGET" in
  all)
    info "Running all backend tests..."
    cargo test --package "$CRATE_NAME" --features real-injection-tests -- --nocapture
    ;;
  atspi)
    info "Running AT-SPI backend tests..."
    cargo test --package "$CRATE_NAME" --features real-injection-tests -- test_atspi --nocapture
    ;;
  ydotool)
    info "Running ydotool backend tests..."
    cargo test --package "$CRATE_NAME" --features real-injection-tests -- test_ydotool --nocapture
    ;;
  kdotool)
    info "Running kdotool backend tests..."
    cargo test --package "$CRATE_NAME" --features real-injection-tests -- test_kdotool --nocapture
    ;;
  clipboard)
    info "Running clipboard backend tests..."
    cargo test --package "$CRATE_NAME" --features real-injection-tests -- test_clipboard --nocapture
    ;;
  enigo)
    info "Running enigo backend tests..."
    cargo test --package "$CRATE_NAME" --features real-injection-tests -- test_enigo --nocapture
    ;;
  *)
    fatal "Unknown test target: '$TEST_TARGET'. Available targets: all, atspi, ydotool, kdotool, clipboard, enigo"
    ;;
esac

success "Test run completed for '$TEST_TARGET'."
