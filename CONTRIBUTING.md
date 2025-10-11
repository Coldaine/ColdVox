# Contributing to ColdVox

Thank you for your interest in contributing to ColdVox! This document provides guidelines and workflows for developers.

## Development Setup

### Prerequisites
- Rust toolchain (stable, minimum MSRV 1.90)
- System dependencies for your platform (see platform-specific sections below)

### Initial Setup
1. Clone the repository:
   ```bash
   git clone https://github.com/Coldaine/ColdVox.git
   cd ColdVox
   ```

2. Install dependencies:
   ```bash
   # Build the project to verify setup
   cargo build --workspace
   ```

3. Run tests:
   ```bash
   cargo test --workspace
   ```

## Code Quality & Formatting

### Pre-Commit Hook (Recommended)

To maintain consistent formatting and reduce CI failures, we provide a pre-commit hook that automatically runs `cargo fmt --all` before each commit.

**Install the hook:**
```bash
./scripts/install-githooks.sh
```

This copies the hooks from `.githooks/` into `.git/hooks/` and makes them executable. The hook will:
- Run `cargo fmt --all` automatically before each commit
- Block the commit if formatting changes are made
- Show you which files were reformatted

**To bypass the hook** (not recommended):
```bash
git commit --no-verify
```

### Manual Formatting

Run formatting manually anytime:
```bash
cargo fmt --all
```

Check if formatting is needed without making changes:
```bash
cargo fmt --all -- --check
```

### Linting

Run Clippy to catch common mistakes:
```bash
cargo clippy --all-targets --locked -- -D warnings
```

### Full Local CI Check

Run the same checks that CI will run:
```bash
./scripts/local_ci.sh
```

Or use the `just` command runner:
```bash
just ci
```

## CI Behavior

### Advisory Formatting Checks

**Important:** Formatting checks in CI are **advisory only** and will not block merges.

- If formatting differences are detected, CI will emit a **warning** but continue
- The workflow will remain green even with formatting issues
- You are encouraged to fix formatting locally before pushing, but it won't block your PR

This design allows flexibility while encouraging consistent code style.

### Running CI Locally

Use the provided scripts to mirror CI behavior locally:

```bash
# Full CI simulation
./scripts/local_ci.sh

# Or with just
just ci

# Quick checks (format, clippy, check)
just lint

# Run tests
just test
```

## Testing

### Unit Tests
```bash
cargo test --workspace --lib
```

### Integration Tests
```bash
cargo test --workspace
```

### With Vosk Model (E2E Tests)
```bash
export VOSK_MODEL_PATH="models/vosk-model-small-en-us-0.15"
cargo test --workspace
```

## Building

### Standard Build
```bash
cargo build --workspace
```

### Release Build
```bash
cargo build --workspace --release
```

### With Features
```bash
# With STT enabled
cargo build --features vosk

# With text injection
cargo build --features text-injection

# With both
cargo build --features vosk,text-injection
```

## Pull Request Guidelines

1. **Format your code**: Run `cargo fmt --all` before committing
2. **Pass Clippy**: Ensure `cargo clippy --all-targets --locked -- -D warnings` passes
3. **Run tests**: Verify `cargo test --workspace` passes
4. **Update documentation**: Add/update docs for new features or changes
5. **Write clear commit messages**: Describe what and why, not just how

## Code Style

- Follow Rust standard formatting (enforced by `rustfmt`)
- Add comments for complex logic
- Write idiomatic Rust code
- Keep functions focused and modular

## Getting Help

- Open an issue for bugs or feature requests
- Start a discussion for questions or design proposals
- Check existing issues and PRs before creating new ones

## License

By contributing to ColdVox, you agree that your contributions will be licensed under the same terms as the project (dual MIT/Apache-2.0).
