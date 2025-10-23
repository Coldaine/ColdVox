# ColdVox Development Commands
# Install just: https://github.com/casey/just

# Default recipe lists all available commands
default:
    @just --list

# Run local CI checks (mirrors GitHub Actions exactly)
ci:
    ./scripts/local_ci.sh

# Run pre-commit hooks manually
check:
    pre-commit run --all-files

# Quick development checks (format, clippy, check)
lint:
    cargo fmt --all
    cargo clippy --all-targets --locked -- -D warnings
    cargo check --workspace --all-targets --locked

# Run all tests
test:
    cargo test --workspace --locked

# Run tests with Whisper model if available
test-full:
    #!/usr/bin/env bash
    if [[ -d "models/whisper-base.en" ]]; then
        export WHISPER_MODEL_PATH="models/whisper-base.en"
        cargo test --workspace --locked
    else
        echo "Whisper model not found, running without E2E tests"
        cargo test --workspace --locked -- --skip test_end_to_end_wav_pipeline
    fi

# Build all crates
build:
    cargo build --workspace --locked

# Build release
build-release:
    cargo build --workspace --locked --release

# Clean build artifacts
clean:
    cargo clean

# Format code
fmt:
    cargo fmt --all

# Generate documentation
docs:
    cargo doc --workspace --no-deps --locked --open

# Validate documentation changes (requires uv and Python 3.12)
docs-validate base="origin/main" head="HEAD":
    uv run scripts/validate_docs.py {{base}} {{head}}

# Install pre-commit hooks
setup-hooks:
    pre-commit install

# Skip Rust checks in pre-commit (useful for quick commits)
commit-fast *args:
    SKIP_RUST_CHECKS=1 git commit {{args}}

# Run specific test by name
test-filter filter:
    cargo test --workspace --locked {{filter}}

# Run main app with default features
run:
    cd crates/app && cargo run

# Run TUI dashboard
tui:
    cd crates/app && cargo run --bin tui_dashboard

# Run mic probe utility
mic-probe duration="30":
    cd crates/app && cargo run --bin mic_probe -- --duration {{duration}}
