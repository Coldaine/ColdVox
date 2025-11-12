# Issue Clearance Sprint - Status Tracker

**Branch**: `feature/issue-clearance-sprint`
**Started**: 2025-11-10
**Master Plan**: [issue-clearance-master-plan.md](./issue-clearance-master-plan.md)

## Sprint Objectives

Execute systematic clearance of 26 open GitHub issues following the master plan.

## Current Phase: Phase 1 - Foundation (Weeks 1-4)

**Goal**: Complete critical path + quick wins

### Active Work

- [ ] **Issue #221**: Implement WhisperEngine API (Candle Migration) - IN PROGRESS
  - Status: Not started
  - Next: Create `crates/coldvox-stt/src/candle/audio.rs`

- [ ] **Issue #171**: Complete AT-SPI Focus Backend - READY
  - Status: Not started
  - Next: Implement `query_focus()` in `focus.rs`

### Recently Completed

- [x] **Issue #34**: STT Plugin System - CLOSED 2025-11-10
- [x] **Issue #37**: STT Error Recovery - CLOSED 2025-11-10
- [x] **Issue #159**: AT-SPI D-Bus Path Format - CLOSED 2025-11-10
- [x] **Issue #160**: Clipboard Fallback Wiring - CLOSED 2025-11-10
- [x] **Issue #172**: X11 Clipboard Test - CLOSED 2025-11-10
- [x] **Issue #58-63, #227**: GUI Issues (consolidated into #226) - CLOSED 2025-11-10
- [x] **Issue #36**: Audio Callback Allocations - FIXED (PR #231)
- [x] **Issue #38**: AT-SPI Placeholders - FIXED (PR #231)

## Progress Summary

- **Total Issues at Start**: 38
- **Issues Closed**: 11
- **Issues Fixed (PR pending)**: 2
- **Remaining**: 26

## Week 1 Plan

### Monday-Tuesday (Days 1-2)
- [x] Complete issue triage and master plan
- [x] Fix #36 and #38 (quick wins)
- [ ] Start #221: Create candle module structure
- [ ] Complete #171: AT-SPI focus backend

### Wednesday-Thursday (Days 3-4)
- [ ] #221: Implement audio preprocessing (audio.rs)
- [ ] #221: Start model loader (loader.rs)

### Friday (Day 5)
- [ ] #221: Complete model loader
- [ ] #221: Test model download and loading
- [ ] Week 1 review and planning for Week 2

## Phase Milestones

### Milestone 1: Foundation Complete (Week 4)
- [ ] #221 complete - Candle Whisper working
- [ ] #171 complete - AT-SPI focus backend working
- [ ] 2-3 quick wins closed

### Milestone 2: Docs & Benchmarks (Week 6)
- [ ] README updated (#224)
- [ ] Benchmarks comparing old vs new STT (#222)
- [ ] Word timestamp research complete (#223)

### Milestone 3: Quality (Week 10)
- [ ] Code coverage >80% (#162, #211)
- [ ] Test matrix for 3+ platforms (#40)
- [ ] CI improvements deployed (#212, #213)

### Milestone 4: Complete (Week 12)
- [ ] <10 open issues remaining
- [ ] All P1/P2 issues closed
- [ ] Documentation complete

## Notes & Decisions

### 2025-11-10
- Created master plan with atomic steps for all 26 issues
- Identified #221 (Candle) as critical path blocking 3 other issues
- Fixed #36 and #38 as quick wins (PR #231)
- Closed 11 duplicate/completed issues during triage
- Decision: Start with #221 audio.rs module, then quick win #171

## Blockers

None currently.

## Next Actions

1. **Immediate**: Complete #171 (AT-SPI focus backend) - 30-45 min
2. **Today**: Start #221 - Create candle module directory structure
3. **This Week**: Implement #221 audio.rs and loader.rs

---

**Last Updated**: 2025-11-10
**Updated By**: Claude Code
