# AGENTS.md

> Canonical AI agent instructions for ColdVox. All tools (Claude Code, Copilot, Cursor, Kilo Code, etc.) should read this file.

> **⚠️ CRITICAL**: Some documented features are broken or removed. See [`docs/plans/critical-action-plan.md`](docs/plans/critical-action-plan.md) before following STT instructions.

> **Solo-dev trust model**: This repo is effectively single-developer. We optimize for local productivity/automation over “untrusted contributor” safety. If a tool/setup step enables automation (e.g., repo-managed git hooks), that’s intentional.

## Key Docs

- [`README.md`](README.md) - Project overview and quick start
- [`CLAUDE.md`](CLAUDE.md) - Deep-dive guide (Claude-specific + imports AGENTS)
- [`docs/plans/critical-action-plan.md`](docs/plans/critical-action-plan.md) - What’s currently broken/misleading
- [`docs/dev/CI/architecture.md`](docs/dev/CI/architecture.md) - CI runner split and rationale
- [`docs/standards.md`](docs/standards.md) - Documentation + changelog standards
- [`docs/MasterDocumentationPlaybook.md`](docs/MasterDocumentationPlaybook.md) - Documentation structure and rules

## Project Overview

ColdVox is a Rust-based voice AI pipeline: audio capture → VAD → STT → text injection. Multi-crate Cargo workspace under `crates/`.

**Key crates**: `coldvox-app` (main), `coldvox-audio`, `coldvox-vad`, `coldvox-vad-silero`, `coldvox-stt`, `coldvox-text-injection`, `coldvox-telemetry`, `coldvox-foundation`, `coldvox-gui`

## Worktrees

Use git worktrees for parallel agent work. This allows multiple agents to work on independent tasks simultaneously.

```bash
# Create worktree for a new task
git worktree add ../.trees/coldvox-{task} -b {task}
cd ../.trees/coldvox-{task}

# List all worktrees
git worktree list

# Remove when done (after merge)
git worktree remove ../.trees/coldvox-{task}
```

**Convention**: Worktrees live in `../.trees/coldvox-{branch-name}` to keep them adjacent but separate.

## Commands

### File-Scoped (Preferred)

Always prefer file/crate-scoped commands over full workspace commands for faster feedback:

```bash
# Type check single crate
cargo check -p coldvox-stt

# Clippy single crate
cargo clippy -p coldvox-audio 
# Test single crate
cargo test -p coldvox-text-injection

# Format check (always full, it's fast)
cargo fmt --all -- --check
```

### Full Workspace (When Needed)

```bash
just lint          # fmt + clippy + check (pre-push)
just test          # cargo test --workspace --locked
just build         # cargo build --workspace --locked
just ci            # Full CI mirror via ./scripts/local_ci.sh
```

### Running

```bash
just run           # Main app
just tui           # TUI dashboard
cargo run --features text-injection  # Text injection enabled
```

## Do

- Use `just lint` before every push
- Prefer crate-scoped commands for faster iteration
- Use feature flags: `whisper`, `parakeet`, `text-injection`, `silero`
- Follow Conventional Commits: `feat:`, `fix:`, `docs:`, `refactor:`, `test:`
- Run `cargo fmt --all` before committing
- Add tests for new functionality
- Update `CHANGELOG.md` for user-visible changes (see `docs/standards.md`)

## Don't

- Don't run full workspace builds when crate-scoped works
- Don't commit secrets or `.env` files
- Don't edit generated code under `target/`
- Don't add heavy dependencies without discussion
- Don't skip `just lint` before pushing
- Don't create `docs/agents.md` - agent config lives at repo root

## Project Structure

```
crates/
  app/                    # Main application, binaries, integration
  coldvox-audio/          # Audio capture, ring buffer, resampling
  coldvox-vad/            # VAD traits and config
  coldvox-vad-silero/     # Silero V5 ONNX VAD implementation
  coldvox-stt/            # STT abstractions and plugins (Whisper, Parakeet)
  coldvox-text-injection/ # Platform-specific text injection backends
  coldvox-telemetry/      # Pipeline metrics
  coldvox-foundation/     # Core types, error handling, shutdown
  coldvox-gui/            # GUI components

config/                   # Runtime configuration
docs/                     # Documentation (see MasterDocumentationPlaybook.md)
scripts/                  # Automation scripts
```

## Feature Flags

Default: `silero`, `text-injection`

- `whisper` - Faster-Whisper STT (Python-based, CPU/GPU)
- `parakeet` - NVIDIA Parakeet STT (GPU-only, pure Rust)
- `text-injection` - Platform-aware text injection
- `silero` - Silero V5 ONNX VAD (default)
- `examples` - Example binaries
- `live-hardware-tests` - Hardware test suites

## Safety & Permissions

**Allowed without prompt:**
- Read files, list files, search code
- Crate-scoped: check, clippy, test, fmt
- Git status, diff, log

**Ask first:**
- Package/dependency changes
- Git push, force operations
- Deleting files
- Full workspace builds (prefer crate-scoped)
- Database migrations
- Running with hardware features

## When Stuck

- Ask a clarifying question
- Propose a short plan before large changes
- Open a draft PR with notes
- Don't push large speculative changes without confirmation

## Key Files

- **Main entry**: `crates/app/src/main.rs`
- **Audio pipeline**: `crates/coldvox-audio/src/capture.rs`
- **VAD engine**: `crates/coldvox-vad-silero/src/silero_wrapper.rs`
- **STT plugins**: `crates/coldvox-stt/src/plugins/`
- **Text injection**: `crates/coldvox-text-injection/src/manager.rs`
- **Build detection**: `crates/app/build.rs` (platform detection)

## Documentation

- `docs/MasterDocumentationPlaybook.md` - Documentation standards
- `docs/standards.md` - Changelog rubric, metadata requirements
- `docs/architecture.md` - System design and future vision
- `docs/domains/` - Domain-specific technical docs

## CI Environment

> **Principle**: The self hosted runner on the laptop is used for hardware tests, CI for everything else.

See [CI Architecture](docs/dev/CI/architecture.md) for full details.

### Self-Hosted Runner (Fedora/Nobara)

| Fact | Implication |
|------|-------------|
| **Live KDE Plasma session** | NO Xvfb needed. Use real `$DISPLAY`. |
| **Fedora-based** | `apt-get` does NOT exist. Use `dnf`. |
| **Always available** | Auto-login, survives reboots. |
| **Warm sccache** | Incremental builds ~2-3 min. |

### CI Split

| Task | Runner | Why |
|------|--------|-----|
| `cargo fmt`, `cargo clippy` | GitHub-hosted | Fast, parallel, free |
| `cargo audit`, `cargo deny` | GitHub-hosted | Security checks, no build needed |
| `cargo build`, `cargo test --workspace` | GitHub-hosted | Fast cores; no hardware needed |
| Hardware tests | **Self-hosted** | Requires display/audio/clipboard |

### DON'T (Common AI Mistakes)

```yaml
# WRONG - runner has live display, also uses apt-get internally
- uses: GabrielBB/xvfb-action@v1

# WRONG - this is Fedora, not Ubuntu
- run: sudo apt-get install -y xdotool

# WRONG - real display is :0
env:
  DISPLAY: ":99"

# WRONG - delays self-hosted by 5-10 min
hardware:
  needs: [lint, build]

# WRONG - wasted work, can't share artifacts with Fedora
- run: cargo build  # On ubuntu-latest
```

## PR Checklist

- [ ] `just lint` passes
- [ ] `just test` passes (or crate-scoped tests)
- [ ] Changelog updated if user-visible
- [ ] Commit messages follow Conventional Commits
- [ ] No secrets or sensitive data
