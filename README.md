# ColdVox

> ⚠️ **Internal Alpha** - This project is in early development and not ready for production use.

> **⚠️ CRITICAL**: Documentation and feature status changes quickly. See [`docs/plans/critical-action-plan.md`](docs/plans/critical-action-plan.md) for what currently works.

Minimal root README. Full developer & architecture guide: see [`CLAUDE.md`](CLAUDE.md). Assistants should read [`AGENTS.md`](AGENTS.md).

## North Star

Current product and documentation direction is anchored in:

- [`docs/northstar.md`](docs/northstar.md)
- [`docs/anchor-2026-02-09.md`](docs/anchor-2026-02-09.md)
- [`docs/architecture.md`](docs/architecture.md)

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

This project uses repo-tracked git hooks (stored in `.githooks/`) to automate validation tasks, including domain documentation naming checks and AI agent instruction mirror syncing.

1. **Automatic Setup**: If using **[mise](https://mise.jdx.dev)**, run:
   ```bash
   mise run prepare
   ```

2. **Manual Setup**: If not using mise, install the hooks directly:
   ```bash
   ./scripts/install-githooks.sh
   ```

#### Active Hooks
- **pre-commit**: Runs `lint-staged` for formatting and linting.
- **pre-push**: Runs `scripts/validate_domain_docs_naming.py` to ensure documentation standards are met before pushing to the server.

To run the pre-commit hook pipeline manually:
```bash
mise run pre-commit
# OR
pre-commit run --all-files
```

## Contributing

- Review the [Master Documentation Playbook](docs/MasterDocumentationPlaybook.md).
- Follow the repository [Documentation Standards](docs/standards.md).
- Coordinate work through the [Documentation Todo Backlog](docs/todo.md).
