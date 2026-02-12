---
doc_type: history
subsystem: documentation
status: archived
freshness: historical
preservation: permanent
last_reviewed: 2026-02-12
owners: Coldaine
version: 1.0.0
---

# Agent Documentation Research Findings (2025-12-25)

## Context

Research into what AI agents actually need in documentation vs. what's noise. Modern LLM agents (2025+) have vastly more capability than agents from 2023-2024, making much old documentation obsolete.

## Key Insight: Documentation That Ages Out

### What Modern Agents Already Know (Don't Document)

**Standard tooling**:
- ❌ "Run `cargo test` to run tests"
- ❌ "Use `cargo clippy` for linting"
- ❌ "`-p <crate>` targets a specific package"

Modern agents have seen millions of Rust repos. They know standard workflows.

**Language basics**:
- ❌ "Rust uses Cargo.toml for dependencies"
- ❌ "This is a workspace with multiple crates"
- ❌ "Source code lives in `src/`"

Agents understand language ecosystems and standard project layouts.

**Generic workflow advice**:
- ❌ "Create a branch before making changes"
- ❌ "Run tests before committing"

Universal practices, not project-specific knowledge.

### What Agents Actually Need (Do Document)

**1. Deviations from standard patterns**
- ✅ "We use UV instead of pip/pipenv"
- ✅ "This looks wrong but is intentional because..."

**2. Hidden requirements**
- ✅ Environment variables needed
- ✅ External services/binaries required
- ✅ Platform-specific setup

**3. Non-obvious architecture decisions**
- ✅ Why certain patterns were chosen
- ✅ Gotchas that would waste time discovering

**4. Project-specific workflows**
- ✅ CI/CD quirks
- ✅ Feature flag meanings
- ✅ Self-hosted runner labels

**5. Things that would waste time**
- ✅ "Don't bother with X, it's deprecated"
- ✅ "Y is the source of truth, not Z"

## Genuine Confusion Points Found in ColdVox

### Critical: Documentation Actively Misleads

**Issue 1: `whisper` Feature Is A Lie**

AGENTS.md says: `cargo run --features whisper,text-injection  # With STT`

Reality:
- `whisper = []` is an empty stub in Cargo.toml
- `whisper_plugin.rs` that used `faster-whisper-rs` is commented out
- Enabling this feature gives non-functional stub code
- **Agent following docs gets zero STT capability**

**Issue 2: `parakeet` Feature Doesn't Compile**

AGENTS.md lists parakeet as a working feature.

Reality:
```bash
cargo build -p coldvox-app --features parakeet
# FAILS with 6 compile errors
# API mismatch with parakeet-rs crate
```

**Issue 3: Python Version Contradictions**

- mise.toml: `python = "3.13"`
- .python-version: `3.12`
- Actual requirement: 3.12 (PyO3 0.27 breaks on 3.13)

Agents get different Python depending on tool (mise vs direnv vs neither).

**Issue 4: Empty requirements.txt**

- requirements.txt: "No external dependencies currently required"
- pyproject.toml: Has `transformers>=4.35.0`, `torch>=2.0.0`, `librosa>=0.10.0`

Agent following `uv pip install -r requirements.txt` installs nothing useful.

### What Actually Works (Verified 2025-12-25)

✅ Default build: `cargo build -p coldvox-app`
✅ Moonshine: `cargo build -p coldvox-stt --features moonshine` (after `uv sync`)
✅ Default tests: `cargo test -p coldvox-app`
✅ Text injection: Included in default features

### What Doesn't Work (Despite Docs)

❌ Whisper feature - stub with no functionality
❌ Parakeet feature - compile errors
❌ Coqui/Leopard/Silero-STT - empty stubs
❌ Golden master tests on CI - pip installs faster-whisper but Rust code doesn't use it

### Stub Features That Waste Agent Time

These features are defined but do nothing:
- `whisper = []` - placeholder
- `coqui = []` - placeholder
- `leopard = []` - placeholder
- `silero-stt = []` - placeholder
- `no-stt = []` - defined but doesn't gate anything

## Non-Obvious Patterns Worth Documenting

### Build-Time Environment Detection

`crates/app/build.rs` reads environment variables **at compile time**:
- Sets `kde_globalaccel`, `wayland_session`, `x11_session` cfg flags
- **Gotcha**: Switching X11↔Wayland requires clean rebuild
- Binaries built in CI without DISPLAY have different cfg than local builds

### PyO3/Moonshine Requirements

- Python 3.12 required (3.13 breaks PyO3 0.27)
- `.cargo/config.toml` pins to `./.venv/bin/python`
- **Must run `uv sync` before `cargo build --features moonshine`**
- Without venv, PyO3 build fails with cryptic errors

### Text Injection Runtime Detection

- Compile-time features ≠ runtime availability
- `BackendDetector::detect_available_backends()` checks for actual binaries
- `kdotool` feature compiled doesn't mean it works - needs ydotool binary installed

### Test Categories (Four, Not Two)

1. **Unit/Integration** - run everywhere
2. **Golden Master** - requires Whisper model
3. **Hardware capability** - `#[ignore]`, opt-in via `COLDVOX_E2E_REAL_AUDIO=1`
4. **Real injection** - backend-specific `--features real-injection-tests`

### Critical Environment Variables (Undocumented)

```bash
# Test control
COLDVOX_E2E_REAL_AUDIO=1        # opt-in hardware tests
COLDVOX_E2E_REAL_INJECTION=1    # opt-in injection tests
COLDVOX_RUN_AUDIO_IT=1          # audio integration tests

# CI/Runtime
WHISPER_MODEL_PATH              # CI sets from cache
DBUS_SESSION_BUS_ADDRESS        # required for text injection tests
RUST_TEST_TIME_UNIT=10000       # milliseconds, not seconds!
```

### External Dependencies Not in Cargo

- **ydotool daemon** - user systemd service, `scripts/setup_text_injection.sh` generates it
- **faster-whisper** - Python package, not Rust
- **AT-SPI2 libraries** - `libatspi2.0-dev` for text injection

### Config File Hierarchy

- `config/plugins.json` is source of truth
- Legacy `./plugins.json` ignored with warning
- `COLDVOX_CONFIG_PATH` overrides discovery

### Golden Master Approval Workflow

- **First-time tests FAIL by design** - says `cp *.received.json *.approved.json`
- Not a bug - it's the approval workflow
- Artifacts in `crates/app/tests/golden_master_artifacts/`

## Recommendations for Agent Documentation

### Remove from AGENTS.md

- References to whisper as working feature
- Claims about parakeet compiling
- Standard Rust commands agents already know

### Add to AGENTS.md

- "Only `moonshine` STT backend currently works. Requires `uv sync` first."
- "Default build has NO STT - only VAD. Intentional for audio pipeline tests."
- Environment variables that control test behavior
- Golden master approval workflow explanation

### Fix in Codebase

- Either make parakeet compile or remove from feature docs
- Either implement whisper or remove claims
- Consolidate Python version to single source of truth
- Make requirements.txt match pyproject.toml or delete it

## Lessons Learned

1. **Documentation staleness is insidious** - Features get stubbed out, docs remain unchanged

2. **Stub features are worse than missing features** - Agent wastes time trying something that's explicitly listed but doesn't work

3. **Modern agents don't need basic instructions** - Assume 2025 agents know standard tooling and ecosystems

4. **Document the gotchas, not the happy path** - Agents figure out the normal flow; they need warnings about edge cases

5. **One source of truth** - Python version in 3 places with 3 different values is worse than no documentation

## Agent Prompts Used

Documented in original research file. Key insight: Agents told to "find non-obvious patterns" produced useful results. Agents told to "document the codebase" regurgitated known information.

## References

- Research session: 2025-12-25
- Branch: main (documentation review)
- Related: docs/plans/critical-action-plan.md tracks Parakeet integration work
