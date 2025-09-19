# ColdVox Development Commands
# Install just: https://github.com/casey/just

# Default recipe lists all available commands
default:
    @just --list

# One-shot environment setup (safe, no sudo)
setup:
    #!/usr/bin/env bash
    set -euo pipefail
    bash scripts/dev-setup.sh
    echo "✅ Setup complete. Tip: copy .env.example to .env and adjust as needed."

# Idempotent quick check used by other recipes
ensure-deps:
    #!/usr/bin/env bash
    set -euo pipefail
    bash scripts/ensure-deps.sh

# Heavier automation entrypoint that feels automatic (no sudo)
setup-auto:
    just ensure-deps
    cargo fetch
    pre-commit run --all-files || true
    echo "✅ Auto-setup done. For text injection (sudo), run: scripts/setup_text_injection.sh"

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

# Run tests with Vosk model if available
test-full:
    #!/usr/bin/env bash
    if [[ -d "models/vosk-model-small-en-us-0.15" ]]; then
        export VOSK_MODEL_PATH="models/vosk-model-small-en-us-0.15"
        cargo test --workspace --locked
    else
        echo "Vosk model not found, running without E2E tests"
        cargo test --workspace --locked -- --skip test_end_to_end_wav_pipeline
    fi

# Run workspace tests with nextest; autodetect local Vosk model
test-nextest:
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ -d "models/vosk-model-small-en-us-0.15" ]]; then
        export VOSK_MODEL_PATH="models/vosk-model-small-en-us-0.15"
    fi
    cargo nextest run --workspace --locked

# Start development: ensure deps then run the app with common features
dev *args:
    #!/usr/bin/env bash
    set -euo pipefail
    bash scripts/ensure-deps.sh
    cd crates/app
    cargo run --features vosk,text-injection --locked -- {{args}}

# Coverage for core crates only, with Vosk enabled; excludes GUI & Text Injection initially
test-coverage:
    #!/usr/bin/env bash
    set -euo pipefail
    export VOSK_MODEL_PATH="${VOSK_MODEL_PATH:-models/vosk-model-small-en-us-0.15}"
    mkdir -p coverage
    cargo tarpaulin 
        --locked 
        --packages coldvox-foundation coldvox-telemetry coldvox-audio coldvox-vad coldvox-vad-silero coldvox-stt coldvox-stt-vosk 
        --features vosk 
        --exclude coldvox-gui --exclude coldvox-text-injection 
        --out Html --out Lcov --output-dir coverage

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

# Run main app with default features
run:
    cd crates/app && cargo run

# Run TUI dashboard
tui:
    cd crates/app && cargo run --bin tui_dashboard

# Run mic probe utility
mic-probe duration="30":
    cd crates/app && cargo run --bin mic_probe -- --duration {{duration}}
