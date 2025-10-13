# ColdVox - Next Steps After PR #152

**Last Updated:** 2025-10-12  
**Context:** Post PR #152 (Text Injection Orchestrator) Merge  
**Priority:** Transition to UI Development Phase

---

## 🎯 Immediate Actions (This Week)

### 1. Merge PR #152 ✅
- **Status:** READY TO MERGE
- **Action:** Approve and merge `injection-orchestrator-lean` into `main`
- **Post-Merge:** Run full test suite in development environment to verify
  ```bash
  cargo check --workspace --all-targets --locked
  cargo clippy --workspace --all-targets --locked -- -D warnings
  cargo test --workspace --locked
  ```

### 2. Create GitHub Issues 📝
Create the following issues to track identified problems:

#### Issue #159: ONNX Runtime Build Dependencies in Restricted Environments
**Title:** [Build] ONNX Runtime download fails in restricted CI/sandbox environments  
**Priority:** High  
**Labels:** `build`, `dependencies`, `ci`

**Description:**
```markdown
## Problem
The Silero VAD feature requires ONNX Runtime, which `ort-sys` attempts to download during build from `cdn.pyke.io`. This fails in restricted environments (CI, Docker, sandboxes) with no internet access.

## Error
```
Failed to GET https://cdn.pyke.io/0/pyke:ort-rs/ms@1.22.0/x86_64-unknown-linux-gnu.tgz: 
io: failed to lookup address information: No address associated with hostname
```

## Impact
- Cannot build project in isolated environments
- Blocks automated testing and CI workflows
- Prevents reproducible builds in air-gapped setups

## Proposed Solutions
1. **Option A**: Make Silero VAD truly optional with feature flag
2. **Option B**: Pre-cache ONNX binaries in repository or CI artifacts
3. **Option C**: Add alternative VAD backend that doesn't require ONNX
4. **Option D**: Use system-installed ONNX runtime if available

## Acceptance Criteria
- [ ] Project builds successfully without internet access
- [ ] CI can build all feature combinations
- [ ] Documentation updated with build requirements
```

### 3. Triage Open Issues 🔍
**Action:** Review all 20 open issues for:
- **Staleness:** Are they still relevant?
- **Priority:** Update P0/P1/P2 labels
- **Duplicates:** Consolidate similar issues
- **Closure:** Close issues that are no longer applicable

**High Priority Issues to Review First:**
- #136: Crazy enormous issue needs to be parceled out (already partially addressed)
- #46: Security - STT model loading validation
- #37: STT error recovery mechanisms
- #40: Platform-specific text injection testing

---

## 🚀 Phase 1: UI Development Foundation (Weeks 1-2)

### Primary Goal
Connect the existing Qt/QML GUI prototype to the real ColdVox audio/STT backend.

### Tasks

#### 1. Backend Service Integration (Issue #60)
**Estimated Effort:** 3-5 days  
**Files to Modify:**
- `crates/coldvox-gui/src/bridge.rs`
- `crates/coldvox-gui/src/main.rs`
- `crates/app/src/runtime.rs`

**Implementation Steps:**
1. Create service registry for GUI to access backend services
2. Implement channel-based communication (tokio channels)
3. Connect audio level monitoring to RMS values from `coldvox-audio`
4. Wire VAD events to GUI state transitions
5. Stream STT transcription results to GUI display
6. Add error propagation from backend to GUI

**Acceptance Criteria:**
- [ ] GUI displays real audio levels
- [ ] VAD state changes reflect in UI
- [ ] Transcriptions appear in real-time
- [ ] Error messages show in GUI
- [ ] No UI freezing during operations

#### 2. GuiBridge Backend Implementation (Issue #58)
**Estimated Effort:** 2-3 days  
**Files to Modify:**
- `crates/coldvox-gui/src/bridge.rs`

**Methods to Implement:**
- `cmd_start()` - Connect to audio capture and VAD/STT pipeline
- `cmd_stop()` - Properly stop audio processing
- `cmd_toggle_pause()` - Pause/resume audio processing
- `cmd_clear()` - Clear transcript and reset state
- `cmd_open_settings()` - Open settings with actual configuration

**Acceptance Criteria:**
- [ ] All GuiBridge commands functional
- [ ] State management works correctly
- [ ] Error handling propagates to UI
- [ ] Audio pipeline controls respond properly

#### 3. Unit Tests for GuiBridge (Issue #62)
**Estimated Effort:** 1-2 days  
**New File:** `crates/coldvox-gui/src/bridge_tests.rs`

**Test Cases:**
- Valid state transitions (Idle→Recording, Recording→Paused, etc.)
- Invalid transitions properly rejected
- Property updates trigger signals
- Method guards prevent invalid operations
- Edge cases handled (rapid state changes, concurrent ops)

**Acceptance Criteria:**
- [ ] Comprehensive test coverage for state machine
- [ ] All transitions validated
- [ ] Edge cases covered
- [ ] Tests run in CI (when Qt available)

#### 4. Configurable Window Dimensions (Issue #59)
**Estimated Effort:** 1 day  
**Files to Modify:**
- `crates/coldvox-gui/qml/AppRoot.qml`
- `crates/coldvox-gui/qml/SettingsWindow.qml`

**Implementation:**
- Add configuration options for window sizes
- Store preferences in Qt Settings
- Add UI controls for size adjustment
- Implement min/max constraints
- Test on different resolutions

**Acceptance Criteria:**
- [ ] Window sizes configurable
- [ ] Settings persist across sessions
- [ ] UI remains usable at all sizes
- [ ] Works on different DPI settings

---

## 🎨 Phase 2: Feature Completeness (Weeks 3-5)

### Goals
Make the GUI feature-complete with all core functionality.

### Tasks

#### 1. Real-time Audio Visualization
- Waveform display
- Audio level meters with peak indicators
- VAD state indicator (visual feedback)
- Frequency spectrum (optional enhancement)

#### 2. Transcript Display & Editing
- Scrollable transcript area
- Syntax highlighting for partial vs final text
- Copy transcript to clipboard
- Export transcript (TXT, JSON, CSV)
- Search within transcript

#### 3. Comprehensive Settings Panel
- Audio device selection
- VAD sensitivity adjustment
- STT model selection (Vosk, Whisper when available)
- Text injection preferences
- Hotkey configuration
- Theme selection

#### 4. System Tray Integration
- Minimize to tray
- Quick actions from tray menu
- Status indicators
- Notifications for important events

#### 5. Keyboard Shortcuts & Hotkeys
- Global hotkey for start/stop
- Local shortcuts for common actions
- Customizable keybindings
- Help overlay showing shortcuts

---

## 🌟 Phase 3: Polish & UX (Weeks 6-7)

### Goals
Refine the user experience and add polish.

### Tasks

#### 1. Theming Support
- Light and dark themes
- Custom color schemes
- Font selection
- Accessibility-focused themes

#### 2. Accessibility Features
- Screen reader support
- Keyboard-only navigation
- High contrast modes
- Adjustable text sizes

#### 3. User Documentation
- In-app help system
- Getting started guide
- Keyboard shortcut reference
- Troubleshooting guide

#### 4. Error Handling & User Feedback
- Clear error messages
- Recovery suggestions
- Status messages
- Progress indicators for long operations

#### 5. Performance Optimization
- Reduce UI latency
- Optimize rendering
- Memory usage optimization
- Battery usage considerations

---

## 🔧 Parallel Workstreams (Ongoing)

### A. Code Quality Improvements
**Based on Issue #136**

#### Week 1-2: Correctness & Reliability (P0)
1. Fix cooldowns to be per-app instead of global
2. Remove hardcoded "unknown_app" strings
3. Add proper mutex poisoning handling
4. Add timeouts on awaited operations
5. Replace blocking `std::process::Command` with async
6. Fix silent failures in app detection
7. Implement cache invalidation for success records

#### Week 3-4: Performance & Maintainability (P1)
1. Remove duplicate functions (_get_method_priority vs compute_method_order)
2. Optimize redact_text with zero-copy patterns
3. Improve sort comparator efficiency
4. Batch metrics operations
5. Add cache cleanup mechanisms
6. Replace magic numbers with constants
7. Remove or implement dead code (chunk_and_paste, pace_type_text)
8. Add app_id caching with TTL

#### Week 5-6: Structure, Testing & Docs (P2)
1. Refactor monolithic inject() method
2. Add targeted tests for cooldowns, cache, etc.
3. Remove #[allow(dead_code)] attributes
4. Remove cfg!(test) checks from production code
5. Add comprehensive documentation

### B. CI/CD Improvements
**Based on Issue #100**

#### 1. Universal Rust Caching
Add `Swatinem/rust-cache@v2` to all workflow jobs for faster builds.

#### 2. Security Scans
- Implement `cargo deny` enforcement
- Add `cargo audit` checks
- Integrate TruffleHog for secret scanning

#### 3. Pre-commit Enforcement in CI
Run pre-commit hooks in CI to ensure consistency.

#### 4. Coverage Reporting (Optional)
Add `cargo-tarpaulin` for code coverage metrics.

### C. Platform Testing
**Based on Issue #40**

#### Test Matrix
Test text injection on:
- **Desktop Environments:** GNOME (Wayland/X11), KDE (Wayland/X11), XFCE, Cinnamon, Sway, Hyprland, i3
- **Applications:** Terminals, text editors, browsers, chat apps, office apps
- **Scenarios:** Basic injection, performance, edge cases, failure scenarios

#### Deliverables
- Completed test matrix with results
- Per-platform setup documentation
- Known issues/limitations list
- Recommended backends per environment

---

## 🛡️ Security & Stability (Weeks 8-10)

### 1. STT Model Loading Security (Issue #46)
**Priority:** HIGH  
**Tasks:**
- Implement model file integrity validation (SHA-256)
- Add signature verification for official models
- Create sandboxed model loading environment
- Implement secure model storage with access controls
- Add security event logging

### 2. STT Error Recovery (Issue #37)
**Priority:** HIGH  
**Tasks:**
- Implement comprehensive error handling in STT processor
- Add fallback STT engines for error recovery
- Create error recovery state machine
- Implement circuit breaker pattern
- Add error logging and monitoring

### 3. Audio Pipeline Fixes (Issue #36)
**Priority:** MEDIUM  
**Tasks:**
- Eliminate memory allocations in audio capture callbacks
- Implement pre-allocated buffer system
- Add callback performance monitoring
- Test under high load conditions

---

## 📊 Performance Optimization (Weeks 11-12)

### 1. Async STT Processing (Issue #47)
**Priority:** MEDIUM  
**Benefits:**
- Non-blocking UI during transcription
- Support concurrent audio streams
- Better resource utilization

### 2. Audio Format Conversion (Issue #45)
**Priority:** MEDIUM  
**Tasks:**
- Standardize on single internal audio format (f32)
- Optimize conversion algorithms
- Reduce CPU overhead from conversions
- Minimize conversions throughout pipeline

### 3. Long Utterance Support (Issue #42)
**Priority:** MEDIUM  
**Tasks:**
- Implement streaming processing for long audio
- Add memory-efficient buffer management
- Create utterance segmentation for long audio
- Support 10+ minute transcriptions efficiently

---

## 🔌 Feature Expansion (Future)

### 1. Additional STT Backends
- **Whisper** (Issue #41): Implement whisper.cpp or Candle-based backend
- **Cloud STT**: Azure, Google, AWS options
- **Custom Models**: Support for fine-tuned models

### 2. Plugin System (Issue #34)
- Integrate existing plugin architecture
- Allow runtime STT engine switching
- Plugin discovery and management
- Configuration per plugin

### 3. Advanced Features
- Speaker diarization
- Punctuation restoration
- Multi-language support
- Real-time translation
- Transcript formatting options

---

## 📈 Success Metrics

### Development Velocity
- Complete Phase 1 (UI Foundation) in 2 weeks
- Achieve 80% code coverage on new GUI code
- Zero critical bugs in production

### Quality Metrics
- All PRs pass automated checks
- Code review within 24 hours
- Issues triaged within 48 hours

### User Experience
- UI response time < 100ms
- Transcription latency < 500ms
- Zero UI freezes during operation
- Battery impact < 5% on laptops

---

## 🗂️ Issue Management Strategy

### Weekly Cadence
- **Monday:** Triage new issues
- **Wednesday:** Sprint planning
- **Friday:** Sprint review & issue grooming

### Prioritization Framework
- **P0 (Critical):** Blocks core functionality, security issues
- **P1 (High):** Important features, major bugs
- **P2 (Medium):** Enhancements, minor bugs
- **P3 (Low):** Nice-to-haves, future features

### Staleness Policy
- Issues untouched for 90 days marked as "stale"
- Stale issues reviewed monthly
- Close or reprioritize after review

---

## 🎓 Documentation Needs

### User Documentation
- Getting started guide
- Installation instructions per platform
- Configuration reference
- Troubleshooting guide
- FAQ

### Developer Documentation
- Architecture overview (update with GUI)
- Contributing guidelines
- Code style guide
- Testing strategy
- Release process

### API Documentation
- GuiBridge interface
- Plugin API
- Configuration schema
- Event system

---

## 🚦 Risk Management

### Technical Risks
1. **ONNX Dependencies:** May block builds in CI
   - **Mitigation:** Pre-cache binaries, make Silero optional

2. **Platform Compatibility:** Text injection varies by DE/WM
   - **Mitigation:** Comprehensive testing matrix, fallback mechanisms

3. **Qt6 Availability:** Not available on all distros
   - **Mitigation:** Clear build requirements, detection logic

### Schedule Risks
1. **Scope Creep:** Feature requests may delay UI completion
   - **Mitigation:** Strict MVP definition, defer enhancements

2. **Testing Time:** Platform testing is time-intensive
   - **Mitigation:** Automate where possible, prioritize common platforms

### Resource Risks
1. **Solo Development:** Single developer workload
   - **Mitigation:** Clear priorities, incremental delivery, automate repetitive tasks

---

## 💡 Success Factors

### Critical Success Factors
1. ✅ Complete GUI integration (Phase 1) within 2 weeks
2. ✅ Maintain test coverage above 75%
3. ✅ Zero regression bugs in existing functionality
4. ✅ Clear, up-to-date documentation
5. ✅ Responsive issue triage and management

### Key Decisions Needed
1. Which STT backend to prioritize (Vosk vs Whisper)
2. Desktop environment testing scope (all vs subset)
3. Release versioning strategy (semantic versioning)
4. Community contribution guidelines (if open source)

---

## 📅 Timeline Summary

| Week | Focus | Milestones |
|------|-------|------------|
| 1 | PR #152 merge, issue triage, GUI backend connection | GUI shows real data |
| 2 | GuiBridge implementation, unit tests | All GUI controls functional |
| 3-4 | Audio visualization, transcript editing | Feature-complete UI |
| 5 | Settings panel, hotkeys | Configuration system |
| 6-7 | Polish, theming, documentation | Release candidate |
| 8-10 | Security hardening, testing | Production ready |
| 11-12 | Performance optimization | Optimized release |

---

## 🎯 Definition of Done

### For PR #152
- [x] Code reviewed and approved
- [x] Tests passing
- [x] Documentation updated
- [ ] Merged to main
- [ ] Post-merge verification complete

### For Phase 1 (GUI Foundation)
- [ ] GUI connects to real audio/STT backend
- [ ] All GuiBridge methods implemented
- [ ] Unit tests for state transitions
- [ ] Window dimensions configurable
- [ ] Zero UI freezes during operation
- [ ] Documentation updated

### For Full Release
- [ ] All phases complete
- [ ] Comprehensive testing done
- [ ] Security audit passed
- [ ] Performance benchmarks met
- [ ] User documentation complete
- [ ] Release notes prepared

---

## 📞 Next Actions

### For Maintainer
1. **Merge PR #152** (immediate)
2. **Create Issue #159** (ONNX build dependencies)
3. **Triage open issues** (schedule session)
4. **Start Phase 1** (GUI backend connection)

### For Contributors
1. Pick issues labeled "good first issue"
2. Review contributing guidelines
3. Set up development environment
4. Join project communication channels

### For Users
1. Wait for next release with GUI
2. Report any bugs in current CLI version
3. Provide feedback on desired features

---

**This document is a living roadmap and should be updated as priorities shift and new information becomes available.**

**Last Review:** 2025-10-12  
**Next Review:** After PR #152 merge
