#!/usr/bin/env bash
# Local CI script to mirror the GitHub Actions workflow
# Run this before pushing to catch issues locally

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# shellcheck source=common_utils.sh
source "${SCRIPT_DIR}/common_utils.sh"
# shellcheck source=config.env
source "${SCRIPT_DIR}/config.env"

print_step() {
    log_step "$1"
}

print_success() {
    log_success "$1"
}

print_warning() {
    log_warn "$1"
}

print_error() {
    log_error "$1"
}

# Change to repo root
cd "$(git rev-parse --show-toplevel)"

print_step "Starting local CI checks..."

# 1. Check formatting
print_step "Checking code formatting..."
if cargo fmt --all -- --check; then
    print_success "Code formatting check passed"
else
    print_error "Code formatting check failed"
    echo "Run: cargo fmt --all"
    exit 1
fi

# 2. Run clippy
print_step "Running Clippy lints..."
if cargo clippy --all-targets --locked -- -D warnings; then
    print_success "Clippy checks passed"
else
    print_error "Clippy checks failed"
    exit 1
fi

# 3. Type check (matches CI exactly)
print_step "Running type checks..."
if cargo check --workspace --all-targets --locked; then
    print_success "Type checks passed"
else
    print_error "Type checks failed"
    exit 1
fi

# 4. Build
print_step "Building workspace..."
if cargo build --workspace --locked; then
    print_success "Build completed successfully"
else
    print_error "Build failed"
    exit 1
fi

# 5. Build documentation
print_step "Building documentation..."
if cargo doc --workspace --no-deps --locked; then
    print_success "Documentation build completed"
else
    print_error "Documentation build failed"
    exit 1
fi

# 6. Run tests (skip E2E if no Vosk model)
print_step "Running tests..."
if [[ -n "${VOSK_MODEL_PATH:-}" ]] && [[ -d "$VOSK_MODEL_PATH" ]]; then
    print_step "Running all tests (Vosk model found at $VOSK_MODEL_PATH)"
    if cargo test --workspace --locked; then
        print_success "All tests passed"
    else
        print_error "Tests failed"
        exit 1
    fi
else
    print_warning "Skipping E2E tests (VOSK_MODEL_PATH not set or directory not found)"
    if cargo test --workspace --locked -- --skip test_end_to_end_wav_pipeline; then
        print_success "Tests passed (E2E skipped)"
    else
        print_error "Tests failed"
        exit 1
    fi
fi

# 7. Check GUI build (if Qt available)
print_step "Checking GUI build..."
if command -v qmake6 >/dev/null 2>&1 || command -v qmake-qt6 >/dev/null 2>&1 || pkg-config --exists Qt6Core >/dev/null 2>&1; then
    print_step "Qt 6 detected, building GUI..."
    if cargo check -p coldvox-gui --features qt-ui --locked; then
        print_success "GUI build check passed"
    else
        print_error "GUI build check failed"
        exit 1
    fi
else
    print_warning "Qt 6 not detected, skipping GUI build"
fi

print_success "🎉 All local CI checks passed!"
log_success "Ready to push!"
