# 2025-09-18 — Vosk Model Autodetection & Auto-Extraction Plan

Owner: @ColdVox
Status: Proposed (to implement on branch `fix-audio-device-priority-alsa-first`)

## Goals
- Zero-config Vosk usage: no env/flags required for typical cases.
- Deterministic, aggressive autodetection of extracted models.
- Default-on ZIP auto-extraction when no extracted model is present.
- Single, clear log message describing resolution source or actionable failure.

## Scope
- Affects `crates/coldvox-stt-vosk` primarily; minor wiring in `crates/app` (env override consumption) and logging.
- Does not change cross-plugin `TranscriptionConfig` surface.

## Behavior Summary
1. Discovery priority remains: `VOSK_MODEL_PATH` (if set) > explicit config > autodetected candidates.
2. Autodetection: scan `models/` plus up to 3 ancestors for any `vosk-model-*` dir; pick deterministically.
3. Auto-extraction: if no extracted dir found, search for `vosk-model-*.zip` (same paths), pick best, extract into `models/` using temp+rename and a simple lock. Default ON.
4. Logging: one resolution log in `VoskPlugin::initialize`; on failure, a single actionable error with checked paths and remediation.
5. Env override (no CLI): `COLDVOX_STT_AUTO_EXTRACT=true|false` read at plugin/app boundary, default TRUE.

## Deterministic Selection Policy
- Prefer language `en-us` > other languages
- Prefer variant containing `small` (if both present, still pick highest version)
- Prefer highest semantic version parsed from name; fall back to lexical order when unknown
- Tie-breaker: stable lexical ordering

## Implementation Plan
- Broaden model autodetection
  - Update `crates/coldvox-stt-vosk/src/model.rs::locate_model` to collect candidates from `models/` and up to 3 ancestors; reuse a pure `pick_best_candidate` for deterministic selection.
- Selection utility
  - Add `pick_best_candidate(candidates)` in `model.rs`: parse name into components (lang, size, semver); returns the best path and parsed metadata.
- ZIP auto-extraction (default ON)
  - Add `ensure_model_available(auto_extract: bool)` to find best `vosk-model-*.zip` and extract into `models/` when needed. Use a temp directory + atomic rename and a simple lock file (`.extract.lock`) under `models/`. Sanitize ZIP paths to avoid traversal. Extend `ModelError::ExtractionFailed` with remediation tips.
  - Add `ModelSource::Extracted` to logging/info structs.
- Plugin wiring
  - In `crates/coldvox-stt-vosk/src/plugin.rs::initialize`, call `ensure_model_available(true)`, then `locate_model` again to finalize `self.model_path`. Keep `check_requirements` side-effect free.
- Logging improvements
  - Centralize a single `log_model_resolution` call in `initialize` including source and extraction duration if applicable. On error, include searched paths and next steps.
- Env override
  - Read `COLDVOX_STT_AUTO_EXTRACT` at the plugin boundary; default to TRUE. No CLI flag. Do not pollute cross-plugin `TranscriptionConfig`.
- Tests
  - Unit tests for selection and scan depth behavior; feature-gated `zip-tests` for extraction idempotency, lock handling, and path sanitization. Avoid loading libvosk.
- Docs/CI
  - Update `crates/coldvox-stt-vosk/README.md` and `docs/self-hosted-runner-complete-setup.md` to reflect the default behavior and runner caching guidance. Add CHANGELOG entry.

## Failure Modes & Remediation
- No extracted dirs and no ZIPs found: fail with a single error listing checked roots and suggest downloading a model, setting `VOSK_MODEL_PATH`, or placing a ZIP under `models/`.
- Extraction fails (permissions/space): error indicates target path and suggests running with `COLDVOX_STT_AUTO_EXTRACT=false` or pre-extracting.
- Multiple candidates ambiguous but deterministic pick differs from user’s intent: set `VOSK_MODEL_PATH` explicitly or remove undesired copies.

## Security & Robustness
- ZIP traversal protection: reject entries with `..` components or absolute paths.
- Atomic publish: extract to `models/.tmp-<uuid>` then rename to final dir.
- Concurrency: simple lock file to serialize extraction; tolerant if lock is stale.
- Depth cap: scan limited to 3 ancestors for both dirs and ZIPs to avoid slow walks.

## Acceptance Criteria
- With only vendored ZIP(s) present at repo root, first run logs "Extracted" source and succeeds.
- With extracted `models/vosk-model-*/` present, no extraction occurs; logs appropriate source.
- Deterministic selection verified by unit tests with varied candidate sets.
- `COLDVOX_STT_AUTO_EXTRACT=false` prevents extraction and produces a clear error when needed.
