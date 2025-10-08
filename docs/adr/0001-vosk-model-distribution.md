# ADR 0001: Vosk Model Distribution Strategy

Date: 2025-09-09
Status: Accepted
Decision Context: Provide an offline STT path with minimal friction while keeping CI deterministic.

## Options Considered
1. Download model at build/test time (cache in CI).
2. Require manual developer download (document steps only).
3. Commit the small English model into the repository.
4. Use Git LFS for model binaries.

## Decision
Option 3: Commit the small (≈40–50MB) English model directory `models/vosk-model-small-en-us-0.15/` directly.

## Rationale
- Eliminates network flakiness in CI (faster, deterministic runs).
- Simplifies onboarding (clone → run with `--features vosk`).
- Keeps evaluation & e2e tests reproducible (same acoustic graph & LM).
- Size impact acceptable for now; single large directory, infrequent updates.

## Trade-offs
- Repository clone size increases (initial penalty for contributors).
- Future model updates create larger history deltas (git object storage growth).
- Harder to swap languages/variants dynamically without adding more bulk.

## Mitigations
- Provide `SHA256SUMS` for integrity checking.
- Document provenance & license in `THIRDPARTY.md`.
- Warn of impending deprecation if model count grows (trigger revisit threshold: >150MB cumulative models).
- Potential future migration path: move to Git LFS or per-language on-demand download.

## Revisit Conditions
- Added second model variant or language.
- Clone complaints or CI bandwidth constraints surface.
- Need for reproducible benchmarks on alternative models.

## Implementation Notes
- Model resolution logic (`model.rs`): env > config > `models/` dir > legacy root fallback (deprecated).
- CI job validates directory structure plus checksum (to be extended with full hash check if needed).

## Related Documents
- `crates/coldvox-stt-vosk/src/model.rs`
- `README.md` (root)
- Model license: See `models/vosk-model-small-en-us-0.15/LICENSE`
