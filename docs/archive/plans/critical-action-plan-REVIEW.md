---
doc_type: plan
subsystem: general
status: active
freshness: current
preservation: reference
summary: Source of truth for currently broken vs working features
---

# Critical Action Plan

> **Created**: 2025-12-14
> **Status**: ACTIVE
> **Priority**: P0 - Documentation is actively misleading agents and developers

This document tracks critical issues where documentation claims functionality that doesn't exist or is broken. These are not "nice to haves" — they cause immediate failures for anyone following the docs.

---

## P0: Documentation Claims Broken Features Work

### Issue 1: `whisper` Feature is Non-Functional

**Docs claim** (AGENTS.md, README.md, CLAUDE.md):
```bash
cargo run --features whisper,text-injection  # With STT
```

**Reality**:
- `whisper = []` in `crates/app/Cargo.toml` — empty stub
- `whisper_plugin.rs` is commented out: `// pub mod whisper_plugin;`
- Enabling the feature only exposes `whisper_cpp.rs` which is a non-functional stub
- **Result**: Zero STT capability despite docs promising it

**Decision**: Whisper backend was intentionally removed. It should stay gone unless a pure-Rust replacement is implemented.

**Action Required**:
- [ ] Remove all references to `--features whisper` from docs
- [ ] Remove `whisper` feature from `Cargo.toml` entirely (it's misleading)
- [ ] Update README Quick Start to show moonshine as the STT option
- [ ] Update AGENTS.md feature flags section
- [ ] Update CLAUDE.md STT integration section
- [ ] Remove `whisper_plugin.rs` and `whisper_cpp.rs` dead code

**Files to update**:
- `README.md` (lines 28, 34, 55, 62-71)
- `AGENTS.md` (lines 62, 69, 107)
- `CLAUDE.md` (STT Integration section)

---

### Issue 2: `parakeet` Feature Doesn't Compile

**Docs claim** (AGENTS.md):
```
Use feature flags: whisper, parakeet, text-injection, silero
```

**Reality**:
```bash
cargo build -p coldvox-app --features parakeet
# FAILS with 6 compile errors
# - transcribe_samples() signature mismatch
# - confidence field doesn't exist on TimedToken
```

**Root cause**: `parakeet-rs = "0.2"` API changed, plugin code not updated.

**Decision**: Parakeet will be fixed, but is low priority. For now, remove from docs as "working" and mark as "planned".

**Action Required**:
- [ ] Remove parakeet from AGENTS.md "Use feature flags" list (immediate)
- [ ] Add note that parakeet is planned but not yet working
- [ ] Fix `crates/coldvox-stt/src/plugins/parakeet.rs` to match parakeet-rs 0.2 API (low priority)
- [ ] Add CI job that builds with `--features parakeet` once fixed

**Files to fix**:
- `crates/coldvox-stt/src/plugins/parakeet.rs` (low priority)

---

### Issue 3: Python Version Nightmare

**The chaos**:
- `mise.toml`: `python = "3.13"`
- `.python-version`: `3.12`
- System Python: 3.14
- PyO3 0.27: Only works with `<= 3.12`

**What is mise?** mise (formerly rtx) is a polyglot version manager — like asdf/nvm/pyenv unified. It reads `mise.toml` and installs toolchains. The problem: it's configured for 3.13 which breaks PyO3.

**Why can't UV just own everything?** It can! The `.envrc` already creates a UV-managed venv with the correct Python. The issue is:
1. `mise.toml` contradicts `.python-version`
2. Developers who run `mise install` get 3.13 which breaks the build
3. There's no enforcement that UV is the only path

**Resolution**: Make UV the single source of truth. Remove Python from mise.toml entirely.

**Action Required**:
- [ ] Remove `python = "3.13"` from `mise.toml` (let UV own Python)
- [ ] Keep `.python-version = 3.12` for UV and other tools that respect it
- [ ] Document: "All Python flows through UV. Run `uv sync` before building moonshine."
- [ ] Add CI check that moonshine builds with UV-managed Python

**Files to update**:
- `mise.toml` (remove line 11: `python = "3.13"`)
- Add to README/AGENTS.md: "Python is managed by UV only"

---

### Issue 4: requirements.txt vs pyproject.toml Confusion

**requirements.txt**: Empty ("No external dependencies currently required")
**pyproject.toml**: Has `transformers`, `torch`, `librosa`

**Result**: `uv pip install -r requirements.txt` installs nothing useful.

**Action Required**:
- [ ] Delete `requirements.txt` (it's vestigial)
- [ ] OR populate it correctly
- [ ] Document that `uv sync` is the correct command

**Files to update**:
- `requirements.txt` (delete or fix)

---

## P1: Stub Features Waste Developer Time

These features are defined in Cargo.toml but do nothing:

| Feature | Status | Action |
|---------|--------|--------|
| `whisper` | Empty stub | Remove or implement |
| `coqui` | Empty stub | Remove or implement |
| `leopard` | Empty stub | Remove or implement |
| `silero-stt` | Empty stub | Remove or implement |
| `no-stt` | Defined but doesn't gate anything | Remove or implement |

**Action Required**:
- [ ] Remove stub features from Cargo.toml
- [ ] OR add `compile_error!()` that explains they're not implemented
- [ ] Remove from all documentation

---

## P2: CI/Code Mismatch

### Golden Master installs faster-whisper but code doesn't use it

**CI does**: `pip install faster-whisper`
**Code does**: Nothing with it (whisper backend commented out)

**Action Required**:
- [ ] Remove `pip install faster-whisper` from CI
- [ ] OR re-enable whisper backend
- [ ] Clarify what golden master tests actually test

---

## What Actually Works (Verified 2025-12-14)

| Feature | Status | Notes |
|---------|--------|-------|
| Default build | ✅ Works | `cargo build -p coldvox-app` |
| Moonshine STT | ✅ Works | Requires `uv sync` first |
| Text injection | ✅ Works | Default feature |
| Silero VAD | ✅ Works | Default feature |
| Tests | ✅ Works | `cargo test -p coldvox-app` |

---

## Tracking

- [ ] All P0 issues resolved
- [ ] All P1 issues resolved
- [ ] README accurately reflects working features
- [ ] AGENTS.md accurately reflects working features
- [ ] CI verifies all documented features compile

---

*This document should be deleted once all issues are resolved and docs are accurate.*
