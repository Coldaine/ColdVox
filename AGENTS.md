# AGENTS.md

Canonical agent entrypoint for ColdVox.

## Read First

1. [README.md](/D:/_projects/ColdVox/README.md)
2. [docs/reference/crates/app.md](/D:/_projects/ColdVox/docs/reference/crates/app.md)
3. [docs/domains/foundation/fdn-testing-guide.md](/D:/_projects/ColdVox/docs/domains/foundation/fdn-testing-guide.md)
4. [docs/logging.md](/D:/_projects/ColdVox/docs/logging.md)

For substantial work, read the relevant crate reference under [docs/reference/crates](/D:/_projects/ColdVox/docs/reference/crates) and the matching domain docs under [docs/domains](/D:/_projects/ColdVox/docs/domains).

## Precedence

When guidance conflicts, use this order:

1. Code and tests
2. The task-specific crate/domain docs
3. [README.md](/D:/_projects/ColdVox/README.md)
4. This file

[docs/architecture.md](/D:/_projects/ColdVox/docs/architecture.md) is useful for long-range context, but it is not the source of truth for current runtime behavior.

## Repo Truths

- ColdVox is a Rust workspace for audio capture, VAD, STT routing, and text injection.
- Windows is the priority environment.
- `config/default.toml` is the checked-in startup config and currently defaults STT to `mock`.
- `config/plugins.json` is plugin-manager persistence, not the primary startup config.
- `crates/coldvox-gui` exists, but the GUI is still a stub/prototype path.
- The canonical command surface lives in the root [justfile](/D:/_projects/ColdVox/justfile).
- Prefer git worktrees for parallel work under `../.trees/coldvox-{branch-name}`.

## Working Rules

- Prefer crate-scoped Rust commands for iteration; use workspace-wide commands only when needed.
- Validate changes before claiming success. Start with the smallest relevant check, then widen only as needed.
- Keep documentation changes thin and link-heavy; do not turn root agent docs into a second README.
- Update [CHANGELOG.md](/D:/_projects/ColdVox/CHANGELOG.md) only for user-visible changes, following [docs/standards.md](/D:/_projects/ColdVox/docs/standards.md).

## Ask First

- Force pushes, rebases that rewrite shared history, or branch deletion
- Dependency changes
- Destructive file cleanup outside the immediate task
- Infra, release, or governance changes

## Useful Routes

- Runtime/config behavior: [docs/reference/crates/app.md](/D:/_projects/ColdVox/docs/reference/crates/app.md)
- Testing and current test reality: [docs/domains/foundation/fdn-testing-guide.md](/D:/_projects/ColdVox/docs/domains/foundation/fdn-testing-guide.md)
- Logging and runtime artifacts: [docs/logging.md](/D:/_projects/ColdVox/docs/logging.md)
- Documentation policy: [docs/standards.md](/D:/_projects/ColdVox/docs/standards.md)
- Active documentation backlog: [docs/todo.md](/D:/_projects/ColdVox/docs/todo.md)
- Longer-term plans and research: [docs/plans](/D:/_projects/ColdVox/docs/plans) and [docs/research](/D:/_projects/ColdVox/docs/research)
