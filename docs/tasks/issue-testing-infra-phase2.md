# Issue: Testing Infrastructure Phase 2

**Status:** New (Consolidated)
**Original Issues:** #208, #209, #210, #211, #212
**Epic:** Testing
**Tags:** `roadmap`, `testing`, `infrastructure`, `consolidation`

## Summary

This issue consolidates several recent tasks related to documentation and testing infrastructure into a single "Phase 2" meta-issue. The goal is to build upon the reliability improvements from PR #152 and establish a comprehensive, robust testing framework for the entire application.

## Key Initiatives

### 1. Golden Master Test Stabilization
- [ ] **Task:** Stabilize VAD (Voice Activity Detection) golden master tests, applying timeout patterns and test isolation techniques from the clipboard fixes.
- [ ] **Related:** Issue #221
- [ ] **Acceptance:** VAD golden master tests run reliably in CI.

### 2. Configuration & Model Path Reliability
- [ ] **Task:** Fix Vosk and Whisper model path discovery and configuration issues to ensure tests can find model files consistently.
- [ ] **Related:** Issue #222
- [ ] **Acceptance:** Model-dependent tests run without path-related failures.

### 3. VM-Based Compositor Testing Matrix
- [ ] **Task:** Set up a VM-based testing matrix to validate text injection and GUI interaction across different desktop environments (GNOME, KDE, Wayland, X11).
- [ ] **Related:** Issue #173
- [ ] **Acceptance:** Integration tests are automatically run against multiple desktop environments.

### 4. Benchmarking Harness
- [ ] **Task:** Implement a performance benchmarking harness in CI/CD to track performance regressions for VAD, STT, and text injection.
- [ ] **Related:** Issues #44, #45, #47
- [ ] **Acceptance:** Performance metrics are collected on each PR and regressions are flagged.
