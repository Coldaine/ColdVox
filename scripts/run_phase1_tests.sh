#!/bin/bash

# Phase 1 Test Runner Script
# Run all Phase 1 tests with various configurations

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
APP_DIR="$PROJECT_ROOT/crates/app"

cd "$APP_DIR"

echo "==========================================="
echo "       ColdVox Phase 1 Test Suite"
echo "==========================================="
echo ""

# Color codes for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to run tests with nice output
run_test_suite() {
    local suite_name=$1
    local test_command=$2
    
    echo -e "${YELLOW}Running: $suite_name${NC}"
    echo "Command: $test_command"
    echo "-------------------------------------------"
    
    if eval "$test_command"; then
        echo -e "${GREEN}✓ $suite_name passed${NC}"
    else
        echo -e "${RED}✗ $suite_name failed${NC}"
        exit 1
    fi
    echo ""
}

# Check for test mode argument
TEST_MODE=${1:-all}

case $TEST_MODE in
    unit)
        echo "Running UNIT tests only..."
        run_test_suite "Unit Tests" "cargo test --lib"
        ;;
    
    integration)
        echo "Running INTEGRATION tests only..."
        run_test_suite "Integration Tests" "cargo test --test '*'"
        ;;
    
    live)
        echo "Running LIVE HARDWARE tests..."
        run_test_suite "Live Hardware Tests" "cargo test --features live-hardware-tests"
        ;;

    manual)
        echo "Running MANUAL tests (requires audio device)..."
        echo ""
        echo "Test 1: Audio Capture with WAV Output (10 seconds)"
        cargo run --bin mic_probe -- --test-capture
        echo ""
        echo "Test 2: Default Capture (10 seconds)"
        cargo run --bin mic_probe -- --duration 10
        echo ""
        echo "Test 3: Silence Detection (30 seconds)"
        cargo run --bin mic_probe -- --duration 30 --silence-threshold 100
        ;;
    
    coverage)
        echo "Running tests with COVERAGE..."
        # Install tarpaulin if not present
        if ! command -v cargo-tarpaulin &> /dev/null; then
            echo "Installing cargo-tarpaulin..."
            cargo install cargo-tarpaulin
        fi
        cargo tarpaulin --out Html --output-dir coverage
        echo "Coverage report generated in coverage/tarpaulin-report.html"
        ;;
    
    quick)
        echo "Running QUICK test suite (fast unit tests only)..."
        run_test_suite "Quick Unit Tests" "cargo test --lib -- --test-threads=4"
        ;;
    
    all)
        echo "Running ALL automated tests..."
        
        # Run cargo check first
        run_test_suite "Cargo Check" "cargo check --all-targets"
        
        # Run formatting check
        run_test_suite "Format Check" "cargo fmt -- --check"
        
        # Run clippy
        run_test_suite "Clippy Lints" "cargo clippy -- -D warnings"
        
        # Run unit tests
        run_test_suite "Unit Tests" "cargo test --lib"
        
        # Run doc tests
        run_test_suite "Doc Tests" "cargo test --doc"
        
        # Run integration tests (without hardware)
        run_test_suite "Integration Tests" "cargo test --test '*'"
        
        echo ""
        echo -e "${GREEN}=========================================${NC}"
        echo -e "${GREEN}    Phase 1 Automated Test Suite Complete!${NC}"
        echo -e "${GREEN}=========================================${NC}"
        ;;
    
    *)
        echo "Usage: $0 [unit|integration|live|manual|coverage|quick|all]"
        echo ""
        echo "Options:"
        echo "  unit        - Run unit tests only"
        echo "  integration - Run integration tests that do not require hardware"
        echo "  live        - Run live hardware tests (requires audio device)"
        echo "  manual      - Run manual tests with audio device"
        echo "  coverage    - Generate test coverage report"
        echo "  quick       - Run quick unit tests only"
        echo "  all         - Run all automated tests (default)"
        exit 1
        ;;
esac

echo ""
echo "Test run completed at: $(date)"