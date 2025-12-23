---
doc_type: plan
subsystem: general
version: 2.0.0
status: draft
owners: Coldaine
last_reviewed: 2025-12-09
---

# ColdVox Main Branch Review Plan v2.0

## Current State

**Implemented:**
- Parakeet (GPU), Moonshine (CPU), Mock, NoOp

**Remove:**
- Stub plugins: coqui, leopard, silero_stt, whisper_cpp, whisper_plugin  

## 8 Review Agents

1. Concurrency Safety - async hazards, locks, race conditions
2. Audio Pipeline - real-time constraints, buffer safety
3. STT Plugins - Parakeet & Moonshine review
4. Plugin Manager - lifecycle, GC, failover
5. Memory Safety - leaks, unsafe code, PyO3
8. Documentation - match implementation

## Execution

Week 1: Agent 7 cleanup
Week 1-2: Agents 1-6 parallel
Week 3: Agent 8 + synthesis

## Goals

- Remove all stubs
- 80%+ coverage on real features
- Docs match code
