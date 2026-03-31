# Build Commands Reference

## File-Scoped Commands (Preferred)

```bash
cargo check -p coldvox-stt
cargo clippy -p coldvox-audio
cargo test -p coldvox-text-injection
cargo fmt --all -- --check
```

## Workspace Commands

```bash
./scripts/local_ci.sh
cargo clippy --workspace --all-targets --locked
cargo test --workspace --locked
cargo build --workspace --locked
```

## Run Commands

```bash
cargo run -p coldvox-app --bin coldvox
cargo run -p coldvox-app --bin tui_dashboard
cargo run --features text-injection,moonshine
```
