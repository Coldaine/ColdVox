# Repository Guidelines

## Project Structure & Module Organization
ColdVox is a Rust workspace rooted at `Cargo.toml`; each feature lives in `crates/` (`coldvox-stt`, `coldvox-text-injection`, `coldvox-telemetry`, etc.), while the end-user CLI/TUI is in `crates/app`. Shared configs live under `config/`, automation scripts in `scripts/`, and design docs plus assistant playbooks in `docs/`. Integration fixtures and high-touch scenario tests sit in `test/`, and generated artifacts land in `target/` (ignored).

## Build, Test, and Development Commands
`just lint` runs `cargo fmt`, `cargo clippy -- -D warnings`, and `cargo check` to catch regressions fast; use it before every push. `just test` (or `cargo test --workspace --locked`) executes the full suite, while `just run` launches the default app and `just tui` starts the dashboard. Mirror GitHub Actions locally with `./scripts/local_ci.sh`, and enable git hooks via `./scripts/install-githooks.sh` to auto-format on commit.

## Coding Style & Naming Conventions
Rust code follows `rustfmt` defaults (4-space indents, trailing commas for multiline). Keep crates and modules snake_case (`coldvox_stt::pipeline`), types and traits UpperCamelCase, and constants SCREAMING_SNAKE_CASE. Features like `text-injection` or `tui` should remain hyphenated to match `Cargo.toml`. Run `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` before committing; never hand-edit generated code under `target/`.

## Testing Guidelines
Prefer colocated `mod tests` within each crate for unit coverage and reserve `test/` for cross-crate flows. Run `cargo test --workspace --locked` for default coverage, and gate microphone/STT heavy paths with `COLDVOX_SLOW_TESTS=1 cargo test -- --ignored`. Add deterministic fixtures (e.g., under `test/data/`) and document any hardware requirements in the PR. Coverage additions should integrate with the forthcoming CI job in `docs/agents.md`.

## Commit & Pull Request Guidelines
Write concise, imperative subject lines (e.g., `fix: guard AT-SPI placeholder cleanup`) and reference issues/PRs with `#NNN`. Always rebase on `main`, rerun `just lint` and `just test`, and ensure `cargo fmt`, `cargo clippy`, and `cargo test` succeed before pushing. PRs must describe the user impact, list test evidence, and note config/docs touched (`config/plugins.json`, `docs/architecture.md`, etc.). Default to rebase-merging to keep history linear.

## Security & Configuration Tips
Treat `config/plugins.json` as the single source for STT backend selection and avoid editing deprecated root-level `plugins.json`. Never commit secrets; use environment variables or `.coldvox/*.example` templates. When adding new STT backends, document feature flags plus model download steps in `README.md` and `docs/architecture.md`, and ensure cargo-deny plus `cargo audit` stay clean before requesting review.

