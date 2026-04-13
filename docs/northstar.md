---
doc_type: architecture
subsystem: general
status: draft
freshness: stale
preservation: preserve
last_reviewed: 2026-02-09
last_reviewer: Patrick MacLyman
owners: Patrick MacLyman
review_due: 2026-05-10
version: 1.0.0
---

# ColdVox North Star

This document is the product and technical anchor for documentation decisions.
When other docs conflict with this, they should be updated, archived, or removed.

## Core Goals

- Maximize performance on high-end NVIDIA GPUs (specifically RTX 5090 class) with CUDA-first STT execution.
- Provide a reliable fallback STT path for non-CUDA machines, with Moonshine as the baseline fallback backend.
- Deliver a transparent GUI overlay that shows recognized words while speaking.
- Show words live in both push-to-talk (PTT) mode and speech-detection mode.
- Support streaming partial transcription so users do not wait for end-of-utterance text.

## Execution Priority (Current)

- Primary delivery goal: complete reliable end-to-end flow from microphone input to correct text injection.
- Supported STT now: Moonshine.
- Planned later: Parakeet.
- No "no-STT" product mode for normal operation.
- Overlay visibility target: visible while actively capturing.
- Do not overfit to synthetic numeric tolerances at this stage; prioritize practical "it works reliably" behavior.
- CUDA focus means selecting and integrating the best-performing CUDA-capable model path, not hardware-specific micro-optimization.
- Injection failure behavior target: retry once, then notify in overlay.
- Injection confirmation beyond current methods (for example OCR-based verification) is long-term roadmap, not near-term gate.

## Documentation Policy Alignment

- Docs can be aspirational, research-oriented, or implementation-tracking, but must state which.
- Docs claiming shipped behavior must include verifiable references to code/config/tests.
- Outdated but valuable research should be archived, not discarded.
- Active docs should map clearly to one or more North Star goals.
