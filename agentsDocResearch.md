# Research: Evolving AGENTS.md / CLAUDE.md for Modern LLM Agents

## Problem Statement

Documentation written for LLM agents a year ago assumed less capability. Agents needed explicit instructions for things they now know intrinsically. This creates noise that obscures actually useful project-specific information.

## What Used to Be Useful (2023-2024) But Isn't Now

### Standard Tooling
- "Run `cargo test` to run tests"
- "Use `cargo clippy` for linting"
- "Run `cargo fmt` to format code"
- "`-p <crate>` targets a specific package"

**Why obsolete:** Modern agents know standard Rust/Python/JS tooling. They've seen millions of repos.

### File Structure Narration
- "Source code lives in `src/`"
- "`main.rs` is the entry point"
- "Tests are in `tests/` or inline with `#[cfg(test)]`"

**Why obsolete:** Agents understand standard project layouts. They can `ls` and read `Cargo.toml`.

### Language Basics
- "Rust uses Cargo.toml for dependencies"
- "Python uses requirements.txt or pyproject.toml"
- "This is a workspace with multiple crates"

**Why obsolete:** Agents know language ecosystems. Workspace structure is in `Cargo.toml`.

### Generic Workflow Instructions
- "Create a branch before making changes"
- "Run tests before committing"
- "Use meaningful commit messages"

**Why obsolete:** These are universal practices, not project-specific knowledge.

## Examples From This Repo (ColdVox)

### Likely Obsolete

```markdown
# From AGENTS.md - probably unnecessary:

# Type check single crate
cargo check -p coldvox-stt

# Clippy single crate
cargo clippy -p coldvox-audio

# Test single crate
cargo test -p coldvox-text-injection

# Format check (always full, it's fast)
cargo fmt --all -- --check
```

**Why:** Any agent working in Rust knows these commands. Crate names are discoverable.

### Probably Still Useful

```markdown
# Use UV for Python, never pip
uv sync
uv run <command>

# Self-hosted runner labels
runs-on: [self-hosted, Linux, X64, fedora, nobara]
```

**Why:** These are project-specific decisions that contradict common patterns or aren't discoverable.

## What SHOULD Be In Agent Documentation

### 1. Deviations From Standard Patterns
- "We use X instead of the typical Y"
- "This looks wrong but is intentional because Z"

### 2. Hidden Requirements
- Environment variables needed
- External services required
- Platform-specific setup

### 3. Non-Obvious Architecture Decisions
- Why certain patterns were chosen
- Gotchas that would trip up someone unfamiliar

### 4. Project-Specific Workflows
- CI/CD quirks
- Release processes
- Feature flag conventions

### 5. Things That Would Waste Time Discovering
- "Don't bother with X, it's deprecated"
- "Y is the source of truth, not Z"

## Research Questions

1. **What in our current AGENTS.md/CLAUDE.md is actually useful?**
2. **What's noise that modern agents don't need?**
3. **What non-obvious things about this repo would help an agent that aren't documented?**
4. **What's the minimal effective documentation for a 2025 agent?**

---

## Agent Research Findings

*The sections below were added by agents exploring the repository:*

### Agent 1: Non-Obvious Patterns and Gotchas

#### Feature Flag Architecture
- **`whisper` feature is a placeholder** — defined but stubbed out. Actual STT: `parakeet` (GPU-only) or `moonshine` (CPU via PyO3). Agents enabling `whisper` get nothing.
- **`no-stt` feature exists but doesn't gate anything** — misleading if an agent tries to build without STT.
- **Default features include `silero` VAD but no STT backend** — transcription fails silently if agent doesn't explicitly enable `parakeet` or `moonshine`.

#### Build-Time Display Detection
- **`crates/app/build.rs` reads env vars at compile time** — sets `kde_globalaccel`, `wayland_session`, `x11_session` cfg flags.
- **Gotcha:** Rebuilding after switching X11↔Wayland requires clean rebuild. Stale artifacts have wrong session flags.
- **Intentional:** Missing display vars = no session flags set. Valid for headless CI, but text injection won't work.

#### Text Injection Backend Detection
- **Runtime detection, not just compile-time** — `BackendDetector::detect_available_backends()` checks for actual binaries.
- **Gotcha:** `kdotool` feature compiled doesn't mean it works — manager checks for `ydotool` binary at runtime.
- **`combo_clip_ydotool` is internal strategy** — clipboard paste falls back to ydotool, not exposed as standalone.

#### Audio Capture Initialization
- **`AudioCaptureThread::spawn()` initializes `running = true`** — not `false`. Comment explains: "Start in running state so device monitor thread stays alive."
- **Gotcha:** Changing to `false` breaks device monitoring without obvious errors.

#### State Machine Validation
- **`StateManager` enforces transitions:** `Initializing → Running → {Recovering, Stopping} → Stopped`.
- **Invalid transitions panic** — can't go `Running → Stopped` directly.

#### PyO3/Moonshine
- **GIL required for all `Py<PyAny>` access** — safety comments mark this. Bypassing causes data races.
- **Parakeet is pure Rust** — no Python, no GIL issues. Different mental model.

#### Threading Patterns
- **Mixed async/sync:** `tokio::spawn()` for async tasks, `thread::spawn()` for audio capture/device monitoring.
- **Dual channel patterns:** `broadcast` for fanout (VAD events to multiple listeners), `mpsc` for single-receiver (hotkey handling).

#### Watchdog Timer
- **Uses custom clock abstraction** — `WatchdogTimer::new_with_clock()` accepts `SharedClock`. Tests inject mocks.
- **Standard duration methods won't work with injected clocks.**

#### Device Monitor Tuning
- **2-second polling interval is intentional** — shorter intervals cause spurious hotplug events from CPAL enumeration glitches.

#### Ring Buffer Overflow
- **Non-blocking write returns `AudioError::BufferOverflow`** — doesn't block or drop samples. Errors propagate up.

#### Test Isolation
- **`#[serial]` required for env detection tests** — tests manipulate `std::env` which affects other tests.

---

### Agent 2: Documentation Gaps and Actual Useful Info

#### Test Categories (Four, Not Two)
1. **Unit/Integration** — run everywhere (`cargo test --workspace`)
2. **Golden Master** — requires Whisper model (`cargo test -p coldvox-app --test golden_master`)
3. **Hardware capability** — marked `#[ignore]`, opt-in via `COLDVOX_E2E_REAL_AUDIO=1`, `COLDVOX_E2E_REAL_INJECTION=1`
4. **Real injection** — backend-specific (`cargo test -p coldvox-text-injection --features real-injection-tests`)

#### Critical Undocumented Environment Variables
```
# Test control
COLDVOX_TEST_LOG_LEVEL          # default: debug
COLDVOX_TEST_TIMEOUT_SEC        # per-test timeout
COLDVOX_E2E_REAL_AUDIO=1        # opt-in hardware tests
COLDVOX_E2E_REAL_INJECTION=1    # opt-in injection tests
COLDVOX_RUN_AUDIO_IT=1          # audio integration tests

# CI/Runtime
WHISPER_MODEL_PATH              # CI sets from cache script
WHISPER_MODEL_SIZE              # tiny/base/small/medium/large
DBUS_SESSION_BUS_ADDRESS        # required for text injection tests
RUST_TEST_TIME_UNIT=10000       # milliseconds, not seconds!
```

#### PyO3 + Python Version Pinning
- **Python 3.12 required** — `.python-version` pins it. Python 3.13 breaks PyO3 0.27 (free-threading incompatibility).
- **`.cargo/config.toml` pins to `./.venv/bin/python`** — PyO3 always uses repo venv, not system Python.
- **Must run `uv sync` before `cargo build --features moonshine`** — installs transformers, torch, librosa.

#### External Dependencies (Not in Cargo)
- **ydotool daemon** — user systemd service required. `scripts/setup_text_injection.sh` generates it.
- **faster-whisper** — Python package, not Rust. Install via `pip` or `uv sync`.
- **AT-SPI2 libraries** — `libatspi2.0-dev` for text injection on Linux.

#### Config File Hierarchy
- **`config/plugins.json` is source of truth** — legacy `./plugins.json` ignored with warning.
- **`COLDVOX_CONFIG_PATH`** overrides discovery. `COLDVOX_SKIP_CONFIG_DISCOVERY` disables it.

#### CI-Specific Behaviors
- **Whisper models cached at runner-specific path** — `/home/coldaine/ActionRunnerCache/whisper/`, symlinked by setup script.
- **sccache enabled only in text injection job** — different caching strategies per job.
- **Headless setup is non-trivial** — `scripts/start-headless.sh` runs Xvfb, dbus-launch, verifies clipboard tools. Tests fail without it.
- **Cleanup runs on failure** — `if: always()` kills Xvfb, fluxbox, dbus-daemon. Without it, subsequent runs hang.

#### Golden Master Approval Workflow
- **First-time tests FAIL by design** — output says `cp *.received.json *.approved.json`. Not a bug.
- **Artifacts in `crates/app/tests/golden_master_artifacts/`**

#### Logging Specifics
- **Test logging goes to files** — `target/test-logs/<test_name>.log`, not console. TUI tests corrupt terminal.
- **Default log level is INFO** (changed from DEBUG). Use `RUST_LOG=debug` for audio processing details.
- **Audio frame dispatch is TRACE** — ~60 frames/sec, extremely noisy.

#### Common Agent Mistakes
1. Assuming `cargo test` works headless — text injection needs DISPLAY
2. Building Moonshine without `uv sync` — PyO3 fails mysteriously
3. Skipping `--include-ignored` tests — misses hardware issues
4. Golden Master failures = regression — no, it's approval workflow
5. Setting timeouts in seconds — `RUST_TEST_TIME_UNIT` is milliseconds
6. Not clearing CI env vars in tests — `CI=true` affects behavior

---

## Appendix: Agent Prompts Used

### Agent 1 Prompt (Non-Obvious Patterns)

```
You are researching what non-obvious information would be useful for an AI agent working on this codebase.

Explore the repository looking for:
1. Non-standard patterns or configurations
2. Hidden gotchas that aren't documented
3. Things that look wrong but are intentional
4. Environment variables or setup requirements that aren't obvious
5. Feature flags and when they matter
6. Integration points between crates that aren't self-evident
7. Platform-specific code or requirements
8. Deprecated code paths that still exist

Focus on things that would WASTE TIME if an agent had to discover them through trial and error.

DO NOT list obvious things like "it's a Rust workspace" or "use cargo to build."

After your research, append your findings to `/home/coldaine/_projects/ColdVox/agentsDocResearch.md` under a new section called "### Agent 1: Non-Obvious Patterns and Gotchas"

Format as bullet points, each with a brief explanation of WHY this is non-obvious and worth documenting.
```

### Agent 2 Prompt (Documentation Gaps)

```
You are auditing this repository to find information gaps that would help AI agents.

Explore the repository looking for:
1. Build-time detection or conditional compilation that affects behavior
2. Test categories or test requirements (env vars, hardware, etc.)
3. CI/CD specific behaviors or workarounds
4. Python/Rust interop points (PyO3, feature flags)
5. External dependencies that must be installed outside cargo
6. Configuration files that affect runtime behavior
7. Logging/debugging patterns specific to this project
8. Things in CLAUDE.md or AGENTS.md that are ACTUALLY useful vs noise

Focus on things a new agent would struggle with or get wrong on first attempt.

DO NOT include standard Rust knowledge or commands any agent would know.

After your research, append your findings to `/home/coldaine/_projects/ColdVox/agentsDocResearch.md` under a new section called "### Agent 2: Documentation Gaps and Actual Useful Info"

Format as bullet points, explaining what's non-obvious and why it matters.
```

---

## Agent 3: Genuine Confusion Points (Discovered Cold)

*The sections above were generated by agents explicitly told what to look for. This section documents what an agent ACTUALLY gets wrong when exploring the codebase without guidance.*

### Critical: Documentation Actively Misleads

#### 1. `whisper` Feature Is A Lie

**AGENTS.md says:** `cargo run --features whisper,text-injection  # With STT`

**Reality:**
- `whisper = []` in Cargo.toml — it's an empty stub
- The `whisper_plugin.rs` that had `faster-whisper-rs` is commented out: `// pub mod whisper_plugin;`
- Enabling this feature gives you `whisper_cpp.rs` which is a non-functional stub
- **An agent following AGENTS.md instructions gets zero STT capability**

#### 2. `parakeet` Feature Doesn't Compile

**AGENTS.md says:** `Use feature flags: whisper, parakeet, text-injection, silero`

**Reality:**
```
cargo build -p coldvox-app --features parakeet
# FAILS with 6 compile errors
# API mismatch with parakeet-rs crate
```
- The parakeet plugin code is out of sync with the `parakeet-rs = "0.2"` dependency
- `transcribe_samples()` signature wrong, `confidence` field doesn't exist
- **Agent cannot enable GPU STT despite documentation suggesting it works**

#### 3. Python Version Contradiction

**mise.toml says:** `python = "3.13"`
**.python-version says:** `3.12`
**Previous agent research says:** "Python 3.13 breaks PyO3 0.27"

**Reality:**
- System Python: 3.14
- Venv Python: 3.12
- Which one you get depends on whether you use mise, direnv, or neither
- **Only 3.12 actually works for moonshine builds**

#### 4. requirements.txt vs pyproject.toml

**requirements.txt:** Empty (says "No external dependencies currently required")
**pyproject.toml:** Has `transformers>=4.35.0`, `torch>=2.0.0`, `librosa>=0.10.0`

**Reality:**
- `requirements.txt` is vestigial — tells you to install nothing
- `pyproject.toml` has actual deps needed for moonshine
- An agent following `uv pip install -r requirements.txt` installs nothing useful

### What Actually Works (Verified)

1. **Default build works:** `cargo build -p coldvox-app` ✓
2. **Moonshine builds:** `cargo build -p coldvox-stt --features moonshine` ✓ (after `uv sync`)
3. **Default tests pass:** `cargo test -p coldvox-app` ✓
4. **Text injection compiles:** Default features include it

### What Doesn't Work (Despite Docs)

1. **Whisper feature:** Stub — no functionality
2. **Parakeet feature:** Compile errors
3. **Coqui/Leopard/Silero-STT features:** Empty stubs (`coqui = []`, etc.)
4. **Golden master tests on hosted CI:** Require `pip install faster-whisper` but the Rust code doesn't use it

### Stub Features That Waste Agent Time

These features are defined but do nothing:
- `whisper = []` — placeholder
- `coqui = []` — placeholder
- `leopard = []` — placeholder
- `silero-stt = []` — placeholder
- `no-stt = []` — defined but doesn't gate anything

### Genuine Gotchas Not In Prior Research

1. **CI installs `faster-whisper` but code doesn't use it** — The golden master job does `pip install faster-whisper` but the Rust whisper backend is commented out. This is CI cruft that confuses understanding.

2. **venv is required before moonshine build** — `.cargo/config.toml` sets `PYO3_PYTHON = ./.venv/bin/python`. If venv doesn't exist, PyO3 build fails with cryptic errors.

3. **justfile has dead whisper commands** — Line 38 says `## Whisper-specific helpers removed pending new backend` but AGENTS.md still references whisper as if it works.

4. **Build detection happens at compile time** — `build.rs` reads `WAYLAND_DISPLAY`/`DISPLAY` at compile time. Binaries built in CI without these vars have different cfg flags than local builds.

### Recommended Documentation Changes

**Remove from AGENTS.md:**
- `cargo run --features whisper,text-injection  # With STT` — whisper doesn't work
- References to parakeet as a working feature

**Add to AGENTS.md:**
- "Only `moonshine` STT backend currently works. Requires `uv sync` first."
- "Default build has NO STT — only VAD. This is intentional for the audio pipeline tests."

**Fix:**
- Either make parakeet compile or remove it from feature list
- Either implement whisper or remove documentation claims

