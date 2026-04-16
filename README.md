# ColdVox

> ⚠️ **Internal Alpha** - This project is in early development and not ready for production use.

> **⚠️ CRITICAL**: Documentation and feature status changes quickly. See [`docs/plans/current-status.md`](docs/plans/current-status.md) for what currently works.

Minimal root README. Assistants should read [`AGENTS.md`](AGENTS.md).

## North Star

Current product and documentation direction is anchored in:

- [`docs/northstar.md`](docs/northstar.md)
- [`docs/plans/current-status.md`](docs/plans/current-status.md)
- [`docs/architecture.md`](docs/architecture.md)

## Quick Start

Status varies by STT backend and platform. For current "what works" details, see [`docs/plans/current-status.md`](docs/plans/current-status.md).

```bash
# Main app
cargo run -p coldvox-app --bin coldvox

# TUI dashboard
cargo run -p coldvox-app --bin tui_dashboard
```

Common Rust commands:

```bash
# Fast local feedback
cargo check -p coldvox-app

# Format check
cargo fmt --all -- --check
```

## Development

### Developer Git Hooks

Python environments are managed with `uv`. Git hooks and local tooling bootstrap are handled with **[mise](https://mise.jdx.dev)** and **lint-staged**.

1. Install mise: `curl https://mise.run | sh` (or see [docs](https://mise.jdx.dev/getting-started.html))
2. Install toolchain: `mise install`
3. Activate hooks + agent mirrors: `mise run prepare`

If you are working on Moonshine or any Python-backed STT flow, run `uv sync` before building.

To run the hook pipeline manually:

```bash
mise run pre-commit
```

## Contributing

- Review the [North Star](docs/northstar.md) and [current status](docs/plans/current-status.md).
- Follow the repository [Documentation Standards](docs/standards.md).
- Coordinate work through the [Documentation Todo Backlog](docs/todo.md).
