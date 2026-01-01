# ColdVox

> ⚠️ **Internal Alpha** - This project is in early development and not ready for production use.

> **⚠️ CRITICAL**: Documentation and feature status changes quickly. See [`docs/plans/critical-action-plan.md`](docs/plans/critical-action-plan.md) for what currently works.

Minimal root README. Full developer & architecture guide: see [`CLAUDE.md`](CLAUDE.md). Assistants should read [`AGENTS.md`](AGENTS.md).

## Quick Start

Status varies by STT backend and platform. For current “what works” details, see [`docs/plans/critical-action-plan.md`](docs/plans/critical-action-plan.md).

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

This project uses a git hook standard powered by **[mise](https://mise.jdx.dev)** and **lint-staged**.

1. Install mise: `curl https://mise.run | sh` (or see [docs](https://mise.jdx.dev/getting-started.html))
2. Install toolchain: `mise install`
3. Activate hooks + agent mirrors: `mise run prepare`

To run the hook pipeline manually:

```bash
mise run pre-commit
```

## Contributing

- Review the [Master Documentation Playbook](docs/MasterDocumentationPlaybook.md).
- Follow the repository [Documentation Standards](docs/standards.md).
- Coordinate work through the [Documentation Todo Backlog](docs/todo.md).
