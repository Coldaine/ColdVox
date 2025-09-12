---
id: COLDVOX-ADR3-001-vosk-model-distribution
type: ADR
level: 3
title: Vosk Model Distribution Strategy
status: accepted
owner: @team-stt
updated: 2025-09-11
parent: COLDVOX-DOM2-004-stt-engine
links:
  satisfies: [COLDVOX-DOM2-004-stt-engine]
  depends_on: []
  supersedes: []
  related_to: [COLDVOX-SPEC5-004-stt-engine-interface]
---

## Context
We need to provide an offline STT path with minimal friction while keeping CI deterministic. Multiple options exist for distributing the Vosk model.

## Decision
Commit the small (≈40–50MB) English model directory `models/vosk-model-small-en-us-0.15/` directly to the repository.

## Status
Accepted

## Consequences
### Positive
- Eliminates network flakiness in CI (faster, deterministic runs)
- Simplifies onboarding (clone → run with `--features vosk`)
- Keeps evaluation & e2e tests reproducible (same acoustic graph & LM)

### Negative
- Repository clone size increases (initial penalty for contributors)
- Future model updates create larger history deltas (git object storage growth)
- Harder to swap languages/variants dynamically without adding more bulk

## Alternatives Considered
1. Download model at build/test time (cache in CI)
2. Require manual developer download (document steps only)
3. Use Git LFS for model binaries

## Related Documents
- `THIRDPARTY.md`
- `crates/coldvox-stt-vosk/src/model.rs`
- `README.md` (root)

---
satisfies: COLDVOX-DOM2-004-stt-engine  
depends_on:  
supersedes:  
related_to: COLDVOX-SPEC5-004-stt-engine-interface