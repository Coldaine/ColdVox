---
title: Comprehensive Main Branch Review Plan - v2.0 Focused on Actual Implementation
doc_type: plan
status: proposed
created: 2025-12-09
version: 2.0.0
---

# ColdVox Main Branch Comprehensive Review Plan (v2.0)

## Current State (What Actually Exists)

### Implemented STT Plugins
- ✅ Parakeet (NVIDIA GPU, production)
- ✅ Moonshine (CPU via PyO3, recently added PR #259)
- ✅ Mock (testing)
- ✅ NoOp (testing)

### Stub Plugins to Remove
- ❌ coqui, leopard, silero_stt, whisper_cpp, whisper_plugin

### Obsolete to Remove

## Review Agents (8 Total)

### Agent 1: Concurrency Safety Auditor
- Lock hierarchy mapping
- Async/await race conditions
- Background task safety
- Text injection async issues from ti-async-safety-analysis.md

### Agent 2: Audio Pipeline Specialist  
- Ring buffer safety
- Real-time constraints (no allocations in callback)
- Resampling correctness
- Latency validation (150-500ms)

### Agent 3: STT Plugin Reviewer
- Parakeet production readiness
- Moonshine integration review (post-PR #259)
- Plugin error handling

### Agent 4: Plugin Manager Analyst
- Lifecycle state machine
- GC race conditions
- Failover with real plugins
- Config persistence

### Agent 5: Memory Safety Analyst
- Unsafe code audit
- PyO3 safety (Moonshine)
- Model memory cleanup
- Resource leaks

### Agent 6: Test Quality Assessor
- Coverage for Parakeet/Moonshine
- Concurrency tests
- Edge cases

### Agent 7: Cleanup Agent (PRIORITY 1 - Run First)
**Tasks:**
1. Delete stub plugins (coqui, leopard, silero_stt, whisper_cpp, whisper_plugin)
3. Clean feature flags in Cargo.toml
4. Remove commented-out code

### Agent 8: Documentation Reviewer
- Verify docs match implementation
- Document Parakeet/Moonshine
- Remove speculative content

## Execution Order

1. **Week 1:** Agent 7 (cleanup) - unblocks others
2. **Week 1-2:** Agents 1-6 (parallel reviews)
3. **Week 3:** Agent 8 (docs) + synthesis + final report

## Success Criteria

- [ ] All stubs removed
- [ ] Parakeet/Moonshine production-ready or issues documented
- [ ] >80% test coverage for implemented features
- [ ] Docs match implementation
