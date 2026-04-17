# ColdVox Development Commands
# Install just: https://github.com/casey/just

set shell := ["C:/Program Files/PowerShell/7/pwsh.exe", "-NoLogo", "-NoProfile", "-Command"]

# Default recipe lists all available commands
default:
    @just --list

# Run local CI checks (mirrors GitHub Actions exactly)
ci:
    bash ./scripts/local_ci.sh

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

# Install pre-commit hooks
setup-hooks:
    pre-commit install

# Skip Rust checks in pre-commit (useful for quick commits)
commit-fast *args:
    SKIP_RUST_CHECKS=1 git commit {{args}}

# Run specific test by name
test-filter filter:
    cargo test --workspace --locked {{filter}}

# Windows entrypoints for local run validation
windows-run-preflight:
    pwsh -NoProfile -File scripts/windows_live_validate.ps1 -Mode Preflight

windows-smoke:
    pwsh -NoProfile -File scripts/windows_live_validate.ps1 -Mode Smoke

windows-live:
    pwsh -NoProfile -File scripts/windows_live_validate.ps1 -Mode Live

# Run main app with the canonical wave-1 HTTP remote profile
run:
    #!/usr/bin/env pwsh
    if ($IsWindows) {
        $base = uv run python -c "import sys; print(sys.base_prefix)"
        $env:PATH = "$base;$env:PATH"
    }
    cargo run -p coldvox-app --bin coldvox --features http-remote,text-injection

# Run TUI dashboard with the canonical wave-1 HTTP remote profile
tui:
    #!/usr/bin/env pwsh
    if ($IsWindows) {
        $base = uv run python -c "import sys; print(sys.base_prefix)"
        $env:PATH = "$base;$env:PATH"
    }
    cargo run -p coldvox-app --bin tui_dashboard --features http-remote,text-injection

# Run mic probe utility
mic-probe duration="30":
    cd crates/app && cargo run --bin mic_probe -- --duration {{duration}}
