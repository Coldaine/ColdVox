---
doc_type: reference
subsystem: vad
status: draft
freshness: stale
preservation: preserve
domain_code: vad
last_reviewed: 2025-10-19
owners: Documentation Working Group
version: 1.0.0
---

# Modifications from Upstream

## Why Vendored
- Need custom ONNX runtime configuration for ColdVox
- Require specific audio frame handling and integration ergonomics

## Changes Made

### 2024-08-24: Initial vendor
- Vendored from repository: https://github.com/Coldaine/ColdVox-voice_activity_detector
- Commit: <fill-commit-sha>
- No local modifications yet

### 2024-09-XX: Custom frame size support
- Modified: src/lib.rs L45-67
- Reason: ColdVox needs 320-sample frames (20ms @ 16kHz)
- Upstream PR potential: Yes - generally useful

## Notes
- When modifying code, annotate hunks with `COLDVOX_MOD: Start/End` comments.
- Keep this log up to date to simplify upstream syncs and audits.
