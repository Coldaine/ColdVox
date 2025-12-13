# ColdVox Development Commands
# Install just: https://github.com/casey/just

# Default recipe lists all available commands
default:
    @just --list

# Run local CI checks (mirrors GitHub Actions exactly)
ci:
    ./scripts/local_ci.sh

# Run local CI with Whisper enabled (uses venv + cargo features)

# Run pre-commit hooks manually
check:
    pre-commit run --all-files

# Quick development checks (format, clippy, check, security)
lint:
    cargo fmt --all
    cargo clippy --fix --all-targets --locked --allow-dirty --allow-staged -- -D warnings
    cargo check --workspace --all-targets --locked
    cargo deny check
    cargo audit

# Auto-fix linter issues where possible
fix:
    cargo fmt --all
    cargo clippy --fix --all-targets --locked --allow-dirty --allow-staged

# Run all tests
test:
    cargo test --workspace --locked

test-full:
    cargo test --workspace --locked

# Run tests with whisper feature and auto venv activation
## Whisper-specific helpers removed pending new backend

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

# Setup sccache for faster Rust builds (idempotent - safe to run multiple times)
setup-sccache:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v sccache >/dev/null 2>&1; then
        echo "✓ sccache already installed: $(command -v sccache)"
        sccache --version
    elif [[ -x "$HOME/.cargo/bin/sccache" ]]; then
        echo "✓ sccache found at ~/.cargo/bin/sccache"
        "$HOME/.cargo/bin/sccache" --version
    else
        echo "Installing sccache..."
        cargo install sccache --locked
        echo "✓ sccache installed"
    fi
    # Export for current shell (caller may need to source or re-eval)
    echo "To enable: export RUSTC_WRAPPER=sccache"

# Setup all development tools (run once after clone)
setup: setup-hooks setup-sccache
    @echo "✓ Development environment ready"

# Install Moonshine Python dependencies (transformers, torch, librosa via uv)
setup-moonshine:
    ./scripts/install-moonshine-deps.sh

# Build with Moonshine STT backend enabled
build-moonshine: setup-moonshine
    cargo build --workspace --locked --features moonshine

# Run Moonshine verification example
verify-moonshine: setup-moonshine
    cargo run -p coldvox-stt --example verify_moonshine --features moonshine

