# ColdVox State Analysis & Execution Plan

**Date**: 2026-02-28  
**Analyst**: AI Agent  
**Repository**: Coldaine/ColdVox

---

## A) Current State vs Goal

### Subsystems That Exist

| Subsystem | Status | Notes |
|-----------|--------|-------|
| **STT Pipeline** | ⚠️ Partial | Moonshine working; Parakeet broken (compile errors); Whisper stubbed |
| **Audio Pipeline** | ✅ Working | Capture, VAD (Silero), chunking all functional |
| **Text Injection** | ✅ Working | Multiple backends: xdotool, ydotool, clipboard, atspi, enigo |
| **Telemetry** | ✅ Implemented | PipelineMetrics, SttPerformanceMetrics, BasicMetrics - but lacks schema enforcement |
| **GUI** | 🔄 In Dev | Qt groundwork laid, not fully integrated |
| **TUI Dashboard** | ✅ Working | Terminal UI functional |
| **CI/CD** | ⚠️ Partial | Working but docs/hardware split needs refinement |

### Critical Brittleness Points (Source: `docs/plans/critical-action-plan.md`)

| Issue | Severity | Root Cause |
|-------|----------|------------|
| **Python Version Chaos** | P0 | `mise.toml` specifies 3.13, `.python-version` has 3.12, PyO3 only supports ≤3.12 |
| **Whisper Feature Stub** | P0 | Empty feature in Cargo.toml; docs claim it works; dead code not removed |
| **Parakeet Compile Errors** | P0 | Plugin code incompatible with parakeet-rs 0.2 API |
| **requirements.txt Empty** | P1 | Claims "no deps" but `pyproject.toml` has transformers/torch/librosa |
| **Telemetry No Schema Enforcement** | P2 | Metrics exist but no validation that naming conventions are followed |

### Verified Working Features (2025-12-14)

```bash
# Confirmed working:
cargo build -p coldvox-app                    # ✅ Default build
cargo test -p coldvox-app                     # ✅ Unit tests
cargo run -p coldvox-app --bin coldvox        # ✅ With moonshine feature
cargo run -p coldvox-app --bin tui_dashboard  # ✅ TUI works
```

---

## B) Execution Plan

### Milestone 1: Documentation & Tooling Correctness (PR #1)

**Goal**: Remove documentation/feature drift that causes immediate failures for developers following the docs.

**Deliverables**:
1. **Fix Python version configuration**
   - Remove `python = "3.13"` from `mise.toml` (UV owns Python)
   - Keep `.python-version = "3.12"` as source of truth
   - Document: "All Python flows through UV. Run `uv sync` before building moonshine."

2. **Clean up requirements.txt**
   - Delete empty/misleading `requirements.txt`
   - Add comment to `pyproject.toml` pointing to `uv sync`

3. **Remove whisper feature stub**
   - Delete `whisper = []` from `crates/app/Cargo.toml`
   - Remove dead code: `whisper_plugin.rs`, `whisper_cpp.rs`
   - Update AGENTS.md feature flags section
   - Update README Quick Start

4. **Mark parakeet as planned not working**
   - Update AGENTS.md: "parakeet is planned but not currently functional"
   - Remove from "Use feature flags" working list

5. **Add telemetry naming convention validator**
   - Script/tool to verify metrics follow `subsystem.metric_name` pattern
   - Run in CI to prevent regression

**Verification**:
```bash
./scripts/local_ci.sh      # Must pass
cargo build -p coldvox-app --features parakeet  # Should still fail (expected)
cargo build -p coldvox-app --features whisper   # Should fail (feature removed)
```

### Milestone 2: Parakeet STT Fix (PR #2)

**Goal**: Fix parakeet plugin to compile with parakeet-rs 0.2 API.

**Deliverables**:
1. Fix `transcribe_samples()` signature mismatch
2. Fix `TimedToken.confidence` field access
3. Add compile-gate CI job for parakeet feature
4. Document parakeet as "experimental" not "planned"

### Milestone 3: Telemetry Schema Enforcement (PR #3)

**Goal**: Enforce naming conventions and add verification path for metrics.

**Deliverables**:
1. Define canonical metric naming schema: `coldvox.{subsystem}.{metric_name}.{unit}`
2. Add `telemetry_schema_validator.py` script
3. CI check that fails on metric naming violations
4. Update `docs/domains/telemetry/tele-observability-playbook.md` with schema reference

### Milestone 4+ (Future)

- GUI overlay live text display
- Full CUDA optimization path
- OCR-based injection verification (roadmap)

---

## C) Milestone 1 Implementation Details

### Changes Required

| File | Change | Reason |
|------|--------|--------|
| `mise.toml` | Remove `python = "3.13"` line | UV owns Python, 3.13 breaks PyO3 |
| `requirements.txt` | Delete file | Empty/misleading, use pyproject.toml |
| `crates/app/Cargo.toml` | Remove `whisper = []` feature | Non-functional stub |
| `crates/coldvox-stt/src/plugins/` | Delete whisper files | Dead code removal |
| `AGENTS.md` | Update feature flags section | Document reality |
| `README.md` | Update Quick Start | Remove whisper refs, add moonshine |
| `docs/plans/critical-action-plan.md` | Check off resolved items | Track progress |

### New Files

- `scripts/validate_telemetry_schema.py` - Validates metric naming conventions
- `.github/workflows/telemetry-schema-check.yml` - CI enforcement

### Test Plan

```bash
# Pre-PR verification
cargo fmt --all -- --check
cargo clippy --all-targets --locked
cargo test --workspace --locked
./scripts/local_ci.sh

# Feature-specific tests
cargo build -p coldvox-app --features moonshine,text-injection  # ✅ Should work
cargo build -p coldvox-app --features whisper                    # ❌ Should fail (removed)
```

---

## Telemetry Naming Convention (Proposed)

Format: `coldvox.{subsystem}.{metric_name}.{unit}`

Examples:
- `coldvox.pipeline.latency_ms` - Pipeline latency in milliseconds
- `coldvox.stt.transcription_success_total` - Counter of successful transcriptions
- `coldvox.vad.detected_speech_bool` - Boolean gauge of speech detection state
- `coldvox.audio.level_db` - Audio level in decibels

Existing metrics to migrate:
- `capture_frames` → `coldvox.pipeline.capture_frames_total`
- `stt_transcription_success` → `coldvox.stt.transcription_success_total`
- `end_to_end_ms` → `coldvox.pipeline.latency_ms`

---

*This document is a living plan. Update as milestones are completed.*
